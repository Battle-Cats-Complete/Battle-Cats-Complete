use std::path::{Path, PathBuf};
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;

use crate::features::enemy::paths;
use crate::features::enemy::data::{t_unit::{self, EnemyRaw}, enemyname, enemypicturebook};
use crate::features::settings::logic::state::ScannerConfig;
use crate::global::formats::maanim::Animation;

#[derive(Clone, Debug)]
pub struct EnemyEntry {
    pub id: u32,
    pub name: String,
    pub description: Vec<String>,
    pub stats: EnemyRaw,
    pub icon_path: Option<PathBuf>,
    pub atk_anim_frames: i32,
}

impl EnemyEntry {
    pub fn base_id_str(&self) -> String {
        format!("{:03}", self.id)
    }

    pub fn id_str(&self) -> String {
        format!("{}-E", self.base_id_str())
    }

    pub fn display_name(&self) -> String {
        if self.name.is_empty() {
            self.id_str()
        } else {
            self.name.clone()
        }
    }
}

// Reads just the first 25 bytes of a PNG to grab the bit-depth instantly
fn is_placeholder_png(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return true, // If it fails to open, consider it junk
    };

    let mut buffer = [0u8; 25];
    if file.read_exact(&mut buffer).is_err() {
        return true; 
    }

    // Verify it's actually a PNG (first 8 bytes)
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG {
        return true;
    }

    // Byte index 24 (the 25th byte) is the Bit Depth in the IHDR chunk
    let bit_depth = buffer[24];
    
    // If bit depth is less than 8, it's a dummy placeholder
    bit_depth < 8
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
        .filter_map(|(id, stats)| {
            let id_u32 = id as u32;
            let icon_p = paths::icon(root, id_u32);
            
            // Instantly drop enemies missing an icon folder/file
            if !icon_p.exists() {
                return None;
            }

            // Instantly drop enemies with a 1-bit placeholder icon
            if is_placeholder_png(&icon_p) {
                return None;
            }

            // Find the attack animation frames
            let atk_maanim_path = paths::maanim(root, id_u32, 2);
            let mut atk_anim_frames = 0;
            if let Ok(file_content) = std::fs::read_to_string(&atk_maanim_path) {
                let duration = Animation::scan_duration(&file_content);
                atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
            }

            // Safely grab the text
            let name = names.get(id).cloned().unwrap_or_default();
            let description = descriptions.get(id).cloned().unwrap_or_default();

            Some(EnemyEntry {
                id: id_u32,
                name,
                description,
                stats,
                icon_path: Some(icon_p), 
                atk_anim_frames,
            })
        })
        .collect();

    entries
}