use eframe::egui;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use super::state::EnemyListState;
use super::loader;
use crate::features::enemy::paths;
use crate::features::settings::logic::state::ScannerConfig;

pub struct EnemyWatchers {
    _watcher: RecommendedWatcher,
}

impl EnemyWatchers {
    pub fn new(sender: Sender<PathBuf>, ctx: egui::Context) -> Option<Self> {
        let (internal_tx, internal_rx) = channel();

        thread::spawn(move || {
            debounce_loop(internal_rx, sender, ctx);
        });

        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    for path in event.paths {
                        let _ = internal_tx.send(path);
                    }
                }
            }
        }).ok()?;

        let mut w = Self { _watcher: watcher };
        let root_path = Path::new(paths::DIR_ENEMIES);
        
        if root_path.exists() {
            let _ = w._watcher.watch(root_path, RecursiveMode::Recursive);
        }

        Some(w)
    }

    pub fn handle_events(
        state: &mut EnemyListState, 
        receiver: &Receiver<PathBuf>, 
        ctx: &egui::Context,
        config: &ScannerConfig
    ) {
        while let Ok(path) = receiver.try_recv() {
            let path_str = path.to_string_lossy().to_string();

            if path_str.contains("t_unit") || path_str.contains("EnemyName") || path_str.contains("EnemyPictureBook") {
                loader::restart_scan(state, config.clone());
                ctx.request_repaint();
                continue;
            }

            if let Some(id) = extract_enemy_id(&path_str) {
                if path_str.contains(".maanim") || path_str.contains(".png") || path_str.contains(".imgcut") || path_str.contains(".mamodel") {
                    if state.selected_tab == super::state::EnemyDetailTab::Animation {
                        if state.selected_enemy == Some(id) {
                            state.anim_viewer.loaded_id.clear();
                            state.anim_viewer.texture_version += 1; 
                            ctx.request_repaint();
                        }
                    }
                }

                state.enemy_list.flush_icon(id); // UNIFIED
                if state.selected_enemy == Some(id) {
                    state.detail_texture = None; 
                    state.detail_key.clear();
                }
                
                loader::refresh_enemy(state, id, config);
                ctx.request_repaint();
            }
        }
    }
}

fn debounce_loop(rx: Receiver<PathBuf>, tx: Sender<PathBuf>, ctx: egui::Context) {
    let mut pending = HashSet::new();
    let mut last_event = Instant::now();
    let debounce_duration = Duration::from_millis(150);

    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(path) => {
                pending.insert(path);
                last_event = Instant::now();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if !pending.is_empty() && last_event.elapsed() >= debounce_duration {
                    for path in pending.drain() {
                        let _ = tx.send(path);
                    }
                    ctx.request_repaint();
                }
            }
            Err(_) => break, 
        }
    }
}

fn extract_enemy_id(path_str: &str) -> Option<u32> {
    let path = Path::new(path_str);
    if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
        if parent.len() == 3 && parent.chars().all(|c| c.is_ascii_digit()) {
            return parent.parse::<u32>().ok();
        }
    }
    None
}

pub fn init(state: &mut super::state::EnemyListState, ctx: &egui::Context) {
    if state.watchers.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        state.watch_receiver = Some(rx);
        state.watchers = EnemyWatchers::new(tx, ctx.clone());
    }
}

#[allow(dead_code)] // TODO: Remove this once Global Watcher is hooked up
pub fn handle_event(
    state: &mut super::state::EnemyListState, 
    ctx: &egui::Context, 
    path: &std::path::PathBuf, 
    config: ScannerConfig
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = tx.send(path.clone());
    
    EnemyWatchers::handle_events(state, &rx, ctx, &config);
}