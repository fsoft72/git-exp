use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
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
                    // Ignora lock temporanei di git e file .git/objects per evitare loop di refresh
                    let should_ignore = ev.paths.iter().all(|p| {
                        let s = p.to_string_lossy();
                        s.ends_with(".lock")
                            || s.contains("/.git/objects/")
                            || s.contains("/target/debug/ incremental")
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
            last_trigger: Instant::now() - Duration::from_secs(10),
            pending: false,
        })
    }

    /// Ritorna `true` (con debounce di 350ms) quando ci sono cambiamenti sul filesystem da ricaricare.
    pub fn poll_should_refresh(&mut self) -> bool {
        while let Ok(Ok(_event)) = self.rx.try_recv() {
            self.pending = true;
        }

        if self.pending && self.last_trigger.elapsed() >= Duration::from_millis(350) {
            self.pending = false;
            self.last_trigger = Instant::now();
            return true;
        }
        false
    }
}
