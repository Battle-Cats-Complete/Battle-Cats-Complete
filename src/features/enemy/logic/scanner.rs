use std::path::{Path, PathBuf};
use rayon::prelude::*;

use crate::features::enemy::paths;
use crate::features::enemy::data::{t_unit::{self, EnemyRaw}, enemyname, enemypicturebook};
use crate::features::settings::logic::handle::ScannerConfig;
use crate::global::maanim::Animation;

#[derive(Clone, Debug)]
pub struct EnemyEntry {
    pub id: u32,
    pub name: String,
    pub description: Vec<String>,
    pub stats: EnemyRaw,
    pub icon_path: Option<PathBuf>,
    pub atk_anim_frames: i32,
}

pub fn scan_all(config: &ScannerConfig) -> Vec<EnemyEntry> {
    let root = Path::new(paths::DIR_ENEMIES);
    
    // 1. Load the raw math stats
    let t_unit_path = paths::stats(root);
    let raw_enemies = match t_unit::load_all(&t_unit_path) {
        Some(e) => e,
        None => return Vec::new(),
    };

    // 2. Load names and descriptions using the language hierarchy
    let lang_code = &config.language;
    let names = enemyname::load(root, lang_code);
    let descriptions = enemypicturebook::load(root, lang_code);

    // 3. Zip everything together in parallel
    let entries: Vec<EnemyEntry> = raw_enemies
        .into_par_iter()
        .enumerate()
        .map(|(id, stats)| {
            let id_u32 = id as u32;
            
            // Check for the icon
            let icon_p = paths::icon(root, id_u32);
            let icon_path = if icon_p.exists() { Some(icon_p) } else { None };

            // Find the attack animation frames (Usually index 2 for attack)
            let atk_maanim_path = paths::maanim(root, id_u32, 2);
            let mut atk_anim_frames = 0;
            if atk_maanim_path.exists() {
                if let Ok(file_content) = std::fs::read_to_string(&atk_maanim_path) {
                    let duration = Animation::scan_duration(&file_content);
                    atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
                }
            }

            // Safely grab the text (fallback to empty if the TSV/CSV was shorter than t_unit)
            let name = names.get(id).cloned().unwrap_or_default();
            let description = descriptions.get(id).cloned().unwrap_or_default();

            EnemyEntry {
                id: id_u32,
                name,
                description,
                stats,
                icon_path,
                atk_anim_frames,
            }
        })
        .collect();

    entries
}