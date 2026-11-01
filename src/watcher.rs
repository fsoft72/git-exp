use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

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

        watcher.watch(repo_path, RecursiveMode::Recursive).ok()?;

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
        while let Ok(Ok(_event)) = self.rx.try_recv() {
            self.pending = true;
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
