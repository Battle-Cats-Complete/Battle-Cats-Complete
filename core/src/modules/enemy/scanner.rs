use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use nyanko::enemy::unit::Battle;
use nyanko::graphics::rig::Animation;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::common::io::cache;
use crate::common::resolver;
use crate::modules::enemy::paths;
use crate::modules::enemy::waiter::{enemyname, enemypicturebook, t_unit};
use crate::modules::settings::ScannerConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyEntry {
    pub id: u32,
    pub name: String,
    pub description: Vec<String>,
    pub stats: Battle,
    pub icon_path: Option<PathBuf>,
    pub atk_anim_frames: i32,
}

impl EnemyEntry {
    pub fn base_id_str(&self) -> String { format!("{:03}", self.id) }
    pub fn id_str(&self) -> String { format!("{}-E", self.base_id_str()) }
    pub fn display_name(&self) -> String {
        if self.name.is_empty() { self.id_str() } else { self.name.clone() }
    }
}

fn is_placeholder_png(path: &Path) -> bool {
    let mut file = match File::open(path) { Ok(f) => f, Err(_) => return true, };
    let mut buffer = [0u8; 25];
    if file.read_exact(&mut buffer).is_err() { return true; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return true; }
    buffer[24] < 4
}

struct EnemyCache;

impl cache::CacheSpec for EnemyCache {
    type Data = Vec<EnemyEntry>;
    const FILE: &'static str = "enemies_cache.bin";
    const VERSION: u32 = 1;
}

pub fn load(config: ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> Vec<EnemyEntry> {
    if let Some((hash, cached_enemies)) = cache::read::<EnemyCache>() {
        debug!(hash, count = cached_enemies.len(), "loaded enemies from cache fast-path");
        return cached_enemies;
    }

    scan(config, progress)
}

fn scan(config: ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> Vec<EnemyEntry> {
    trace!("starting enemy repository scan");
    let root = Path::new(paths::DIR_ENEMIES);
    let priority = &config.language_priority;

    let t_unit_p = paths::stats(root);

    let Some(t_unit_parent) = t_unit_p.parent() else {
        warn!("enemy scan aborted: stats path has no parent");
        return Vec::new();
    };
    let Some(t_unit_name) = t_unit_p.file_name().and_then(|n| n.to_str()) else {
        warn!("enemy scan aborted: stats path has no file name");
        return Vec::new();
    };

    let Some(raw_enemies) = t_unit(t_unit_parent, t_unit_name, priority) else {
        warn!("enemy scan aborted: t_unit table unavailable");
        return Vec::new();
    };

    let names = enemyname(root, priority);
    let descriptions = enemypicturebook(root, priority);

    let total_enemies = raw_enemies.len();
    let processed_count = AtomicUsize::new(0);

    let mut parsed_enemies: Vec<EnemyEntry> = raw_enemies.into_par_iter().enumerate().filter_map(|(id, stats)| {
        let id_u32 = id as u32;
        let name = names.get(id).cloned().unwrap_or_default();
        let description = descriptions.get(id).cloned().unwrap_or_default();

        let enemy = process_enemy_entry(id_u32, stats, name, description, &config);

        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
        progress(done, total_enemies);

        enemy
    }).collect();

    parsed_enemies.sort_by_key(|e| e.id);

    if !resolver::is_mod_active() {
        let current_hash = cache::get_game_hash(None);
        cache::write::<EnemyCache>(current_hash, &parsed_enemies);
    }

    parsed_enemies
}

pub fn scan_single(id: u32, config: &ScannerConfig) -> Option<EnemyEntry> {
    let root = Path::new(paths::DIR_ENEMIES);
    let priority = &config.language_priority;

    let t_unit_p = paths::stats(root);

    let Some(t_unit_parent) = t_unit_p.parent() else { return None; };
    let Some(t_unit_name) = t_unit_p.file_name().and_then(|n| n.to_str()) else { return None; };

    let raw_enemies = t_unit(t_unit_parent, t_unit_name, priority)?;
    let stats = raw_enemies.get(id as usize)?.clone();

    let name = enemyname(root, priority).get(id as usize).cloned().unwrap_or_default();
    let description = enemypicturebook(root, priority).get(id as usize).cloned().unwrap_or_default();

    process_enemy_entry(id, stats, name, description, config)
}

fn process_enemy_entry(id: u32, stats: Battle, name: String, description: Vec<String>, config: &ScannerConfig) -> Option<EnemyEntry> {
    let root = Path::new(paths::DIR_ENEMIES);
    let priority = &config.language_priority;

    let icon_p = paths::icon(root, id);
    let mut resolved_icon = None;
    if let (Some(parent), Some(icon_name)) = (icon_p.parent(), icon_p.file_name().and_then(|n| n.to_str())) {
        resolved_icon = resolver::get(parent, [icon_name], priority).into_iter().next();
    }

    if let Some(ref p) = resolved_icon
        && is_placeholder_png(p) && !config.show_invalid_enemies {
        resolved_icon = None;
    }

    if resolved_icon.is_none() && !config.show_invalid_enemies {
        return None;
    }

    let mut atk_anim_frames = 0;
    let atk_p = paths::maanim(root, id, 2);
    if let (Some(parent), Some(anim_name)) = (atk_p.parent(), atk_p.file_name().and_then(|n| n.to_str()))
        && let Some(resolved_atk) = resolver::get(parent, [anim_name], priority).into_iter().next()
        && let Ok(bytes) = fs::read(&resolved_atk) {
        let content = String::from_utf8_lossy(&bytes);
        let duration = Animation::scan_duration(content.as_bytes());
        atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
    }

    Some(EnemyEntry { id, name, description, stats, icon_path: resolved_icon, atk_anim_frames })
}
