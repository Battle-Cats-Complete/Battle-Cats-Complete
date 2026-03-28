use std::path::{Path, PathBuf};
use regex::Regex;
use crate::features::stage::patterns;

pub struct StageMatcher {
    map_data: Regex,
    map_name: Regex,
    map_sn: Regex,
    stage_normal: Regex,
    stage_file: Regex,
    stage_name: Regex,
    legacy_stage_name: Regex,
    castle: Regex,
    bg_map: Regex,
    bg_battle: Regex,
    bg_data: Regex,
    bg_effect: Regex,
    limit_msg: Regex,
    ex_files: Regex,
    certification_preset: Regex,
}

impl StageMatcher {
    pub fn new() -> Self {
        Self {
            map_data: Regex::new(patterns::MAP_STAGE_DATA_PATTERN).unwrap(),
            map_name: Regex::new(patterns::MAP_NAME_PATTERN).unwrap(),
            map_sn: Regex::new(patterns::MAP_SN_PATTERN).unwrap(),
            stage_normal: Regex::new(patterns::STAGE_NORMAL_PATTERN).unwrap(),
            stage_file: Regex::new(patterns::STAGE_FILE_PATTERN).unwrap(),
            stage_name: Regex::new(patterns::STAGE_NAME_PATTERN).unwrap(),
            legacy_stage_name: Regex::new(patterns::LEGACY_STAGE_NAME_PATTERN).unwrap(),
            castle: Regex::new(patterns::CASTLE_PATTERN).unwrap(),
            bg_map: Regex::new(patterns::BG_MAP_PATTERN).unwrap(),
            bg_battle: Regex::new(patterns::BG_BATTLE_PATTERN).unwrap(),
            bg_data: Regex::new(patterns::BG_DATA_PATTERN).unwrap(),
            bg_effect: Regex::new(patterns::BG_EFFECT_PATTERN).unwrap(),
            limit_msg: Regex::new(patterns::LIMIT_MSG_PATTERN).unwrap(),
            ex_files: Regex::new(patterns::EX_PATTERN).unwrap(),
            certification_preset: Regex::new(patterns::CERTIFICATION_PRESET_PATTERN).unwrap(),
        }
    }

    // The Algorithmic Rosetta Stone: Translates Stage Prefixes into Map Prefixes
    fn format_prefix(prefix: &str) -> String {
        let upper = prefix.to_uppercase();
        
        // 1. Keep specific casing for Space
        if upper == "SPACE" {
            return "Space".to_string();
        }
        
        // 2. The PONOS Madness Rule: Prune the leading 'R' if it's not a lone 'R'.
        if upper.starts_with('R') && upper.len() > 1 {
            return upper[1..].to_string();
        }

        // 3. Fallback for everything else
        upper
    }

    pub fn get_dest(&self, name: &str, stages_dir: &Path) -> Option<PathBuf> {
        // --- Exact Master File Matches ---
        match name {
            "bg.csv" => return Some(stages_dir.join("backgrounds").join("battle")),
            "Stage_option.csv" => return Some(stages_dir.to_path_buf()),
            "fixed_formation.csv" => return Some(stages_dir.join("fixedlineup")), 
            "stage.csv" => return Some(stages_dir.join("EC").join("data")),
            "SpecialRulesMap.json" | "SpecialRulesMapOption.json" => return Some(stages_dir.join("SR")),
            "tower_layout.csv" => return Some(stages_dir.join("V")), 
            "stage_conditions.csv" => return Some(stages_dir.join("L")),
            "stage_hint_popup.csv" => return Some(stages_dir.join("G")),
            _ => {} 
        }

        // --- Fixed Lineup Presets ---
        if self.certification_preset.is_match(name) {
            return Some(stages_dir.join("fixedlineup"));
        }

        // --- EX Specific Files ---
        if self.ex_files.is_match(name) {
            return Some(stages_dir.join("EX"));
        }

        // --- Map Stage Limits ---
        if self.limit_msg.is_match(name) {
            return Some(stages_dir.join("MapStageLimitMessage"));
        }

        // --- Master Stage Names (Text) ---
        if let Some(caps) = self.stage_name.captures(name) {
            return Some(stages_dir.join(Self::format_prefix(&caps[1])));
        }

        // --- Legacy Image Stage Names (ec048_n_en.png, wc015_n.png, sc018_n_en.png) ---
        if let Some(caps) = self.legacy_stage_name.captures(name) {
            let raw_prefix = caps[1].to_lowercase();
            let map_id = &caps[2];

            let mut mapped_prefix = match raw_prefix.as_str() {
                "wc" => "W",
                "sc" => "Space",
                _ => "EC" // Default for 'ec'
            };

            // Reroute specific 'ec' IDs to Challenge (CL) and Punt (PT)
            if raw_prefix == "ec" {
                if let Ok(id) = map_id.parse::<u32>() {
                    // The Moon stage image sharing offsets the image IDs by 2!
                    // Data 50 (Challenge) = Image 48
                    // Data 51-52 (Punt) = Image 49-50
                    if id == 48 { mapped_prefix = "CL"; }
                    if id >= 49 && id <= 50 { mapped_prefix = "PT"; }
                }
            }

            // Route all to their prefix, then into a lowercase "names" folder
            return Some(stages_dir.join(mapped_prefix).join("names"));
        }

        // --- Stage Normal (EC, ItF, CotC, and Zombies) ---
        if let Some(caps) = self.stage_normal.captures(name) {
            let chapter = &caps[1];
            let map_id = caps.get(2).map(|m| m.as_str());
            let is_zombie = name.ends_with("_Z.csv");

            let base_folder = if is_zombie {
                "Z".to_string()
            } else {
                match chapter {
                    "0" => "EC".to_string(),
                    "1" => "W".to_string(),
                    "2" => "Space".to_string(),
                    _ => format!("Normal_{}", chapter),
                }
            };

            let mut path = stages_dir.join(base_folder);
            if is_zombie { path = path.join(chapter); }
            if let Some(id) = map_id { path = path.join(id); }

            return Some(path);
        }

        // --- Individual Stage Data (DEEP NESTING) ---
        if let Some(caps) = self.stage_file.captures(name) {
            let prefix = caps.get(1).map(|m| m.as_str());
            let map_id = &caps[2];
            let stage_id = caps.get(3).map(|m| m.as_str()); 

            if let Some(p) = prefix {
                let mut path = stages_dir.join(Self::format_prefix(p)).join(map_id);
                if let Some(s) = stage_id { path = path.join(s); }
                return Some(path);
            } else {
                // Legacy Routing puts them in a lowercase "data" folder
                if let Ok(id) = map_id.parse::<u32>() {
                    if id <= 49 { return Some(stages_dir.join("EC").join("data")); }
                    if id == 50 { return Some(stages_dir.join("CL").join("data")); }
                    if id >= 51 && id <= 52 { return Some(stages_dir.join("PT").join("data")); }
                }
            }
        }

        // --- Dynamic Categories (Maps & Data) ---
        if let Some(caps) = self.map_data.captures(name) {
            return Some(stages_dir.join(Self::format_prefix(&caps[1])).join(&caps[2]));
        }
        if let Some(caps) = self.map_name.captures(name) {
            return Some(stages_dir.join(Self::format_prefix(&caps[2])).join(&caps[1]));
        }
        if let Some(caps) = self.map_sn.captures(name) {
            let map_id = &caps[1];
            let stage_id = &caps[2];
            let prefix = &caps[3];
            
            return Some(stages_dir.join(Self::format_prefix(prefix)).join(map_id).join(stage_id));
        }

        // --- Castles ---
        if let Some(caps) = self.castle.captures(name) {
            if name.starts_with("fc000") { return None; } 
            return Some(stages_dir.join("castles").join(&caps[1]));
        }

        // --- Backgrounds ---
        if let Some(caps) = self.bg_map.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                return Some(stages_dir.join("backgrounds").join("maps").join(format!("{:03}", id)));
            }
        }
        if let Some(caps) = self.bg_battle.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                return Some(stages_dir.join("backgrounds").join("battle").join(format!("{:03}", id)));
            }
        }
        if let Some(caps) = self.bg_effect.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                return Some(stages_dir.join("backgrounds").join("effects").join(format!("{:03}", id)));
            }
        }
        if self.bg_data.is_match(name) {
            return Some(stages_dir.join("backgrounds").join("effects").join("data"));
        }

        None
    }
}