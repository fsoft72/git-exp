use crate::git::cli::{GitCommandLog, run_git};
use crate::theme::VsCodeTheme;
use egui::Color32;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatusKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
    TypeChanged,
}

impl FileStatusKind {
    /// Lettera di badge identica a VS Code (`M`, `A`, `D`, `R`, `C`, `U`).
    pub fn badge_letter(self) -> &'static str {
        match self {
            Self::Modified | Self::TypeChanged => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
            Self::Untracked => "U",
            Self::Conflicted => "!",
        }
    }

    /// Tooltip descrittivo stile VS Code.
    pub fn label(self, staged: bool, lang: crate::i18n::Language) -> &'static str {
        use crate::i18n::tr;
        match (self, staged) {
            (Self::Modified, true) => tr(lang, "Index Modified", "Modificato nell'Index"),
            (Self::Modified, false) => tr(lang, "Modified", "Modificato"),
            (Self::Added, _) => tr(lang, "Index Added", "Aggiunto nell'Index"),
            (Self::Deleted, true) => tr(lang, "Index Deleted", "Eliminato nell'Index"),
            (Self::Deleted, false) => tr(lang, "Deleted", "Eliminato"),
            (Self::Renamed, _) => tr(lang, "Index Renamed", "Rinominato nell'Index"),
            (Self::Copied, _) => tr(lang, "Index Copied", "Copiato nell'Index"),
            (Self::Untracked, _) => tr(lang, "Untracked", "Non tracciato"),
            (Self::Conflicted, _) => tr(lang, "Merge Conflict", "Conflitto di Merge"),
            (Self::TypeChanged, _) => tr(lang, "Type Changed", "Tipo Modificato"),
        }
    }

    /// Colore ufficiale del decoratore Git in VS Code Dark Modern.
    pub fn color(self) -> Color32 {
        match self {
            Self::Modified | Self::TypeChanged => VsCodeTheme::GIT_MODIFIED,
            Self::Added | Self::Renamed | Self::Copied => VsCodeTheme::GIT_ADDED,
            Self::Untracked => VsCodeTheme::GIT_UNTRACKED,
            Self::Deleted => VsCodeTheme::GIT_DELETED,
            Self::Conflicted => VsCodeTheme::GIT_CONFLICT,
        }
    }

    pub fn from_porcelain_char(c: char) -> Option<Self> {
        match c {
            'M' => Some(Self::Modified),
            'A' => Some(Self::Added),
            'D' => Some(Self::Deleted),
            'R' => Some(Self::Renamed),
            'C' => Some(Self::Copied),
            'T' => Some(Self::TypeChanged),
            'U' => Some(Self::Conflicted),
            '?' => Some(Self::Untracked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeGroup {
    Merge,
    Staged,
    WorkingTree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub orig_path: Option<String>,
    pub kind: FileStatusKind,
    pub group: ChangeGroup,
    pub raw_xy: String,
}

impl FileChange {
    pub fn file_name(&self) -> &str {
        Path::new(&self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.path)
    }

    pub fn parent_dir(&self) -> &str {
        Path::new(&self.path)
            .parent()
            .and_then(|s| s.to_str())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub commit_short: String,
    pub upstream: Option<String>,
    pub subject: String,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub reference: String, // e.g. stash@{0}
    pub message: String,
    pub relative_time: String,
}

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub commit_short: String,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct RepoStatus {
    pub head_name: String,
    pub head_oid: String,
    pub is_detached: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub merge_changes: Vec<FileChange>,
    pub staged_changes: Vec<FileChange>,
    pub working_changes: Vec<FileChange>,
    /// Paths ignored by `.gitignore` (directories end with `/`), shown in grey in the sidebar
    pub ignored_paths: Vec<String>,
    pub branches: Vec<BranchInfo>,
    pub stashes: Vec<StashEntry>,
    pub tags: Vec<TagInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub head_commit_message: String,
    pub merge_head_in_progress: bool,
    pub rebase_in_progress: bool,
}

impl RepoStatus {
    pub fn total_changes_count(&self) -> usize {
        self.merge_changes.len() + self.staged_changes.len() + self.working_changes.len()
    }
}

/// Parsa l'output di `git status --porcelain=v2 --branch -z -uall`.
pub fn parse_porcelain_v2_z(input: &str) -> RepoStatus {
    let mut status = RepoStatus {
        head_name: "main".to_string(),
        ..Default::default()
    };

    let mut tokens = input.split('\0').peekable();
    while let Some(entry) = tokens.next() {
        if entry.is_empty() {
            continue;
        }
        if let Some(rest) = entry.strip_prefix("# ") {
            if let Some(oid) = rest.strip_prefix("branch.oid ") {
                status.head_oid = oid.trim().to_string();
            } else if let Some(head) = rest.strip_prefix("branch.head ") {
                let h = head.trim();
                if h == "(detached)" {
                    status.is_detached = true;
                    let short = status.head_oid.chars().take(7).collect::<String>();
                    status.head_name = if short.is_empty() {
                        "HEAD detached".to_string()
                    } else {
                        format!("HEAD ({})", short)
                    };
                } else {
                    status.head_name = h.to_string();
                }
            } else if let Some(up) = rest.strip_prefix("branch.upstream ") {
                status.upstream = Some(up.trim().to_string());
            } else if let Some(ab) = rest.strip_prefix("branch.ab ") {
                for part in ab.split_whitespace() {
                    if let Some(a) = part.strip_prefix('+') {
                        status.ahead = a.parse().unwrap_or(0);
                    } else if let Some(b) = part.strip_prefix('-') {
                        status.behind = b.parse().unwrap_or(0);
                    }
                }
            }
        } else if entry.starts_with("1 ") || entry.starts_with("2 ") {
            let is_rename = entry.starts_with("2 ");
            // Format 1: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            // Format 2: 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>\0<origPath>
            let parts: Vec<&str> = entry
                .splitn(if is_rename { 10 } else { 9 }, ' ')
                .collect();
            if parts.len() >= 9 {
                let xy = parts[1];
                let path = parts[if is_rename { 9 } else { 8 }].to_string();
                let orig_path = if is_rename {
                    tokens.next().map(|s| s.to_string())
                } else {
                    None
                };

                let mut chars = xy.chars();
                let x = chars.next().unwrap_or('.');
                let y = chars.next().unwrap_or('.');

                if let Some(kind) = FileStatusKind::from_porcelain_char(x) {
                    status.staged_changes.push(FileChange {
                        path: path.clone(),
                        orig_path: orig_path.clone(),
                        kind,
                        group: ChangeGroup::Staged,
                        raw_xy: xy.to_string(),
                    });
                }
                if let Some(kind) = FileStatusKind::from_porcelain_char(y) {
                    status.working_changes.push(FileChange {
                        path,
                        orig_path,
                        kind,
                        group: ChangeGroup::WorkingTree,
                        raw_xy: xy.to_string(),
                    });
                }
            }
        } else if entry.starts_with("u ") {
            // Unmerged entry: u <xy> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
            let parts: Vec<&str> = entry.splitn(11, ' ').collect();
            if parts.len() >= 11 {
                let xy = parts[1];
                let path = parts[10].to_string();
                status.merge_changes.push(FileChange {
                    path,
                    orig_path: None,
                    kind: FileStatusKind::Conflicted,
                    group: ChangeGroup::Merge,
                    raw_xy: xy.to_string(),
                });
            }
        } else if let Some(path) = entry.strip_prefix("? ") {
            status.working_changes.push(FileChange {
                path: path.to_string(),
                orig_path: None,
                kind: FileStatusKind::Untracked,
                group: ChangeGroup::WorkingTree,
                raw_xy: "??".to_string(),
            });
        }
    }

    status
}

/// Raccoglie lo stato completo del repository Git (status, branch, stash, tag, remote, ultimo commit).
pub fn fetch_repo_status(repo_path: &Path) -> Result<(RepoStatus, Vec<GitCommandLog>), String> {
    let mut logs = Vec::new();

    let status_out = run_git(
        repo_path,
        &["status", "--porcelain=v2", "--branch", "-z", "-uall"],
        None,
        false,
    )?;
    logs.push(status_out.log.clone());
    if !status_out.is_success() {
        return Err(status_out.stderr);
    }

    let mut status = parse_porcelain_v2_z(&status_out.stdout);

    // Check merge/rebase state in .git
    let git_dir = repo_path.join(".git");
    status.merge_head_in_progress = git_dir.join("MERGE_HEAD").exists();
    status.rebase_in_progress =
        git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists();

    // Fetch ignored paths (whole ignored directories are reported once, not their content)
    if let Ok(ign_out) = run_git(
        repo_path,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "-z",
        ],
        None,
        false,
    ) {
        if ign_out.is_success() {
            status.ignored_paths = ign_out
                .stdout
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect();
            status.ignored_paths.sort();
        }
    }

    // Fetch branches
    if let Ok(br_out) = run_git(
        repo_path,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)|%(refname)|%(objectname:short)|%(upstream:short)|%(HEAD)|%(contents:subject)",
            "refs/heads",
            "refs/remotes",
        ],
        None,
        false,
    ) {
        logs.push(br_out.log);
        for line in br_out.stdout.lines() {
            let parts: Vec<&str> = line.splitn(6, '|').collect();
            if parts.len() >= 6 {
                let short_name = parts[0].trim();
                let full_ref = parts[1].trim();
                if short_name.ends_with("/HEAD") || short_name == "origin" {
                    continue;
                }
                let is_remote = full_ref.starts_with("refs/remotes/");
                let commit_short = parts[2].trim().to_string();
                let upstream = if parts[3].trim().is_empty() {
                    None
                } else {
                    Some(parts[3].trim().to_string())
                };
                let is_current = parts[4].trim() == "*";
                let subject = parts[5].trim().to_string();

                status.branches.push(BranchInfo {
                    name: short_name.to_string(),
                    is_remote,
                    is_current,
                    commit_short,
                    upstream,
                    subject,
                });
            }
        }
    }

    // Fetch stashes
    if let Ok(st_out) = run_git(
        repo_path,
        &["stash", "list", "--format=%gd|%gs|%cr"],
        None,
        false,
    ) {
        logs.push(st_out.log);
        for line in st_out.stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 3 {
                status.stashes.push(StashEntry {
                    reference: parts[0].to_string(),
                    message: parts[1].to_string(),
                    relative_time: parts[2].to_string(),
                });
            }
        }
    }

    // Fetch tags
    if let Ok(tag_out) = run_git(
        repo_path,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)|%(objectname:short)",
            "refs/tags",
        ],
        None,
        false,
    ) {
        logs.push(tag_out.log);
        for line in tag_out.stdout.lines() {
            if let Some((name, sha)) = line.split_once('|') {
                status.tags.push(TagInfo {
                    name: name.trim().to_string(),
                    commit_short: sha.trim().to_string(),
                });
            }
        }
    }

    // Fetch remotes
    if let Ok(rem_out) = run_git(repo_path, &["remote", "-v"], None, false) {
        logs.push(rem_out.log);
        for line in rem_out.stdout.lines() {
            if line.ends_with("(fetch)") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    status.remotes.push(RemoteInfo {
                        name: parts[0].to_string(),
                        url: parts[1].to_string(),
                    });
                }
            }
        }
    }

    // Fetch last commit message (useful for Amend)
    if status.head_oid != "(initial)" && !status.head_oid.is_empty() {
        if let Ok(msg_out) = run_git(repo_path, &["log", "-1", "--format=%B"], None, false) {
            if msg_out.is_success() {
                status.head_commit_message = msg_out.stdout.trim_end().to_string();
            }
        }
    }

    Ok((status, logs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_v2_z() {
        let sample = "# branch.oid 1234567890abcdef\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\01 MM N... 100644 100644 100644 111111 222222 src/main.rs\02 R. N... 100644 100644 100644 111111 222222 R100 src/new.rs\0src/old.rs\0? untracked.txt\0";
        let st = parse_porcelain_v2_z(sample);
        assert_eq!(st.head_name, "main");
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!(st.ahead, 2);
        assert_eq!(st.behind, 1);
        assert_eq!(st.staged_changes.len(), 2);
        assert_eq!(st.working_changes.len(), 2);
        assert_eq!(st.staged_changes[0].path, "src/main.rs");
        assert_eq!(st.staged_changes[0].kind, FileStatusKind::Modified);
        assert_eq!(st.staged_changes[1].path, "src/new.rs");
        assert_eq!(st.staged_changes[1].orig_path.as_deref(), Some("src/old.rs"));
        assert_eq!(st.staged_changes[1].kind, FileStatusKind::Renamed);
        assert_eq!(st.working_changes[1].path, "untracked.txt");
        assert_eq!(st.working_changes[1].kind, FileStatusKind::Untracked);
    }
}
