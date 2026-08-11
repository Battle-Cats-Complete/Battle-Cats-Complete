use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use nyanko::enemy::unit::Battle;
use nyanko::graphics::rig::Animation;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::common::io::cache::{self, Scan};
use crate::modules::enemy::files;
use crate::modules::settings::ScannerConfig;
use crate::{Vfs, Vault};

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
}

pub fn purge() {
    cache::purge::<EnemyCache>();
}

pub fn hydrate(config: &ScannerConfig) -> Option<(u64, Vec<EnemyEntry>)> {
    if config.active_mod.is_some() {
        return None;
    }

    let (hash, cached_enemies) = cache::read::<EnemyCache>()?;
    debug!(hash, count = cached_enemies.len(), "hydrated enemies from cache");

    Some((hash, cached_enemies))
}

pub fn persist(payload: &[u8]) {
    cache::store::<EnemyCache>(payload);
}

pub fn load(config: ScannerConfig, vault: Arc<Vault>, progress: impl Fn(usize, usize) + Sync) -> Scan<Vec<EnemyEntry>> {
    scan(config, &vault, progress)
}

fn scan(config: ScannerConfig, vault: &Vault, progress: impl Fn(usize, usize) + Sync) -> Scan<Vec<EnemyEntry>> {
    trace!("starting enemy repository scan");
    let vfs = &vault.vfs;

    let raw_enemies = vault.vds.enemies.stats(vfs);

    if raw_enemies.is_empty() {
        warn!("enemy scan aborted: t_unit table unavailable");
        return Scan { data: Vec::new(), key: None, payload: None };
    }

    let names = vault.vds.enemies.names(vfs);
    let descriptions = vault.vds.enemies.descriptions(vfs);

    let total_enemies = raw_enemies.len();
    let processed_count = AtomicUsize::new(0);

    let mut parsed_enemies: Vec<EnemyEntry> = raw_enemies.par_iter().enumerate().filter_map(|(id, stats)| {
        let id_u32 = id as u32;
        let name = names.get(id).cloned().unwrap_or_default();
        let description = descriptions.get(id).cloned().unwrap_or_default();

        let enemy = process_enemy_entry(id_u32, vfs, stats.clone(), name, description, config.show_invalid_enemies);

        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
        progress(done, total_enemies);

        enemy
    }).collect();

    parsed_enemies.sort_by_key(|e| e.id);

    let key = config.active_mod.is_none().then(|| cache::content_hash(&config));
    let payload = key.and_then(|key| cache::encode::<EnemyCache>(key, &parsed_enemies));

    Scan { data: parsed_enemies, key, payload }
}

pub fn scan_single(id: u32, vault: &Vault, show_invalid: bool) -> Option<EnemyEntry> {
    let vfs = &vault.vfs;

    let stats = vault.vds.enemies.stats(vfs).get(id as usize)?.clone();

    let name = vault.vds.enemies.names(vfs).get(id as usize).cloned().unwrap_or_default();
    let description = vault.vds.enemies.descriptions(vfs).get(id as usize).cloned().unwrap_or_default();

    process_enemy_entry(id, vfs, stats, name, description, show_invalid)
}

fn process_enemy_entry(id: u32, vfs: &Vfs, stats: Battle, name: String, description: Vec<String>, show_invalid: bool) -> Option<EnemyEntry> {
    let mut resolved_icon = vfs.find(&files::icon_file(id));

    if let Some(ref p) = resolved_icon
        && is_placeholder_png(p) && !show_invalid {
        resolved_icon = None;
    }

    if resolved_icon.is_none() && !show_invalid {
        return None;
    }

    let mut atk_anim_frames = 0;
    if let Some(resolved_atk) = vfs.find(&files::maanim_file(id, 2))
        && let Ok(bytes) = fs::read(&resolved_atk) {
        let content = String::from_utf8_lossy(&bytes);
        let duration = Animation::scan_duration(content.as_bytes());
        atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
    }

    Some(EnemyEntry { id, name, description, stats, icon_path: resolved_icon, atk_anim_frames })
}
