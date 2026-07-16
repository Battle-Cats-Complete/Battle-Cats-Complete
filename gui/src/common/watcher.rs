use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub(crate) struct GuiWatcher {
    _watcher: RecommendedWatcher,
    pub rx: Receiver<PathBuf>,
}

impl GuiWatcher {
    pub(crate) fn new(ctx: egui::Context) -> Option<Self> {
        let (internal_tx, internal_rx) = channel();
        let (final_tx, final_rx) = channel();

        thread::spawn(move || {
            debounce_loop(internal_rx, final_tx, ctx);
        });

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && !matches!(event.kind, notify::EventKind::Access(_)) {
                for path in event.paths {
                    let path_str = path.to_string_lossy().to_lowercase();
                    if path_str.contains("raw") { continue; }

                    if !path_str.contains("game") && !path_str.contains("mods") {
                        continue;
                    }

                    let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
                    if let Some(mods_idx) = components.iter().position(|c| c == "mods")
                        && let Some(sub_folder) = components.get(mods_idx + 2)
                            && sub_folder != "patch" && sub_folder != "icons" && sub_folder != "loose" {
                            continue;
                        }

                    let _ = internal_tx.send(path);
                }
            }
        }).ok()?;

        let _ = watcher.watch(Path::new("."), RecursiveMode::NonRecursive);

        let path = Path::new("game");
        if path.exists() {
            let _ = watcher.watch(path, RecursiveMode::Recursive);
        }

        let mods_path = Path::new("mods");
        if mods_path.exists() {
            let _ = watcher.watch(mods_path, RecursiveMode::Recursive);
        }

        Some(Self {
            _watcher: watcher,
            rx: final_rx,
        })
    }
}

fn debounce_loop(rx: Receiver<PathBuf>, final_sender: Sender<PathBuf>, ctx: egui::Context) {
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
                    for path in pending_paths.drain() {
                        let _ = final_sender.send(path);
                    }
                    ctx.request_repaint();
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
                pending_paths.insert(path);
                let now = Instant::now();
                deadline = Some(now + buffer_duration);
                if max_deadline.is_none() {
                    max_deadline = Some(now + max_duration);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}