use std::path::{Path, PathBuf};

use regex::Regex;

use crate::stage::patterns;

pub struct StageMatcher {
    map_data: Regex,
    map_name: Regex,
    map_sn: Regex,
    map_global: Regex,
    stage_normal: Regex,
    stage_file: Regex,
    stage_name: Regex,
    stage_numeric: Regex,
    stage_base: Regex,
    legacy_stage: Regex,
    castle: Regex,
    bg_map: Regex,
    bg_battle: Regex,
    bg_data: Regex,
    bg_effect: Regex,
    limit_msg: Regex,
    ex_files: Regex,
    preset: Regex,
    drop_item: Regex,
    charagroup: Regex,
    score_bonus: Regex,
    difficulty: Regex,
    drop_chara: Regex,
    lock_skip: Regex,
    scat_cpu: Regex,
    first_msg: Regex,
}

impl Default for StageMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl StageMatcher {
    pub fn new() -> Self {
        Self {
            map_data: Regex::new(patterns::MAP_STAGE_DATA_PATTERN).unwrap(),
            map_name: Regex::new(patterns::MAP_NAME_PATTERN).unwrap(),
            map_sn: Regex::new(patterns::MAP_SN_PATTERN).unwrap(),
            map_global: Regex::new(patterns::MAP_GLOBAL_NAME_PATTERN).unwrap(),
            stage_normal: Regex::new(patterns::STAGE_NORMAL_PATTERN).unwrap(),
            stage_file: Regex::new(patterns::STAGE_FILE_PATTERN).unwrap(),
            stage_name: Regex::new(patterns::STAGE_NAME_PATTERN).unwrap(),
            stage_numeric: Regex::new(patterns::STAGE_NAME_NUMERIC_PATTERN).unwrap(),
            stage_base: Regex::new(patterns::STAGE_NAME_BASE_PATTERN).unwrap(),
            legacy_stage: Regex::new(patterns::LEGACY_STAGE_NAME_PATTERN).unwrap(),
            castle: Regex::new(patterns::CASTLE_PATTERN).unwrap(),
            bg_map: Regex::new(patterns::BG_MAP_PATTERN).unwrap(),
            bg_battle: Regex::new(patterns::BG_BATTLE_PATTERN).unwrap(),
            bg_data: Regex::new(patterns::BG_DATA_PATTERN).unwrap(),
            bg_effect: Regex::new(patterns::BG_EFFECT_PATTERN).unwrap(),
            limit_msg: Regex::new(patterns::LIMIT_MSG_PATTERN).unwrap(),
            ex_files: Regex::new(patterns::EX_PATTERN).unwrap(),
            preset: Regex::new(patterns::CERTIFICATION_PRESET_PATTERN).unwrap(),
            drop_item: Regex::new(patterns::DROP_ITEM_PATTERN).unwrap(),
            charagroup: Regex::new(patterns::CHARAGROUP_PATTERN).unwrap(),
            score_bonus: Regex::new(patterns::SCORE_BONUS_PATTERN).unwrap(),
            difficulty: Regex::new(patterns::DIFFICULTY_LEVEL_PATTERN).unwrap(),
            drop_chara: Regex::new(patterns::DROP_CHARA_PATTERN).unwrap(),
            lock_skip: Regex::new(patterns::LOCK_SKIP_DATA_PATTERN).unwrap(),
            scat_cpu: Regex::new(patterns::SCAT_CPU_SETTING_PATTERN).unwrap(),
            first_msg: Regex::new(patterns::STAGE_FIRST_MESSAGE_PATTERN).unwrap(),
        }
    }

    fn format_prefix(prefix: &str) -> String {
        let upper = prefix.to_uppercase();
        if upper == "SPACE" { return "Space".to_string(); }
        if upper.starts_with('R') && upper.len() > 1 { return upper[1..].to_string(); }
        upper
    }

    pub fn get_dest(&self, target_file: &str, base_dir: &Path) -> Option<PathBuf> {
        let cat_dir = base_dir.join("categories");

        if self.map_global.is_match(target_file) {
            return Some(base_dir.join("Map_Name"));
        }

        match target_file {
            "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" |
            "DropItem.csv" | "Charagroup.csv" | "drop_chara.csv" |
            "LockSkipData.csv" | "ScatCPUsetting.csv" | "StageClearBonus_setting.tsv" => return Some(base_dir.to_path_buf()),
            "ScoreBonusMap.json" => return Some(cat_dir.join("R")),
            "SpecialRulesMap.json" | "SpecialRulesMapOption.json" => return Some(cat_dir.join("SR")),
            "bg.csv" => return Some(base_dir.join("backgrounds").join("battle")),
            "fixed_formation.csv" => return Some(base_dir.join("fixedlineup")),
            "stage.csv" => return Some(cat_dir.join("EC").join("000")),
            "tower_layout.csv" => return Some(cat_dir.join("V")),
            "stage_conditions.csv" => return Some(cat_dir.join("L")),
            "stage_hint_popup.csv" => return Some(cat_dir.join("G")),
            _ => {}
        }

        // Regex matches
        if self.drop_item.is_match(target_file) { return Some(base_dir.to_path_buf()); }
        if self.charagroup.is_match(target_file) { return Some(base_dir.to_path_buf()); }
        if self.difficulty.is_match(target_file) { return Some(base_dir.to_path_buf()); }
        if self.drop_chara.is_match(target_file) { return Some(base_dir.to_path_buf()); }
        if self.lock_skip.is_match(target_file) { return Some(base_dir.to_path_buf()); }
        if self.scat_cpu.is_match(target_file) { return Some(base_dir.to_path_buf()); }

        if self.score_bonus.is_match(target_file) { return Some(cat_dir.join("R")); }
        if self.preset.is_match(target_file) { return Some(base_dir.join("fixedlineup")); }
        if self.ex_files.is_match(target_file) { return Some(cat_dir.join("EX")); }
        if self.limit_msg.is_match(target_file) { return Some(base_dir.join("MapStageLimitMessage")); }
        if self.first_msg.is_match(target_file) { return Some(base_dir.join("StageFirstMessage")); }

        // StageName{N}_{cc}.csv
        if let Some(caps) = self.stage_numeric.captures(target_file) {
            let num_match = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            return match num_match {
                "0" => Some(cat_dir.join("EC").join("StageName0")),
                "1" => Some(cat_dir.join("W").join("StageName1")),
                "2" => Some(cat_dir.join("Space").join("StageName2")),
                _ => None,
            };
        }

        // StageName_{cc}.csv
        if self.stage_base.is_match(target_file) {
            return Some(cat_dir.join("EC").join("StageName"));
        }

        // StageName_{Category}_{cc}.csv
        if let Some(caps) = self.stage_name.captures(target_file) {
            return Some(cat_dir.join(Self::format_prefix(&caps[1])).join("StageName"));
        }

        // Legacy Images (Forced to Category/000/StageID)
        if let Some(caps) = self.legacy_stage.captures(target_file) {
            let raw_prefix = caps[1].to_lowercase();
            let mut mapped_prefix = match raw_prefix.as_str() {
                "wc" => "W", "sc" => "Space", _ => "EC"
            };

            let Ok(parsed_stage) = caps[2].parse::<u32>() else { return None; };
            let mut target_folder = parsed_stage;

            if raw_prefix == "ec" {
                if parsed_stage == 48 { mapped_prefix = "M"; }
                if (49..=50).contains(&parsed_stage) {
                    mapped_prefix = "PT";
                    target_folder = parsed_stage + 2;
                }
            }

            return Some(cat_dir.join(mapped_prefix).join("000").join(format!("{:02}", target_folder)));
        }

        // Stage Normal (EoC, ItF, CotC, and Zombies)
        if let Some(caps) = self.stage_normal.captures(target_file) {
            let chapter_str = &caps[1];
            let sub_chapter = caps.get(2).map(|m| m.as_str()).unwrap_or("0");
            let is_zombie = target_file.ends_with("_Z.csv");

            let category_str = if is_zombie {
                "Z".to_string()
            } else {
                match chapter_str {
                    "0" => "EC".to_string(),
                    "1" => "W".to_string(),
                    "2" => "Space".to_string(),
                    _ => format!("Normal_{}", chapter_str),
                }
            };

            let map_id = match (chapter_str, sub_chapter) {
                ("0", _) => "000",
                ("1", "0") => "004",
                ("1", "1") => "005",
                ("1", "2") => "006",
                ("2", "0") => "007",
                ("2", "1") => "008",
                ("2", "2") => "009",
                _ => "000",
            };

            return Some(cat_dir.join(category_str).join(map_id));
        }

        // Stage Files (Unified to Category/Map/Stage)
        if let Some(caps) = self.stage_file.captures(target_file) {
            let cap_prefix = caps.get(1).map(|m| m.as_str());
            let Ok(parsed_map) = caps[2].parse::<u32>() else { return None; };

            if let Some(valid_prefix) = cap_prefix {
                let mut path = cat_dir.join(Self::format_prefix(valid_prefix)).join(format!("{:03}", parsed_map));

                if let Some(stage_cap) = caps.get(3)
                    && let Ok(parsed_stage) = stage_cap.as_str().parse::<u32>() {
                    path = path.join(format!("{:02}", parsed_stage));
                }
                return Some(path);
            } else {
                let mut fallback_prefix = "EC";
                let target_folder = parsed_map;

                if parsed_map == 48 { fallback_prefix = "M"; }
                if (51..=52).contains(&parsed_map) { fallback_prefix = "PT"; }

                return Some(cat_dir.join(fallback_prefix).join("000").join(format!("{:02}", target_folder)));
            }
        }

        // Map & Stage Dynamic Content
        if let Some(caps) = self.map_data.captures(target_file) {
            let Ok(parsed_map) = caps[2].parse::<u32>() else { return None; };
            return Some(cat_dir.join(Self::format_prefix(&caps[1])).join(format!("{:03}", parsed_map)));
        }

        if let Some(caps) = self.map_name.captures(target_file) {
            let Ok(parsed_map) = caps[1].parse::<u32>() else { return None; };
            return Some(cat_dir.join(Self::format_prefix(&caps[2])).join(format!("{:03}", parsed_map)));
        }

        if let Some(caps) = self.map_sn.captures(target_file) {
            let Ok(parsed_map) = caps[1].parse::<u32>() else { return None; };
            let Ok(parsed_stage) = caps[2].parse::<u32>() else { return None; };
            return Some(cat_dir.join(Self::format_prefix(&caps[3])).join(format!("{:03}", parsed_map)).join(format!("{:02}", parsed_stage)));
        }

        // Assorted Assets
        if let Some(caps) = self.castle.captures(target_file) {
            if target_file.starts_with("fc000") { return None; }
            return Some(base_dir.join("castles").join(&caps[1]));
        }

        if let Some(caps) = self.bg_map.captures(target_file)
            && let Ok(parsed_id) = caps[1].parse::<u32>() {
            return Some(base_dir.join("backgrounds").join("maps").join(format!("{:03}", parsed_id)));
        }

        if let Some(caps) = self.bg_battle.captures(target_file)
            && let Ok(parsed_id) = caps[1].parse::<u32>() {
            return Some(base_dir.join("backgrounds").join("battle").join(format!("{:03}", parsed_id)));
        }

        if let Some(caps) = self.bg_effect.captures(target_file)
            && let Ok(parsed_id) = caps[1].parse::<u32>() {
            return Some(base_dir.join("backgrounds").join("effects").join(format!("{:03}", parsed_id)));
        }

        if self.bg_data.is_match(target_file) {
            return Some(base_dir.join("backgrounds").join("effects").join("data"));
        }

        None
    }
}