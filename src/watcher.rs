use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// Directory names that are never watched: they are huge and produce no relevant git changes.
const IGNORED_DIR_NAMES: [&str; 3] = ["node_modules", "target", "objects"];

/// Returns true when the directory must not be watched (build output, dependencies, git internals).
fn is_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    IGNORED_DIR_NAMES.contains(&name)
        || (name == "logs" && path.parent().is_some_and(|p| p.ends_with(".git")))
}

/// Adds a non-recursive watch on `dir` and on every sub directory that is not ignored.
fn watch_tree(watcher: &mut RecommendedWatcher, dir: &Path) -> notify::Result<()> {
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // file_type() does not follow symlinks, so symlink loops are avoided
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir || is_ignored_dir(&path) {
            continue;
        }
        // A single unreadable sub directory must not disable the whole watcher
        let _ = watch_tree(watcher, &path);
    }
    Ok(())
}

pub struct RepoWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    last_trigger: Instant,
    pending: bool,
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

        watch_tree(&mut watcher, repo_path).ok()?;

        Some(Self {
            _watcher: watcher,
            rx,
            last_trigger: Instant::now(),
            pending: false,
        })
    }

    /// Consuma gli eventi del filesystem e ritorna `true` solo se ci sono state modifiche reali
    /// ed è trascorso almeno `min_interval` dall'ultimo trigger.
    pub fn poll_should_refresh(&mut self, min_interval: Duration) -> bool {
        while let Ok(Ok(event)) = self.rx.try_recv() {
            self.pending = true;
            // Newly created directories are not covered by the initial walk: watch them too
            if matches!(event.kind, EventKind::Create(_)) {
                for path in event
                    .paths
                    .iter()
                    .filter(|p| p.is_dir() && !is_ignored_dir(p))
                {
                    let _ = watch_tree(&mut self._watcher, path);
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
    fn test_is_ignored_dir() {
        assert!(is_ignored_dir(Path::new("/r/app/node_modules")));
        assert!(is_ignored_dir(Path::new("/r/target")));
        assert!(is_ignored_dir(Path::new("/r/.git/objects")));
        assert!(is_ignored_dir(Path::new("/r/.git/logs")));
        assert!(!is_ignored_dir(Path::new("/r/src/logs")));
        assert!(!is_ignored_dir(Path::new("/r/src")));
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
