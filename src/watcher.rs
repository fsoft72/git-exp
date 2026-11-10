use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// Directory names skipped when git cannot tell which folders are ignored (e.g. not a git repo).
const FALLBACK_IGNORED_DIR_NAMES: [&str; 2] = ["node_modules", "target"];

/// Folders that must not be scanned: the ones ignored by `.gitignore` plus git internals.
struct IgnoredDirs {
    root: PathBuf,
    /// Absolute paths of the directories reported as ignored by git
    dirs: HashSet<PathBuf>,
    /// False when git failed: then only the fallback names are used
    git_ok: bool,
}

impl IgnoredDirs {
    /// Asks git for the ignored directories of the repository (one fast call, nothing is scanned by us).
    fn load(root: &Path) -> Self {
        let output = Command::new("git")
            .current_dir(root)
            .args([
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "--directory",
                "-z",
            ])
            .output();
        let Ok(out) = output
            .map_err(|_| ())
            .and_then(|o| o.status.success().then_some(o).ok_or(()))
        else {
            return Self {
                root: root.to_path_buf(),
                dirs: HashSet::new(),
                git_ok: false,
            };
        };
        let dirs = String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter_map(|entry| entry.strip_suffix('/'))
            .map(|entry| root.join(entry))
            .collect();
        Self {
            root: root.to_path_buf(),
            dirs,
            git_ok: true,
        }
    }

    /// Returns true when the directory must not be watched.
    fn is_ignored_dir(&self, path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "objects" && path.parent().is_some_and(|p| p.ends_with(".git")) {
            return true;
        }
        if name == "logs" && path.parent().is_some_and(|p| p.ends_with(".git")) {
            return true;
        }
        if self.git_ok {
            return self.dirs.contains(path);
        }
        FALLBACK_IGNORED_DIR_NAMES.contains(&name)
    }

    /// Checks a directory created after startup against the ignore rules and remembers the answer.
    fn check_new_dir(&mut self, path: &Path) -> bool {
        if self.is_ignored_dir(path) {
            return true;
        }
        if !self.git_ok || !path.starts_with(&self.root) {
            return false;
        }
        let ignored = Command::new("git")
            .current_dir(&self.root)
            .args(["check-ignore", "-q", "--"])
            .arg(path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ignored {
            self.dirs.insert(path.to_path_buf());
        }
        ignored
    }
}

/// Adds a non-recursive watch on `dir` and on every sub directory that is not ignored.
fn watch_tree(
    watcher: &mut RecommendedWatcher,
    ignored: &IgnoredDirs,
    dir: &Path,
) -> notify::Result<()> {
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // file_type() does not follow symlinks, so symlink loops are avoided
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir || ignored.is_ignored_dir(&path) {
            continue;
        }
        // A single unreadable sub directory must not disable the whole watcher
        let _ = watch_tree(watcher, ignored, &path);
    }
    Ok(())
}

pub struct RepoWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    last_trigger: Instant,
    pending: bool,
    ignored: IgnoredDirs,
}

impl RepoWatcher {
    pub fn new(repo_path: &Path, ctx: egui::Context) -> Option<Self> {
        let (tx, rx) = channel();
        let egui_ctx = ctx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(ref ev) = res {
                    // IMPORTANTE su Linux (inotify): ignora tutti gli eventi `Access` (open/read/close-nowrite)
                    // altrimenti l'esecuzione di `git status` legge `.git` e innesca un loop infinito!
                    if !matches!(
                        ev.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) {
                        return;
                    }

                    // Ignora lock temporanei di git e directory di build/oggetti interni
                    let should_ignore = ev.paths.iter().all(|p| {
                        let s = p.to_string_lossy();
                        s.ends_with(".lock")
                            || s.contains("/.git/objects/")
                            || s.contains("/.git/logs/")
                            || s.contains("/target/")
                            || s.contains("/node_modules/")
                    });
                    if should_ignore {
                        return;
                    }
                }
                let _ = tx.send(res);
                egui_ctx.request_repaint();
            },
            Config::default(),
        )
        .ok()?;

        let ignored = IgnoredDirs::load(repo_path);
        watch_tree(&mut watcher, &ignored, repo_path).ok()?;

        Some(Self {
            _watcher: watcher,
            rx,
            last_trigger: Instant::now(),
            pending: false,
            ignored,
        })
    }

    /// Consuma gli eventi del filesystem e ritorna `true` solo se ci sono state modifiche reali
    /// ed è trascorso almeno `min_interval` dall'ultimo trigger.
    pub fn poll_should_refresh(&mut self, min_interval: Duration) -> bool {
        while let Ok(Ok(event)) = self.rx.try_recv() {
            self.pending = true;
            // Newly created directories are not covered by the initial walk: watch them too
            if matches!(event.kind, EventKind::Create(_)) {
                for path in event.paths.iter().filter(|p| p.is_dir()) {
                    if !self.ignored.check_new_dir(path) {
                        let _ = watch_tree(&mut self._watcher, &self.ignored, path);
                    }
                }
            }
        }

        if self.pending && self.last_trigger.elapsed() >= min_interval {
            self.pending = false;
            self.last_trigger = Instant::now();
            return true;
        }
        false
    }

    /// Svuota eventuali eventi accumulati durante l'esecuzione stessa di un comando git.
    pub fn clear_pending(&mut self) {
        while self.rx.try_recv().is_ok() {}
        self.pending = false;
        self.last_trigger = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ignored_dir_fallback() {
        let ig = IgnoredDirs::load(Path::new("/nonexistent-gitexp-dir"));
        assert!(!ig.git_ok);
        assert!(ig.is_ignored_dir(Path::new("/r/app/node_modules")));
        assert!(ig.is_ignored_dir(Path::new("/r/.git/objects")));
        assert!(ig.is_ignored_dir(Path::new("/r/.git/logs")));
        assert!(!ig.is_ignored_dir(Path::new("/r/src/logs")));
        assert!(!ig.is_ignored_dir(Path::new("/r/src")));
    }

    #[test]
    fn test_gitignore_dirs_are_not_watched() {
        let dir = std::env::temp_dir().join(format!("gitexp-ign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("build/deep")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join(".gitignore"), "build/\ncache2/\n").unwrap();
        assert!(
            Command::new("git")
                .current_dir(&dir)
                .args(["init", "-q"])
                .status()
                .unwrap()
                .success()
        );

        let mut w = RepoWatcher::new(&dir, egui::Context::default()).expect("watcher");
        std::thread::sleep(Duration::from_millis(200));
        w.clear_pending();

        // A folder ignored through .gitignore is not scanned, whatever its name
        std::fs::write(dir.join("build/deep/x.o"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(!w.poll_should_refresh(Duration::ZERO));

        // A new ignored folder is recognised too; changes inside it are not reported
        std::fs::create_dir_all(dir.join("cache2")).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        w.poll_should_refresh(Duration::ZERO);
        w.clear_pending();
        std::fs::write(dir.join("cache2/y.bin"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(!w.poll_should_refresh(Duration::ZERO));

        // Normal folders still are
        std::fs::write(dir.join("src/a.rs"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(w.poll_should_refresh(Duration::ZERO));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_watcher_skips_ignored_and_sees_new_dirs() {
        let dir = std::env::temp_dir().join(format!("gitexp-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let mut w = RepoWatcher::new(&dir, egui::Context::default()).expect("watcher");

        std::fs::write(dir.join("src/a.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(w.poll_should_refresh(Duration::ZERO));

        // A directory created after startup must be watched too
        std::fs::create_dir_all(dir.join("newdir")).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(w.poll_should_refresh(Duration::ZERO));
        w.clear_pending();
        std::fs::write(dir.join("newdir/b.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(w.poll_should_refresh(Duration::ZERO));

        // Changes inside ignored directories are not reported
        w.clear_pending();
        std::fs::write(dir.join("node_modules/pkg/c.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(!w.poll_should_refresh(Duration::ZERO));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
