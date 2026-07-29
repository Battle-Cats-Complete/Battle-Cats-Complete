use std::{collections::HashSet, path::PathBuf, sync::mpsc::{channel, Receiver, Sender}, time::{Duration, Instant}};

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use tracing::{debug, info, trace, warn};

pub struct GuiWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: Receiver<PathBuf>,
}

impl GuiWatcher {
    pub fn new() -> Option<Self> {
        info!("Initializing GuiWatcher");

        let (internal_tx, internal_rx) = channel();
        let (final_tx, final_rx) = channel();

        std::thread::spawn(move || {
            debounce_loop(internal_rx, final_tx);
        });

        let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    if !matches!(event.kind, EventKind::Access(_)) {
                        for path in event.paths {
                            let path_str = path.to_string_lossy().to_lowercase();
                            if path_str.contains("raw") {
                                continue;
                            }

                            if !path_str.contains("game") && !path_str.contains("mods") {
                                continue;
                            }

                            let components: Vec<_> = path
                                .components()
                                .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
                                .collect();

                            if let Some(mods_idx) = components.iter().position(|c| c == "mods")
                                && let Some(sub_folder) = components.get(mods_idx + 2)
                                && sub_folder != "patch" && sub_folder != "icons" && sub_folder != "loose" {
                                continue;
                            }

                            trace!("GuiWatcher detected file change: {:?}", path);
                            let _ = internal_tx.send(path);
                        }
                    }
                }
                Err(e) => warn!("GuiWatcher received an event error: {}", e),
            }
        }).ok()?;

        let current_dir = std::path::Path::new(".");
        if let Err(e) = watcher.watch(current_dir, RecursiveMode::NonRecursive) {
            warn!("Failed to watch current directory: {e}");
        }

        let game_path = std::path::Path::new("game");
        if game_path.exists()
            && let Err(e) = watcher.watch(game_path, RecursiveMode::Recursive) {
            warn!("Failed to watch game directory: {e}");
        }

        let mods_path = std::path::Path::new("mods");
        if mods_path.exists()
            && let Err(e) = watcher.watch(mods_path, RecursiveMode::Recursive) {
            warn!("Failed to watch mods directory: {e}");
        }

        Some(Self {
            _watcher: watcher,
            rx: final_rx,
        })
    }
}

fn debounce_loop(rx: Receiver<PathBuf>, final_sender: Sender<PathBuf>) {
    let mut pending_paths: HashSet<PathBuf> = HashSet::new();
    let mut deadline: Option<Instant> = None;
    let mut max_deadline: Option<Instant> = None;

    let buffer_duration = Duration::from_millis(500);
    let max_duration = Duration::from_secs(2);

    loop {
        let timeout = if let (Some(d), Some(md)) = (deadline, max_deadline) {
            let now = Instant::now();
            let effective_deadline = d.min(md);

            if now >= effective_deadline {
                if !pending_paths.is_empty() {
                    debug!("Debounce loop threshold triggered, pushing {} paths", pending_paths.len());
                    for path in pending_paths.drain() {
                        let _ = final_sender.send(path);
                    }
                }
                deadline = None;
                max_deadline = None;
                Duration::from_millis(u64::MAX)
            } else {
                effective_deadline.saturating_duration_since(now)
            }
        } else {
            Duration::from_millis(u64::MAX)
        };

        match rx.recv_timeout(timeout) {
            Ok(path) => {
                trace!("Debouncer queued path: {:?}", path);
                pending_paths.insert(path);

                let now = Instant::now();
                deadline = Some(now + buffer_duration);
                if max_deadline.is_none() {
                    max_deadline = Some(now + max_duration);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                debug!("Debounce loop disconnected upstream, shutting down");
                break;
            }
        }
    }
}