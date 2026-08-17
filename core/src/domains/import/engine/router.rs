use std::path::{Path, PathBuf};

use regex::Regex;

use crate::common::io as global_patterns;
use crate::domains::cat::patterns as cat_patterns;
use crate::domains::enemy::patterns as enemy_patterns;
use crate::domains::settings::ImportStructure;
use crate::domains::stage::patterns as stage_patterns;

struct CatPatternsSet {
    universal: Regex,
    skill_desc: Regex,
    skill_name: Regex,
    stats: Regex,
    icon: Regex,
    upgrade: Regex,
    gacha: Regex,
    anim: Regex,
    maanim: Regex,
    explain: Regex,
    egg_icon: Regex,
    egg_upgrade: Regex,
    egg_gacha: Regex,
    egg_anim: Regex,
    egg_maanim: Regex,
}

impl CatPatternsSet {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            universal: Regex::new(cat_patterns::CAT_EVOLVE_PATTERN)?,
            skill_desc: Regex::new(cat_patterns::SKILL_DESC_PATTERN)?,
            skill_name: Regex::new(cat_patterns::SKILL_NAME_PATTERN)?,
            stats: Regex::new(cat_patterns::CAT_STATS_PATTERN)?,
            icon: Regex::new(cat_patterns::CAT_ICON_PATTERN)?,
            upgrade: Regex::new(cat_patterns::CAT_UPGRADE_PATTERN)?,
            gacha: Regex::new(cat_patterns::CAT_GACHA_PATTERN)?,
            anim: Regex::new(cat_patterns::CAT_ANIM_PATTERN)?,
            maanim: Regex::new(cat_patterns::CAT_MAANIM_PATTERN)?,
            explain: Regex::new(cat_patterns::CAT_EXPLAIN_PATTERN)?,
            egg_icon: Regex::new(cat_patterns::EGG_ICON_PATTERN)?,
            egg_upgrade: Regex::new(cat_patterns::EGG_UPGRADE_PATTERN)?,
            egg_gacha: Regex::new(cat_patterns::EGG_GACHA_PATTERN)?,
            egg_anim: Regex::new(cat_patterns::EGG_ANIM_PATTERN)?,
            egg_maanim: Regex::new(cat_patterns::EGG_MAANIM_PATTERN)?,
        })
    }

    fn map_egg(code: &str) -> &'static str {
        match code {
            "m00" => "f",
            "m01" => "c",
            "m02" => "s",
            _ => "c",
        }
    }

    fn get_dest(&self, name: &str, cats_dir: &Path) -> Option<PathBuf> {
        if self.skill_desc.is_match(name) {
            return Some(cats_dir.join("SkillDescriptions"));
        }
        if self.skill_name.is_match(name) {
            return Some(cats_dir.join("Skill_name"));
        }
        if self.universal.is_match(name) {
            return Some(cats_dir.join("unitevolve"));
        }

        if let Some(caps) = self.stats.captures(name)
            && let Ok(id) = caps[1].parse::<u32>()
            && id > 0 {
            return Some(cats_dir.join(format!("{:03}", id - 1)));
        }
        if let Some(caps) = self.icon.captures(name) {
            return Some(cats_dir.join(&caps[1]).join(&caps[2]));
        }
        if let Some(caps) = self.upgrade.captures(name) {
            return Some(cats_dir.join(&caps[1]).join(&caps[2]));
        }
        if let Some(caps) = self.gacha.captures(name) {
            return Some(cats_dir.join(&caps[1]));
        }
        if let Some(caps) = self.anim.captures(name) {
            return Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim"));
        }
        if let Some(caps) = self.maanim.captures(name) {
            return Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim"));
        }
        if let Some(caps) = self.explain.captures(name)
            && let Ok(id) = caps[1].parse::<u32>()
            && id > 0 {
            return Some(cats_dir.join(format!("{:03}", id - 1)).join("lang"));
        }

        if let Some(caps) = self.egg_icon.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join(Self::map_egg(&caps[2])));
        }
        if let Some(caps) = self.egg_upgrade.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join(Self::map_egg(&caps[2])));
        }
        if let Some(caps) = self.egg_gacha.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])));
        }
        if let Some(caps) = self.egg_anim.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        if let Some(caps) = self.egg_maanim.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }

        None
    }
}

struct EnemyPatternsSet {
    stats: Regex,
    icon: Regex,
    anim_base: Regex,
    maanim: Regex,
    name: Regex,
    pic_book: Regex,
    pic_book_2: Regex,
    pic_book_q: Regex,
    dict_list: Regex,
    autoset_exclude: Regex,
    zombie_effect: Regex,
}

impl EnemyPatternsSet {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            stats: Regex::new(enemy_patterns::ENEMY_STATS)?,
            icon: Regex::new(enemy_patterns::ENEMY_ICON)?,
            anim_base: Regex::new(enemy_patterns::ENEMY_ANIM_BASE)?,
            maanim: Regex::new(enemy_patterns::ENEMY_MAANIM)?,
            name: Regex::new(enemy_patterns::ENEMY_NAME)?,
            pic_book: Regex::new(enemy_patterns::ENEMY_PICTURE_BOOK)?,
            pic_book_2: Regex::new(enemy_patterns::ENEMY_PICTURE_BOOK_2)?,
            pic_book_q: Regex::new(enemy_patterns::ENEMY_PICTURE_BOOK_QUESTION)?,
            dict_list: Regex::new(enemy_patterns::ENEMY_DICT_LIST)?,
            autoset_exclude: Regex::new(enemy_patterns::AUTOSET_EXCLUDE)?,
            zombie_effect: Regex::new(enemy_patterns::ENEMY_ZOMBIE_EFFECT)?,
        })
    }

    fn get_dest(&self, name: &str, enemy_dir: &Path) -> Option<PathBuf> {
        if self.stats.is_match(name) || self.dict_list.is_match(name) || self.autoset_exclude.is_match(name) {
            return Some(enemy_dir.to_path_buf());
        }

        if let Some(caps) = self.icon.captures(name) {
            return Some(enemy_dir.join(&caps[1]));
        }
        if let Some(caps) = self.anim_base.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }
        if let Some(caps) = self.maanim.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }

        if self.name.is_match(name) {
            return Some(enemy_dir.join("Enemyname"));
        }
        if self.pic_book.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBook"));
        }
        if self.pic_book_2.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBook2"));
        }
        if self.pic_book_q.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBookQuestion"));
        }

        if self.zombie_effect.is_match(name) {
            return Some(enemy_dir.join("zombie"));
        }

        None
    }
}

struct GlobalPatternsSet {
    gatya_item_d: Regex,
    gatya_item_buy: Regex,
    gatya_item_name: Regex,
    img015: Regex,
    img015_cut: Regex,
    img022: Regex,
    img022_cut: Regex,
    localizable: Regex,
    param: Regex,
    audio_ogg: Regex,
    audio_caf: Regex,
}

impl GlobalPatternsSet {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            gatya_item_d: Regex::new(global_patterns::GATYA_ITEM_D_PATTERN)?,
            gatya_item_buy: Regex::new(global_patterns::GATYA_ITEM_BUY_PATTERN)?,
            gatya_item_name: Regex::new(global_patterns::GATYA_ITEM_NAME_PATTERN)?,
            img015: Regex::new(global_patterns::ASSET_IMG015_PATTERN)?,
            img015_cut: Regex::new(global_patterns::ASSET_015CUT_PATTERN)?,
            img022: Regex::new(global_patterns::ASSET_IMG022_PATTERN)?,
            img022_cut: Regex::new(global_patterns::ASSET_022CUT_PATTERN)?,
            localizable: Regex::new(global_patterns::LOCALIZEABLE_PATTERN)?,
            param: Regex::new(global_patterns::PARAM_PATTERN)?,
            audio_ogg: Regex::new(global_patterns::AUDIO_OGG_PATTERN)?,
            audio_caf: Regex::new(global_patterns::AUDIO_CAF_PATTERN)?,
        })
    }

    fn get_dest(&self, name: &str, sheets_dir: &Path, ui_dir: &Path, tables_dir: &Path, audio_dir: &Path) -> Option<PathBuf> {
        if self.param.is_match(name) || self.gatya_item_buy.is_match(name) {
            return Some(tables_dir.to_path_buf());
        }
        if self.localizable.is_match(name) {
            return Some(tables_dir.join("localizable"));
        }
        if self.gatya_item_name.is_match(name) {
            return Some(tables_dir.join("GatyaitemName"));
        }

        if self.gatya_item_d.is_match(name) {
            return Some(ui_dir.join("gatyaitemD"));
        }

        if self.img015.is_match(name) || self.img015_cut.is_match(name) {
            return Some(sheets_dir.join("img015"));
        }
        if self.img022.is_match(name) || self.img022_cut.is_match(name) {
            return Some(sheets_dir.join("img022"));
        }

        if self.audio_ogg.is_match(name) {
            return Some(audio_dir.join("ogg"));
        }
        if self.audio_caf.is_match(name) {
            return Some(audio_dir.join("caf"));
        }

        None
    }
}

struct StagePatternsSet {
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

impl StagePatternsSet {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            map_data: Regex::new(stage_patterns::MAP_STAGE_DATA_PATTERN)?,
            map_name: Regex::new(stage_patterns::MAP_NAME_PATTERN)?,
            map_sn: Regex::new(stage_patterns::MAP_SN_PATTERN)?,
            map_global: Regex::new(stage_patterns::MAP_GLOBAL_NAME_PATTERN)?,
            stage_normal: Regex::new(stage_patterns::STAGE_NORMAL_PATTERN)?,
            stage_file: Regex::new(stage_patterns::STAGE_FILE_PATTERN)?,
            stage_name: Regex::new(stage_patterns::STAGE_NAME_PATTERN)?,
            stage_numeric: Regex::new(stage_patterns::STAGE_NAME_NUMERIC_PATTERN)?,
            stage_base: Regex::new(stage_patterns::STAGE_NAME_BASE_PATTERN)?,
            legacy_stage: Regex::new(stage_patterns::LEGACY_STAGE_NAME_PATTERN)?,
            castle: Regex::new(stage_patterns::CASTLE_PATTERN)?,
            bg_map: Regex::new(stage_patterns::BG_MAP_PATTERN)?,
            bg_battle: Regex::new(stage_patterns::BG_BATTLE_PATTERN)?,
            bg_data: Regex::new(stage_patterns::BG_DATA_PATTERN)?,
            bg_effect: Regex::new(stage_patterns::BG_EFFECT_PATTERN)?,
            limit_msg: Regex::new(stage_patterns::LIMIT_MSG_PATTERN)?,
            ex_files: Regex::new(stage_patterns::EX_PATTERN)?,
            preset: Regex::new(stage_patterns::CERTIFICATION_PRESET_PATTERN)?,
            drop_item: Regex::new(stage_patterns::DROP_ITEM_PATTERN)?,
            charagroup: Regex::new(stage_patterns::CHARAGROUP_PATTERN)?,
            score_bonus: Regex::new(stage_patterns::SCORE_BONUS_PATTERN)?,
            difficulty: Regex::new(stage_patterns::DIFFICULTY_LEVEL_PATTERN)?,
            drop_chara: Regex::new(stage_patterns::DROP_CHARA_PATTERN)?,
            lock_skip: Regex::new(stage_patterns::LOCK_SKIP_DATA_PATTERN)?,
            scat_cpu: Regex::new(stage_patterns::SCAT_CPU_SETTING_PATTERN)?,
            first_msg: Regex::new(stage_patterns::STAGE_FIRST_MESSAGE_PATTERN)?,
        })
    }

    fn format_prefix(prefix: &str) -> String {
        let upper = prefix.to_uppercase();
        if upper == "SPACE" {
            return "Space".to_string();
        }
        if upper.starts_with('R') && upper.len() > 1 {
            return upper[1..].to_string();
        }
        upper
    }

    fn get_dest(&self, target_file: &str, base_dir: &Path) -> Option<PathBuf> {
        let cat_dir = base_dir.join("categories");

        if self.map_global.is_match(target_file) {
            return Some(base_dir.join("Map_Name"));
        }

        match target_file {
            "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" | "DropItem.csv"
            | "Charagroup.csv" | "drop_chara.csv" | "LockSkipData.csv" | "ScatCPUsetting.csv"
            | "StageClearBonus_setting.tsv" => return Some(base_dir.to_path_buf()),

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

        if let Some(caps) = self.stage_numeric.captures(target_file) {
            let num_match = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            return match num_match {
                "0" => Some(cat_dir.join("EC").join("StageName0")),
                "1" => Some(cat_dir.join("W").join("StageName1")),
                "2" => Some(cat_dir.join("Space").join("StageName2")),
                _ => None,
            };
        }

        if self.stage_base.is_match(target_file) {
            return Some(cat_dir.join("EC").join("StageName"));
        }

        if let Some(caps) = self.stage_name.captures(target_file) {
            return Some(cat_dir.join(Self::format_prefix(&caps[1])).join("StageName"));
        }

        if let Some(caps) = self.legacy_stage.captures(target_file) {
            let raw_prefix = caps[1].to_lowercase();
            let mut mapped_prefix = match raw_prefix.as_str() {
                "wc" => "W",
                "sc" => "Space",
                _ => "EC",
            };

            let Ok(parsed_stage) = caps[2].parse::<u32>() else {
                return None;
            };

            let mut target_folder = parsed_stage;

            if raw_prefix == "ec" {
                if parsed_stage == 48 {
                    mapped_prefix = "M";
                }
                if (49..=50).contains(&parsed_stage) {
                    mapped_prefix = "PT";
                    target_folder = parsed_stage + 2;
                }
            }

            return Some(cat_dir.join(mapped_prefix).join("image").join(format!("{:02}", target_folder)));
        }

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
                ("0", "1") => "001",
                ("0", "2") => "002",
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

        if let Some(caps) = self.stage_file.captures(target_file) {
            let cap_prefix = caps.get(1).map(|m| m.as_str());
            let Ok(parsed_map) = caps[2].parse::<u32>() else {
                return None;
            };

            let is_invasion = target_file.contains("_Invasion");
            let is_z = target_file.contains("_Z");

            if let Some(valid_prefix) = cap_prefix {
                let category_folder = if is_z {
                    "Z".to_string()
                } else {
                    Self::format_prefix(valid_prefix)
                };

                let mut path = cat_dir.join(category_folder).join(format!("{:03}", parsed_map));

                if is_invasion {
                    path = path.join("Invasion");
                } else if let Some(stage_cap) = caps.get(3)
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

        if let Some(caps) = self.map_data.captures(target_file)
            && let Ok(parsed_map) = caps[2].parse::<u32>() {
            return Some(cat_dir.join(Self::format_prefix(&caps[1])).join(format!("{:03}", parsed_map)));
        }

        if let Some(caps) = self.map_name.captures(target_file)
            && let Ok(parsed_map) = caps[1].parse::<u32>() {
            return Some(cat_dir.join(Self::format_prefix(&caps[2])).join(format!("{:03}", parsed_map)));
        }

        if let Some(caps) = self.map_sn.captures(target_file)
            && let Ok(parsed_map) = caps[1].parse::<u32>()
            && let Ok(parsed_stage) = caps[2].parse::<u32>() {
            return Some(cat_dir.join(Self::format_prefix(&caps[3])).join(format!("{:03}", parsed_map)).join(format!("{:02}", parsed_stage)));
        }

        if let Some(caps) = self.castle.captures(target_file) {
            if target_file.starts_with("fc000") {
                return None;
            }
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

pub(crate) struct AssetRouter {
    structure: ImportStructure,

    cat_matcher: CatPatternsSet,
    enemy_matcher: EnemyPatternsSet,
    global_matcher: GlobalPatternsSet,
    stage_matcher: StagePatternsSet,

    game_dir: PathBuf,
    cats_dir: PathBuf,
    sheets_dir: PathBuf,
    ui_dir: PathBuf,
    tables_dir: PathBuf,
    enemy_dir: PathBuf,
    stages_dir: PathBuf,
    raw_dir: PathBuf,
    audio_dir: PathBuf,
}

impl AssetRouter {
    pub(crate) fn new(game_root: &Path, structure: ImportStructure) -> Result<Self, regex::Error> {
        Ok(Self {
            structure,

            cat_matcher: CatPatternsSet::new()?,
            enemy_matcher: EnemyPatternsSet::new()?,
            global_matcher: GlobalPatternsSet::new()?,
            stage_matcher: StagePatternsSet::new()?,

            game_dir: game_root.to_path_buf(),
            cats_dir: game_root.join("cats"),
            sheets_dir: game_root.join("sheets"),
            ui_dir: game_root.join("ui"),
            tables_dir: game_root.join("tables"),
            enemy_dir: game_root.join("enemies"),
            stages_dir: game_root.join("stages"),
            raw_dir: game_root.join("raw"),
            audio_dir: game_root.join("audio"),
        })
    }

    fn clean_base_name(stem: &str, ext: &str) -> String {
        for &(code, _) in global_patterns::APP_LANGUAGES {
            let suffix = format!("_{}", code);
            if stem.len() > suffix.len() && stem.ends_with(&suffix) {
                let clean_stem = &stem[..stem.len() - suffix.len()];
                return if ext.is_empty() {
                    clean_stem.to_string()
                } else {
                    format!("{}.{}", clean_stem, ext)
                };
            }
        }

        if ext.is_empty() {
            stem.to_string()
        } else {
            format!("{}.{}", stem, ext)
        }
    }

    fn is_cat_base_banner(name: &str, clean_name: &str) -> bool {
        if !name.starts_with("udi") || name.len() < 6 {
            return false;
        }

        let Ok(id) = name[3..6].parse::<u32>() else {
            return false;
        };

        if id > 9 {
            return false;
        }

        name != clean_name
    }

    pub(crate) fn resolve_destination(&self, original_name: &str, final_name: &str) -> PathBuf {
        if self.structure == ImportStructure::Flat {
            return self.game_dir.join(final_name);
        }

        let path = Path::new(original_name);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let base_name = Self::clean_base_name(&stem, &ext);

        if Self::is_cat_base_banner(original_name, &base_name) {
            return self.cats_dir.join("CatBase").join(final_name);
        }

        if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            return self.cats_dir.join(final_name);
        }

        let routed_folder = self.global_matcher
            .get_dest(&base_name, &self.sheets_dir, &self.ui_dir, &self.tables_dir, &self.audio_dir)
            .or_else(|| self.cat_matcher.get_dest(&base_name, &self.cats_dir))
            .or_else(|| self.enemy_matcher.get_dest(&base_name, &self.enemy_dir))
            .or_else(|| self.stage_matcher.get_dest(&base_name, &self.stages_dir));

        routed_folder.map_or_else(|| self.raw_dir.join(final_name), |folder| folder.join(final_name))
    }
}