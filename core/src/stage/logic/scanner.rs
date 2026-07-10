// TODO: split into "load story" function and "load legend" function
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use nyanko::common::utils::csv;
use tracing::{instrument, warn};

use crate::settings::logic::state::ScannerConfig;
use crate::stage::data;
use crate::stage::paths;
use crate::stage::registry::{Map, Stage, StageRegistry};

use super::xp::get_hardcoded_xp;

pub struct ScanContext<'a> {
    pub lang_priority: &'a [String],
    pub map_names: HashMap<u32, String>,
    pub map_options: HashMap<u32, data::map_option::MapOption>,
    pub stage_options: HashMap<u32, Vec<data::stage_option::StageOption>>,
    pub charagroups: HashMap<u32, data::charagroup::CharaGroup>,
    pub drop_items: HashMap<u32, data::dropitem::DropItem>,
    pub score_bonuses: HashMap<u32, data::scorebonusmap::ScoreBonus>,
    pub special_rules: HashMap<u32, data::specialrulesmap::SpecialRule>,
    pub special_rule_options: HashMap<u8, data::specialrulesmapoption::SpecialRuleOption>,
    pub ex_options: HashMap<u32, u32>,
    pub difficulties: HashMap<u32, Vec<u16>>,
}

#[instrument(skip(config))]
pub fn start_scan(config: &ScannerConfig) -> Receiver<StageRegistry> {
    let (tx_channel, rx_channel) = mpsc::channel();
    let lang_priority_clone = config.language_priority.clone();

    thread::spawn(move || {
        let registry = scan_all(&lang_priority_clone);
        let _ = tx_channel.send(registry);
    });

    rx_channel
}

#[instrument(skip(lang_priority))]
fn scan_all(lang_priority: &[String]) -> StageRegistry {
    let mut registry = StageRegistry::default();
    let root_path = Path::new(paths::DIR_STAGES);

    let ctx = ScanContext {
        lang_priority,
        map_names: data::map_name::load(&root_path.join("Map_Name"), "Map_Name.csv", lang_priority),
        map_options: data::map_option::load(root_path, "Map_option.csv", lang_priority),
        stage_options: data::stage_option::load(root_path, "Stage_option.csv", lang_priority),
        charagroups: data::charagroup::load(root_path, "Charagroup.csv", lang_priority),
        drop_items: data::dropitem::load(root_path, "DropItem.csv", lang_priority),
        score_bonuses: data::scorebonusmap::load(&root_path.join("R"), "ScoreBonusMap.json", lang_priority),
        special_rules: data::specialrulesmap::load(&root_path.join("SR"), "SpecialRulesMap.json", lang_priority),
        special_rule_options: data::specialrulesmapoption::load(&root_path.join("SR"), "SpecialRulesMapOption.json", lang_priority),
        ex_options: data::ex_option::load(root_path, "EX_option.csv", lang_priority),
        difficulties: data::difficulty_level::load(root_path, "difficulty_level.tsv", lang_priority),
    };

    let Ok(categories_dir) = fs::read_dir(root_path) else {
        warn!("Failed to read root stages directory");
        return registry;
    };

    for category_entry in categories_dir.flatten() {
        let cat_path = category_entry.path();
        let cat_name = cat_path.file_name().unwrap_or_default().to_string_lossy();

        let is_ignored_dir = matches!(
            cat_name.as_ref(),
            "backgrounds" | "castles" | "fixedlineup" | "MapStageLimitMessage" |
            "Map_Name" | "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" |
            "DropItem.csv" | "Charagroup.csv" | "EX_option.csv" | "difficulty_level.tsv"
        );

        if is_ignored_dir || !cat_path.is_dir() {
            continue;
        }

        scan_category(&mut registry, &cat_path, &ctx);
    }

    registry
}

#[instrument(skip(registry, ctx))]
fn scan_category(registry: &mut StageRegistry, cat_path: &Path, ctx: &ScanContext) {
    let cat_prefix = cat_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let cat_display_name = data::map_name::get_category_name(&cat_prefix);

    let mut stage_names = data::stagename::load(cat_path, &format!("StageName_{}.csv", cat_prefix), ctx.lang_priority);
    if stage_names.is_empty() {
        stage_names = data::stagename::load(cat_path, &format!("StageName_R{}.csv", cat_prefix), ctx.lang_priority);
    }

    let Ok(maps_dir) = fs::read_dir(cat_path) else {
        return;
    };

    for map_entry in maps_dir.flatten() {
        let map_path = map_entry.path();
        if !map_path.is_dir() {
            continue;
        }

        let map_folder_name = map_path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(map_id) = map_folder_name.parse::<u32>() else {
            continue;
        };

        let mut global_map_id = data::map_name::get_global_map_id(&cat_prefix, map_id);

        if global_map_id.is_none() || global_map_id == Some(map_id) {
            let routed_id = match (cat_prefix.as_str(), map_id) {
                ("EC", 0) => Some(3000),
                ("EC", 1) => Some(3001),
                ("EC", 2) => Some(3002),
                ("W", 4) => Some(3003),
                ("W", 5) => Some(3004),
                ("W", 6) => Some(3005),
                ("Space", 7) => Some(3006),
                ("Space", 8) => Some(3007),
                ("Space", 9) => Some(3008),
                _ => global_map_id,
            };
            if routed_id.is_some() && routed_id != global_map_id {
                global_map_id = routed_id;
            }
        }

        let map_display_name = global_map_id
            .and_then(|id| ctx.map_names.get(&id))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:03}", map_id));

        load_map(
            registry,
            &cat_prefix,
            map_id,
            &map_path,
            &map_display_name,
            &cat_display_name,
            &stage_names,
            ctx,
            global_map_id
        );
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
fn load_map(
    registry: &mut StageRegistry,
    cat_prefix: &str,
    map_id: u32,
    map_path: &Path,
    map_display_name: &str,
    cat_display_name: &str,
    stage_names: &HashMap<u32, Vec<String>>,
    ctx: &ScanContext,
    global_map_id: Option<u32>
) {
    let global_id_val = global_map_id.unwrap_or(0);
    let map_opt = ctx.map_options.get(&global_id_val).cloned().unwrap_or_default();

    // Map the special rules and resolve invalid combos
    let special_rules = ctx.special_rules.get(&global_id_val).cloned();
    let mut invalid_combos = Vec::new();

    if let Some(rule) = &special_rules {
        for target_rule in &rule.rules {
            let rule_id = match target_rule {
                data::specialrulesmap::RuleType::TrustFund(_) => 0,
                data::specialrulesmap::RuleType::CooldownEquality(_) => 1,
                data::specialrulesmap::RuleType::RarityLimit(_) => 3,
                data::specialrulesmap::RuleType::CheapLabor(_) => 4,
                data::specialrulesmap::RuleType::RestrictPrice(_) => 5,
                data::specialrulesmap::RuleType::RestrictCd(_) => 6,
                data::specialrulesmap::RuleType::DeployLimit(_) => 7,
                data::specialrulesmap::RuleType::AwesomeCatSpawn(_) => 8,
                data::specialrulesmap::RuleType::AwesomeCatCannon(_) => 9,
                data::specialrulesmap::RuleType::AwesomeUnitSpeed(_) => 10,
                data::specialrulesmap::RuleType::Unknown(id, _) => *id,
            };

            if let Some(opt) = ctx.special_rule_options.get(&rule_id) {
                invalid_combos.extend(&opt.invalid_combo_ids);
            }
        }

        invalid_combos.sort_unstable();
        invalid_combos.dedup();
    }

    let mut map_struct = Map {
        id: format!("{}_{}", cat_prefix, map_id),
        name: map_display_name.to_string(),
        category: cat_prefix.to_string(),
        category_name: cat_display_name.to_string(),
        map_id,
        stages: Vec::new(),
        max_crowns: map_opt.max_crowns,
        has_abyss: map_opt.has_abyss,
        crown_1_mag: map_opt.crown_1_mag,
        crown_2_mag: map_opt.crown_2_mag,
        crown_3_mag: map_opt.crown_3_mag,
        crown_4_mag: map_opt.crown_4_mag,
        reset_type: map_opt.reset_type,
        max_clears: map_opt.max_clears,
        cooldown_minutes: map_opt.cooldown_minutes,
        hidden_upon_clear: map_opt.hidden_upon_clear,
        comment: map_opt.comment,
        ex_invasion: ctx.ex_options.get(&global_id_val).cloned(),
        score_bonuses: ctx.score_bonuses.get(&global_id_val).cloned(),
        special_rules,
        invalid_combos,
        drop_items: ctx.drop_items.get(&global_id_val).cloned(),
    };

    let stage_opts = ctx.stage_options.get(&global_id_val).cloned().unwrap_or_default();

    let mut story_data = HashMap::new();
    let mut stage_data_entries = Vec::new();

    if (3000..=3008).contains(&global_id_val) {
        let story_file = match global_id_val {
            3000 | 3001 | 3002 => "stageNormal0.csv",
            3003 => "stageNormal1_0.csv",
            3004 => "stageNormal1_1.csv",
            3005 => "stageNormal1_2.csv",
            3006 => "stageNormal2_0.csv",
            3007 => "stageNormal2_1.csv",
            3008 => "stageNormal2_2.csv",
            _ => "",
        };

        if !story_file.is_empty() {
            let story_path = map_path.join(story_file);
            if let Ok(content) = fs::read_to_string(&story_path) {
                let sep = csv::detect_separator(&content);
                for (idx, line) in content.lines().skip(2).enumerate() {
                    let clean = line.split("//").next().unwrap_or("").trim();
                    if clean.is_empty() { continue; }

                    let parts: Vec<&str> = clean.split(sep).collect();
                    if parts.len() >= 6 {
                        let energy = parts[0].trim().parse().unwrap_or(0);
                        let init_track: i16 = parts[2].trim().parse().unwrap_or(0);
                        let boss_track: i16 = parts[5].trim().parse().unwrap_or(-1);

                        story_data.insert(idx as u32, (energy, init_track, boss_track));
                    }
                }
            }
        }
    } else {
        if let Ok(files_dir) = std::fs::read_dir(map_path) {
            for file_entry in files_dir.flatten() {
                let filename = file_entry.file_name().to_string_lossy().to_string();
                let is_valid_stage_data = filename.starts_with("MapStageData") && filename.ends_with(".csv");

                if !is_valid_stage_data {
                    continue;
                }

                stage_data_entries = data::mapstagedata::load(map_path, &filename, ctx.lang_priority);

                if !stage_data_entries.is_empty() {
                    break;
                }
            }
        }

        if stage_data_entries.is_empty() {
            stage_data_entries = data::mapstagedata::load(map_path, "stage.csv", ctx.lang_priority);
        }
    }

    let Ok(stages_dir) = std::fs::read_dir(map_path) else {
        return;
    };

    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() {
            continue;
        }

        let stage_folder = stage_path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(stage_id) = stage_folder.parse::<u32>() else {
            continue;
        };

        let mut stage_raw = None;
        if let Ok(files_dir) = std::fs::read_dir(&stage_path) {
            for file_entry in files_dir.flatten() {
                let filename = file_entry.file_name().to_string_lossy().to_string();

                if !filename.ends_with(".csv") {
                    continue;
                }

                stage_raw = data::stage::load(&stage_path, &filename, ctx.lang_priority);

                if stage_raw.is_some() {
                    break;
                }
            }
        }

        let Some(raw_layout) = stage_raw else {
            continue;
        };

        let stage_display_name = stage_names.get(&map_id)
            .and_then(|names_list| names_list.get(stage_id as usize))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id));

        let mut final_opt = data::stage_option::StageOption::default();
        final_opt.target_crowns = -1;

        let valid_options = stage_opts.iter().filter(|o|
            (o.target_stage == -1 || o.target_stage == stage_id as i32) &&
                (o.target_crowns == -1 || o.target_crowns == 0)
        );

        for opt in valid_options {
            if opt.target_crowns != -1 { final_opt.target_crowns = opt.target_crowns; }
            if opt.rarity_mask != 0 { final_opt.rarity_mask = opt.rarity_mask; }
            if opt.deploy_limit != 0 { final_opt.deploy_limit = opt.deploy_limit; }
            if opt.allowed_rows != 0 { final_opt.allowed_rows = opt.allowed_rows; }
            if opt.min_cost != 0 { final_opt.min_cost = opt.min_cost; }
            if opt.max_cost != 0 { final_opt.max_cost = opt.max_cost; }
            if opt.charagroup_id != 0 { final_opt.charagroup_id = opt.charagroup_id; }
        }

        let stage_diff = ctx.difficulties.get(&global_id_val).and_then(|diff_list| diff_list.get(stage_id as usize)).copied().unwrap_or(0);
        let current_charagroup = ctx.charagroups.get(&final_opt.charagroup_id).cloned();

        let stage_key = format!("{}_{}_{}", cat_prefix, map_id, stage_id);
        let mut stage_struct = Stage {
            id: stage_key.clone(),
            name: stage_display_name,
            category: cat_prefix.to_string(),
            category_name: cat_display_name.to_string(),
            map_id,
            stage_id,
            base_id: raw_layout.base_id,
            anim_base_id: raw_layout.anim_base_id,
            width: raw_layout.width,
            base_hp: raw_layout.base_hp,
            min_spawn: raw_layout.min_spawn,
            max_spawn: raw_layout.max_spawn,
            background_id: raw_layout.background_id,
            max_enemies: raw_layout.max_enemies,
            time_limit: raw_layout.time_limit,
            is_no_continues: raw_layout.is_no_continues,
            is_base_indestructible: raw_layout.is_base_indestructible,
            unknown_value: raw_layout.unknown_value,
            enemies: raw_layout.enemies,
            difficulty: stage_diff,
            max_crowns: map_opt.max_crowns,
            target_crowns: final_opt.target_crowns,
            rarity_mask: final_opt.rarity_mask,
            deploy_limit: final_opt.deploy_limit,
            allowed_rows: final_opt.allowed_rows,
            min_cost: final_opt.min_cost,
            max_cost: final_opt.max_cost,
            charagroup: current_charagroup,
            ..Default::default()
        };

        if (3000..=3008).contains(&global_id_val) {
            stage_struct.base_id = stage_id as i32;

            if let Some((energy, init_track, boss_track)) = story_data.get(&stage_id) {
                stage_struct.energy = *energy;
                stage_struct.xp = get_hardcoded_xp(global_id_val, stage_id as usize);
                stage_struct.init_track = *init_track as u32;
                stage_struct.boss_track = *boss_track;
            }
        } else if let Some(entry) = stage_data_entries.get(stage_id as usize) {
            stage_struct.energy = entry.energy;
            stage_struct.xp = entry.xp;
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
            stage_struct.rewards = entry.rewards.clone();
        }

        registry.stages.insert(stage_key.clone(), stage_struct);
        map_struct.stages.push(stage_key);
    }

    if !map_struct.stages.is_empty() {
        map_struct.stages.sort();
        registry.maps.insert(map_struct.id.clone(), map_struct);
    }
}