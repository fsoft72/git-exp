use crate::git::cli::GitCommandLog;
use crate::git::diff::{DiffTarget, FileDiff, fetch_file_diff, target_from_change};
use crate::git::graph::{CommitDetails, CommitNode, fetch_commit_details, fetch_commit_graph};
use crate::git::ops::{self, CommitVariant};
use crate::git::status::{FileChange, FileStatusKind, RepoStatus, fetch_repo_status};
use crate::settings::UserSettings;
use crate::watcher::RepoWatcher;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Instant;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

#[derive(Debug, Clone)]
pub enum CenterTab {
    Diff(FileDiff),
    CommitView(CommitDetails),
    FileEditor {
        path: String,
        content: String,
        dirty: bool,
    },
}

impl CenterTab {
    pub fn id_key(&self) -> String {
        match self {
            Self::Diff(d) => match &d.target {
                DiffTarget::WorkingTree { path, .. } => format!("diff:wt:{}", path),
                DiffTarget::Staged { path, .. } => format!("diff:st:{}", path),
                DiffTarget::Commit { commit_sha, path } => {
                    format!("diff:cm:{}:{}", commit_sha, path)
                }
            },
            Self::CommitView(c) => format!("commit:{}", c.sha),
            Self::FileEditor { path, .. } => format!("edit:{}", path),
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::Diff(d) => d.target.tab_title(),
            Self::CommitView(c) => format!("Commit: {}", c.short_sha),
            Self::FileEditor { path, dirty, .. } => {
                let name = Path::new(path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path);
                if *dirty {
                    format!("* {}", name)
                } else {
                    name.to_string()
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickPickKind {
    CommandPalette,
    Checkout,
    CreateBranch { from_ref: Option<String> },
    CreateBranchFrom { preset_name: Option<String> },
    DeleteBranch { force: bool },
    RenameBranch,
    MergeBranch,
    RebaseBranch,
    StashPush { include_untracked: bool },
    StashPop,
    StashApply,
    StashDrop,
    CreateTag,
    DeleteTag,
    ConfirmDiscardAll,
    ConfirmDiscardFile { path: String, kind: FileStatusKind },
}

#[derive(Debug, Clone)]
pub struct QuickPickState {
    pub kind: QuickPickKind,
    pub query: String,
    pub selected_index: usize,
    pub just_opened: bool,
}

impl QuickPickState {
    pub fn new(kind: QuickPickKind) -> Self {
        Self {
            kind,
            query: String::new(),
            selected_index: 0,
            just_opened: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSidebarPanel {
    SourceControl,
    GraphFull,
    Repositories,
    Settings,
}

pub enum WorkerEvent {
    RefreshCompleted {
        status: RepoStatus,
        graph: Vec<CommitNode>,
        logs: Vec<GitCommandLog>,
    },
    OperationCompleted {
        message: String,
        logs: Vec<GitCommandLog>,
        refresh_needed: bool,
    },
    OperationFailed {
        error: String,
        logs: Vec<GitCommandLog>,
    },
}

pub struct SyntaxHighlighter {
    pub ps: SyntaxSet,
    pub ts: ThemeSet,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }
}

impl SyntaxHighlighter {
    pub fn highlight_line(&self, file_path: &str, line: &str) -> Vec<(egui::Color32, String)> {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("txt");
        let syntax = self
            .ps
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let theme = &self.ts.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);
        match h.highlight_line(line, &self.ps) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text): (SyntectStyle, &str)| {
                    let c = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    (c, text.to_string())
                })
                .collect(),
            Err(_) => vec![(crate::theme::VsCodeTheme::TEXT_PRIMARY, line.to_string())],
        }
    }
}

pub struct AppState {
    pub repo_path: Option<PathBuf>,
    pub settings: UserSettings,
    pub status: RepoStatus,
    pub graph_commits: Vec<CommitNode>,
    pub git_logs: Vec<GitCommandLog>,

    // Sidebar state
    pub active_sidebar: ActiveSidebarPanel,
    pub commit_message: String,
    pub commit_amend: bool,
    pub commit_action_variant: CommitVariant,
    pub tree_collapsed_folders: BTreeSet<String>,
    pub selected_change_path: Option<(String, crate::git::status::ChangeGroup)>,
    pub selected_commit_sha: Option<String>,

    // Center tabs & Diff state
    pub open_tabs: Vec<CenterTab>,
    pub active_tab_idx: usize,
    pub selected_hunk_lines: BTreeMap<usize, BTreeSet<usize>>,
    pub current_hunk_cursor: usize,
    pub scroll_to_hunk: Option<usize>,

    // Quick Pick Modal
    pub quick_pick: Option<QuickPickState>,

    // Async worker & Watcher
    pub is_busy: bool,
    pub is_refreshing_status: bool,
    pub busy_label: String,
    pub last_refresh_time: Instant,
    pub last_error: Option<String>,
    pub toast_notification: Option<(String, Instant)>,
    tx: Sender<WorkerEvent>,
    rx: Receiver<WorkerEvent>,
    pub egui_ctx: Option<egui::Context>,
    pub watcher: Option<RepoWatcher>,
    pub highlighter: SyntaxHighlighter,
}

impl AppState {
    pub fn new(initial_repo: Option<PathBuf>) -> Self {
        let (tx, rx) = channel();
        let mut settings = UserSettings::load();

        let repo_path = initial_repo.or_else(|| {
            let cwd = std::env::current_dir().ok()?;
            if cwd.join(".git").exists() {
                Some(cwd)
            } else {
                settings
                    .recent_repos
                    .first()
                    .map(PathBuf::from)
                    .filter(|p| p.join(".git").exists())
            }
        });

        if let Some(ref p) = repo_path {
            settings.push_recent_repo(&p.to_string_lossy());
        }

        let mut app = Self {
            repo_path,
            settings,
            status: RepoStatus::default(),
            graph_commits: Vec::new(),
            git_logs: Vec::new(),
            active_sidebar: ActiveSidebarPanel::SourceControl,
            commit_message: String::new(),
            commit_amend: false,
            commit_action_variant: CommitVariant::Normal,
            tree_collapsed_folders: BTreeSet::new(),
            selected_change_path: None,
            selected_commit_sha: None,
            open_tabs: Vec::new(),
            active_tab_idx: 0,
            selected_hunk_lines: BTreeMap::new(),
            current_hunk_cursor: 0,
            scroll_to_hunk: None,
            quick_pick: None,
            is_busy: false,
            is_refreshing_status: false,
            busy_label: String::new(),
            last_refresh_time: Instant::now(),
            last_error: None,
            toast_notification: None,
            tx,
            rx,
            egui_ctx: None,
            watcher: None,
            highlighter: SyntaxHighlighter::default(),
        };

        if app.repo_path.is_some() {
            app.trigger_refresh();
        }

        app
    }

    pub fn open_repository(&mut self, path: PathBuf, ctx: &egui::Context) {
        let mut candidate = path.clone();
        // Risali se è una sottocartella di un repo git
        while !candidate.join(".git").exists() {
            if let Some(parent) = candidate.parent() {
                candidate = parent.to_path_buf();
            } else {
                break;
            }
        }

        if !candidate.join(".git").exists() {
            self.last_error = Some(format!(
                "La cartella '{}' non contiene un repository Git (.git non trovato).",
                path.display()
            ));
            return;
        }

        self.settings.push_recent_repo(&candidate.to_string_lossy());
        self.repo_path = Some(candidate.clone());
        self.open_tabs.clear();
        self.active_tab_idx = 0;
        self.selected_hunk_lines.clear();
        self.last_error = None;
        self.watcher = RepoWatcher::new(&candidate, ctx.clone());
        self.trigger_refresh();
    }

    pub fn init_repository_at(&mut self, path: PathBuf, ctx: &egui::Context) {
        match crate::git::cli::run_git(&path, &["init"], None, true) {
            Ok(out) if out.is_success() => {
                self.git_logs.push(out.log);
                self.open_repository(path, ctx);
            }
            Ok(out) => {
                self.git_logs.push(out.log);
                self.last_error = Some(format!("Errore git init: {}", out.stderr));
            }
            Err(e) => {
                self.last_error = Some(e);
            }
        }
    }

    pub fn poll_background(&mut self, ctx: &egui::Context) {
        if self.egui_ctx.is_none() {
            self.egui_ctx = Some(ctx.clone());
        }
        // Assicura che il watcher sia attivo
        if self.watcher.is_none() {
            if let Some(ref repo) = self.repo_path {
                self.watcher = RepoWatcher::new(repo, ctx.clone());
            }
        }

        // Rispetta rigorosamente l'intervallo `refresh_interval_secs` configurato nei Settings
        if self.repo_path.is_some() && self.settings.refresh_interval_secs > 0 {
            let interval = std::time::Duration::from_secs(self.settings.refresh_interval_secs);
            let elapsed = self.last_refresh_time.elapsed();

            let watcher_dirty = self
                .watcher
                .as_mut()
                .map(|w| w.poll_should_refresh(interval))
                .unwrap_or(false);

            if (elapsed >= interval || watcher_dirty)
                && !self.is_busy
                && !self.is_refreshing_status
            {
                self.trigger_refresh();
            } else {
                let remaining = interval
                    .saturating_sub(elapsed)
                    .max(std::time::Duration::from_millis(250));
                ctx.request_repaint_after(remaining);
            }
        } else if let Some(ref mut w) = self.watcher {
            // Se refresh_interval_secs == 0 (auto-refresh disabilitato), svuota la coda del watcher
            w.clear_pending();
        }

        let mut need_refresh = false;
        let mut active_diff_reload: Option<DiffTarget> = None;

        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                WorkerEvent::RefreshCompleted {
                    status,
                    graph,
                    logs,
                } => {
                    self.is_refreshing_status = false;
                    self.last_refresh_time = Instant::now();
                    if let Some(ref mut w) = self.watcher {
                        // Scarta eventuali eventi generati da git stesso durante il refresh
                        w.clear_pending();
                    }
                    if self.busy_label == "Refreshing Git status..." {
                        self.is_busy = false;
                        self.busy_label.clear();
                    }
                    self.status = status;
                    self.graph_commits = graph;
                    self.git_logs.extend(logs);
                    if self.git_logs.len() > 500 {
                        let drain_cnt = self.git_logs.len() - 500;
                        self.git_logs.drain(0..drain_cnt);
                    }

                    // Se il tab attivo è un Diff su WorkingTree o Staged, ricaricalo per riflettere lo stato aggiornato
                    if let Some(CenterTab::Diff(d)) = self.open_tabs.get(self.active_tab_idx) {
                        if matches!(
                            d.target,
                            DiffTarget::WorkingTree { .. } | DiffTarget::Staged { .. }
                        ) {
                            active_diff_reload = Some(d.target.clone());
                        }
                    }
                }
                WorkerEvent::OperationCompleted {
                    message,
                    logs,
                    refresh_needed,
                } => {
                    self.is_busy = false;
                    self.busy_label.clear();
                    self.git_logs.extend(logs);
                    if !message.is_empty() {
                        self.toast_notification = Some((message, Instant::now()));
                    }
                    if refresh_needed {
                        need_refresh = true;
                    }
                }
                WorkerEvent::OperationFailed { error, logs } => {
                    self.is_busy = false;
                    self.is_refreshing_status = false;
                    self.busy_label.clear();
                    self.git_logs.extend(logs);
                    self.last_error = Some(error);
                }
            }
        }

        if let Some(target) = active_diff_reload {
            self.reload_diff_tab(&target);
        }

        if need_refresh {
            self.trigger_refresh();
        }
    }

    pub fn trigger_refresh(&mut self) {
        let Some(repo) = self.repo_path.clone() else {
            return;
        };
        let tx = self.tx.clone();
        let ctx_opt = self.egui_ctx.clone();
        self.is_refreshing_status = true;
        self.last_refresh_time = Instant::now();
        if self.settings.show_refresh_status_message {
            self.is_busy = true;
            self.busy_label = "Refreshing Git status...".to_string();
        }

        thread::spawn(move || {
            match fetch_repo_status(&repo) {
                Ok((status, mut logs)) => {
                    let upstream = status.upstream.clone();
                    let (graph, graph_logs) =
                        fetch_commit_graph(&repo, upstream.as_deref(), 200).unwrap_or_default();
                    logs.extend(graph_logs);
                    let _ = tx.send(WorkerEvent::RefreshCompleted {
                        status,
                        graph,
                        logs,
                    });
                }
                Err(e) => {
                    let _ = tx.send(WorkerEvent::OperationFailed {
                        error: e,
                        logs: Vec::new(),
                    });
                }
            }
            if let Some(ref ctx) = ctx_opt {
                ctx.request_repaint();
            }
        });
    }

    pub fn spawn_git_task<F>(&mut self, label: &str, f: F)
    where
        F: FnOnce(&Path) -> Result<(String, Vec<GitCommandLog>), (String, Vec<GitCommandLog>)>
            + Send
            + 'static,
    {
        let Some(repo) = self.repo_path.clone() else {
            return;
        };
        self.is_busy = true;
        self.busy_label = label.to_string();
        self.last_error = None;
        let tx = self.tx.clone();
        let ctx_opt = self.egui_ctx.clone();
        if let Some(ref ctx) = ctx_opt {
            ctx.request_repaint();
        }

        thread::spawn(move || {
            match f(&repo) {
                Ok((message, logs)) => {
                    let _ = tx.send(WorkerEvent::OperationCompleted {
                        message,
                        logs,
                        refresh_needed: true,
                    });
                }
                Err((error, logs)) => {
                    let _ = tx.send(WorkerEvent::OperationFailed { error, logs });
                }
            }
            if let Some(ref ctx) = ctx_opt {
                ctx.request_repaint();
            }
        });
    }

    // --- Apertura Tab centrali ---

    pub fn open_change_diff(&mut self, change: &FileChange) {
        let target = target_from_change(change);
        self.selected_change_path = Some((change.path.clone(), change.group));
        self.open_diff_target(target);
    }

    pub fn open_diff_target(&mut self, target: DiffTarget) {
        let Some(ref repo) = self.repo_path else {
            return;
        };
        match fetch_file_diff(repo, &target) {
            Ok((diff, log_opt)) => {
                if let Some(l) = log_opt {
                    self.git_logs.push(l);
                }
                self.selected_hunk_lines.clear();
                self.current_hunk_cursor = 0;
                let tab = CenterTab::Diff(diff);
                self.upsert_and_activate_tab(tab);
            }
            Err(e) => {
                self.last_error = Some(e);
            }
        }
    }

    fn reload_diff_tab(&mut self, target: &DiffTarget) {
        let Some(ref repo) = self.repo_path else {
            return;
        };
        if let Ok((diff, _)) = fetch_file_diff(repo, target) {
            // Se non ci sono più hunk (es. tutte le modifiche del file sono state staged/unstaged),
            // aggiorna comunque il tab così l'utente vede che il diff è pulito.
            if let Some(CenterTab::Diff(existing)) = self.open_tabs.get_mut(self.active_tab_idx) {
                if &existing.target == target {
                    *existing = diff;
                    self.selected_hunk_lines.clear();
                }
            }
        }
    }

    pub fn open_commit_inspector(&mut self, commit_sha: &str) {
        let Some(ref repo) = self.repo_path else {
            return;
        };
        self.selected_commit_sha = Some(commit_sha.to_string());
        match fetch_commit_details(repo, commit_sha) {
            Ok((details, log)) => {
                self.git_logs.push(log);
                let tab = CenterTab::CommitView(details);
                self.upsert_and_activate_tab(tab);
            }
            Err(e) => {
                self.last_error = Some(e);
            }
        }
    }

    pub fn open_file_in_editor(&mut self, rel_path: &str) {
        let Some(ref repo) = self.repo_path else {
            return;
        };
        let full = repo.join(rel_path);
        match fs::read_to_string(&full) {
            Ok(content) => {
                let tab = CenterTab::FileEditor {
                    path: rel_path.to_string(),
                    content,
                    dirty: false,
                };
                self.upsert_and_activate_tab(tab);
            }
            Err(e) => {
                self.last_error = Some(format!("Impossibile aprire '{}': {}", rel_path, e));
            }
        }
    }

    pub fn save_file_editor(&mut self, tab_idx: usize) {
        let Some(ref repo) = self.repo_path else {
            return;
        };
        if let Some(CenterTab::FileEditor {
            path,
            content,
            dirty,
        }) = self.open_tabs.get_mut(tab_idx)
        {
            let full = repo.join(&*path);
            match fs::write(&full, content.as_bytes()) {
                Ok(_) => {
                    *dirty = false;
                    self.toast_notification =
                        Some((format!("Salvato {}", path), Instant::now()));
                    self.trigger_refresh();
                }
                Err(e) => {
                    self.last_error = Some(format!("Errore salvataggio {}: {}", path, e));
                }
            }
        }
    }

    fn upsert_and_activate_tab(&mut self, tab: CenterTab) {
        let key = tab.id_key();
        if let Some(pos) = self.open_tabs.iter().position(|t| t.id_key() == key) {
            self.open_tabs[pos] = tab;
            self.active_tab_idx = pos;
        } else {
            self.open_tabs.push(tab);
            self.active_tab_idx = self.open_tabs.len() - 1;
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx < self.open_tabs.len() {
            self.open_tabs.remove(idx);
            if self.active_tab_idx >= self.open_tabs.len() && !self.open_tabs.is_empty() {
                self.active_tab_idx = self.open_tabs.len() - 1;
            }
        }
    }

    // --- Azioni Git ad alto livello invocate dalla UI ---

    pub fn stage_file(&mut self, path: String) {
        self.spawn_git_task("Staging file...", move |repo| {
            ops::stage_paths(repo, &[path])
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn stage_paths_batch(&mut self, paths: Vec<String>) {
        self.spawn_git_task("Staging paths...", move |repo| {
            ops::stage_paths(repo, &paths)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn stage_all(&mut self) {
        self.spawn_git_task("Staging all changes...", move |repo| {
            ops::stage_all(repo)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn unstage_file(&mut self, path: String) {
        let has_commits = !self.status.head_oid.is_empty() && self.status.head_oid != "(initial)";
        self.spawn_git_task("Unstaging file...", move |repo| {
            ops::unstage_paths(repo, &[path], has_commits)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn unstage_paths_batch(&mut self, paths: Vec<String>) {
        let has_commits = !self.status.head_oid.is_empty() && self.status.head_oid != "(initial)";
        self.spawn_git_task("Unstaging paths...", move |repo| {
            ops::unstage_paths(repo, &paths, has_commits)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn unstage_all(&mut self) {
        let has_commits = !self.status.head_oid.is_empty() && self.status.head_oid != "(initial)";
        self.spawn_git_task("Unstaging all changes...", move |repo| {
            ops::unstage_all(repo, has_commits)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn discard_file_confirmed(&mut self, path: String, kind: FileStatusKind) {
        self.spawn_git_task("Discarding file changes...", move |repo| {
            ops::discard_file(repo, &path, kind)
                .map(|l| (format!("Ripristinato {}", path), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn discard_all_confirmed(&mut self) {
        self.spawn_git_task("Discarding all changes...", move |repo| {
            ops::discard_all(repo)
                .map(|logs| ("Tutte le modifiche non staged sono state annullate".into(), logs))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn apply_diff_patch(&mut self, patch: String, cached: bool, reverse: bool, label: &str) {
        let lbl = label.to_string();
        self.spawn_git_task(&lbl, move |repo| {
            ops::apply_patch(repo, &patch, cached, reverse)
                .map(|l| (String::new(), vec![l]))
                .map_err(|(e, l)| (e, vec![l]))
        });
    }

    pub fn execute_commit(&mut self, variant: CommitVariant) {
        let msg = self.commit_message.trim().to_string();
        let amend = self.commit_amend || variant == CommitVariant::Amend;
        let signoff = self.settings.commit_signoff || variant == CommitVariant::SignOff;

        if msg.is_empty() && !amend {
            self.last_error =
                Some("Inserisci un messaggio di commit prima di confermare.".to_string());
            return;
        }

        // Comportamento "Smart Commit" di VS Code: se non ci sono file in Staged ma ci sono file in WorkingTree,
        // fa automaticamente lo stage di tutte le modifiche prima del commit!
        let auto_stage_all =
            self.status.staged_changes.is_empty() && !self.status.working_changes.is_empty();
        let has_upstream = self.status.upstream.is_some();
        let current_branch = self.status.head_name.clone();

        self.commit_message.clear();
        self.commit_amend = false;

        self.spawn_git_task("Committing changes...", move |repo| {
            let mut logs = Vec::new();
            if auto_stage_all {
                let l = ops::stage_all(repo).map_err(|(e, l)| (e, vec![l]))?;
                logs.push(l);
            }
            let c_log = ops::commit(repo, &msg, amend, signoff, false)
                .map_err(|(e, l)| {
                    logs.push(l);
                    (e, logs.clone())
                })?;
            logs.push(c_log);

            match variant {
                CommitVariant::CommitAndPush => {
                    let up = if has_upstream {
                        None
                    } else {
                        Some(current_branch.as_str())
                    };
                    let p_log = ops::push(repo, up, false).map_err(|(e, l)| {
                        logs.push(l);
                        (e, logs.clone())
                    })?;
                    logs.push(p_log);
                    Ok(("Commit & Push completati".to_string(), logs))
                }
                CommitVariant::CommitAndSync => {
                    let s_logs = ops::sync_repo(repo, has_upstream, &current_branch).map_err(
                        |(e, l)| {
                            logs.push(l);
                            (e, logs.clone())
                        },
                    )?;
                    logs.extend(s_logs);
                    Ok(("Commit & Sync completati".to_string(), logs))
                }
                _ => Ok(("Commit completato".to_string(), logs)),
            }
        });
    }
}

