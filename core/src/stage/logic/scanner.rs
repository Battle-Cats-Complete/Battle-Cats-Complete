use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use nyanko::common::tools::csv;
use nyanko::chapter::Category;
use nyanko::chapter::stage::{CharaGroupEntry, FixedFormationEntry, get_hardcoded_xp, StageOptionEntry, StageNameEntry};
use nyanko::chapter::map::{DropItemEntry, MapOptionEntry, ScoreBonusMapEntry, SpecialRulesMapEntry, RuleType, SpecialRulesMapOptionEntry};
use tracing::{debug, instrument, warn};

use crate::settings::logic::state::ScannerConfig;
use crate::stage::paths;
use crate::stage::registry::{Map, Stage, StageRegistry, GlobalMapId, GlobalStageId};
use crate::stage::waiter::{
    battleground, certification_preset, charagroup, difficulty_level, dropitem,
    ex_option, fixed_formation, map_name, map_option, mapstagedata, scorebonusmap,
    specialrulesmap, specialrulesmapoption, stage_option, stagename
};

pub struct ScanContext<'a> {
    pub lang_priority: &'a [String],
    pub map_names: HashMap<u32, String>,
    pub map_options: HashMap<u32, MapOptionEntry>,
    pub stage_options: HashMap<u32, Vec<StageOptionEntry>>,
    pub charagroups: HashMap<u32, CharaGroupEntry>,
    pub drop_items: HashMap<u32, DropItemEntry>,
    pub score_bonuses: HashMap<u32, ScoreBonusMapEntry>,
    pub special_rules: HashMap<u32, SpecialRulesMapEntry>,
    pub special_rule_options: HashMap<u8, SpecialRulesMapOptionEntry>,
    pub ex_options: HashMap<u32, u32>,
    pub difficulties: HashMap<u32, Vec<u16>>,
    pub fixed_formations: HashMap<(u32, u8, u32), FixedFormationEntry>,
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
        map_names: map_name(&root_path.join("Map_Name"), "Map_Name.csv", lang_priority),
        map_options: map_option(root_path, "Map_option.csv", lang_priority),
        stage_options: stage_option(root_path, "Stage_option.csv", lang_priority),
        charagroups: charagroup(root_path, "Charagroup.csv", lang_priority),
        drop_items: dropitem(root_path, "DropItem.csv", lang_priority),
        score_bonuses: scorebonusmap(&root_path.join("R"), "ScoreBonusMap.json", lang_priority),
        special_rules: specialrulesmap(&root_path.join("SR"), "SpecialRulesMap.json", lang_priority),
        special_rule_options: specialrulesmapoption(&root_path.join("SR"), "SpecialRulesMapOption.json", lang_priority),
        ex_options: ex_option(root_path, "EX_option.csv", lang_priority),
        difficulties: difficulty_level(root_path, "difficulty_level.tsv", lang_priority),
        fixed_formations: fixed_formation(&root_path.join("fixedlineup"), "fixed_formation.csv", lang_priority),
    };

    let Ok(categories_dir) = fs::read_dir(root_path) else {
        warn!("Failed to read root stages directory: {}", root_path.display());
        return registry;
    };

    for category_entry in categories_dir.flatten() {
        let cat_path = category_entry.path();
        let Some(os_name) = cat_path.file_name() else { continue; };
        let cat_name = os_name.to_string_lossy();

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
    let Some(os_name) = cat_path.file_name() else { return; };
    let cat_prefix = os_name.to_string_lossy().to_string();
    let category = Category::from_prefix(&cat_prefix);

    let mut stage_names = stagename(cat_path, &format!("StageName_{}.csv", cat_prefix), ctx.lang_priority);
    if stage_names.is_empty() {
        stage_names = stagename(cat_path, &format!("StageName_R{}.csv", cat_prefix), ctx.lang_priority);
    }

    let Ok(maps_dir) = fs::read_dir(cat_path) else {
        debug!("Failed to read category directory: {}", cat_path.display());
        return;
    };

    for map_entry in maps_dir.flatten() {
        let map_path = map_entry.path();
        if !map_path.is_dir() { continue; }

        let Some(os_folder_name) = map_path.file_name() else { continue; };
        let Ok(map_id) = os_folder_name.to_string_lossy().parse::<u32>() else { continue; };

        let mut global_map_id = category.global_map_id(map_id);

        if global_map_id.is_none() || global_map_id == Some(map_id) {
            let routed_id = match (cat_prefix.as_str(), map_id) {
                ("EC", 0) => Some(3000),
                ("EC", 1) => Some(3001),
                ("EC", 2) => Some(3002),
                ("W", 4)  => Some(3003),
                ("W", 5)  => Some(3004),
                ("W", 6)  => Some(3005),
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
            &category,
            map_id,
            &map_path,
            &map_display_name,
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
    category: &Category,
    map_id: u32,
    map_path: &Path,
    map_display_name: &str,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>
) {
    let map_opt = global_map_id
        .and_then(|id| ctx.map_options.get(&id))
        .cloned()
        .unwrap_or_default();

    let map_key = GlobalMapId { category: category.clone(), map: map_id };

    let special_rules = global_map_id.and_then(|id| ctx.special_rules.get(&id)).cloned();
    let mut invalid_combos = Vec::new();

    if let Some(rule) = &special_rules {
        for target_rule in &rule.rules {
            let rule_id = match target_rule {
                RuleType::TrustFund(_) => 0,
                RuleType::CooldownEquality(_) => 1,
                RuleType::RarityLimit(_) => 3,
                RuleType::CheapLabor(_) => 4,
                RuleType::CatCost(_) => 5,
                RuleType::CatProduction(_) => 6,
                RuleType::TotalDeployLimit(_) => 7,
                RuleType::MoreThanOne(_) => 8,
                RuleType::MegaCatCannon(_) => 9,
                RuleType::UniformMotion(_) => 10,
                RuleType::Unknown(id, _) => *id,
            };

            if let Some(opt) = ctx.special_rule_options.get(&rule_id) {
                invalid_combos.extend(&opt.invalid_combo_ids);
            }
        }
        invalid_combos.sort_unstable();
        invalid_combos.dedup();
    }

    let mut map_struct = Map {
        name: map_display_name.to_string(),
        category: category.clone(),
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
        ex_invasion: global_map_id.and_then(|id| ctx.ex_options.get(&id)).cloned(),
        score_bonuses: global_map_id.and_then(|id| ctx.score_bonuses.get(&id)).cloned(),
        special_rules,
        invalid_combos,
        drop_items: global_map_id.and_then(|id| ctx.drop_items.get(&id)).cloned(),
    };

    let is_story_map = global_map_id
        .map(|id| (3000..=3008).contains(&id))
        .unwrap_or(true);

    let stage_ids = if is_story_map {
        load_story_stages(registry, map_path, category, map_id, map_opt.max_crowns, stage_names, ctx, global_map_id)
    } else {
        load_legend_stages(registry, map_path, category, map_id, map_opt.max_crowns, stage_names, ctx, global_map_id)
    };

    map_struct.stages = stage_ids;

    if !map_struct.stages.is_empty() {
        map_struct.stages.sort();
        registry.maps.insert(map_key, map_struct);
    }
}

#[allow(clippy::too_many_arguments)]
fn load_story_stages(
    registry: &mut StageRegistry,
    map_path: &Path,
    category: &Category,
    map_id: u32,
    max_crowns: u8,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> Vec<u32> {
    let mut stages_list = Vec::new();
    let mut story_data = HashMap::new();

    let story_file = match global_map_id {
        Some(3000 | 3001 | 3002) => "stageNormal0.csv",
        Some(3003) => "stageNormal1_0.csv",
        Some(3004) => "stageNormal1_1.csv",
        Some(3005) => "stageNormal1_2.csv",
        Some(3006) => "stageNormal2_0.csv",
        Some(3007) => "stageNormal2_1.csv",
        Some(3008) => "stageNormal2_2.csv",
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

    let Ok(stages_dir) = fs::read_dir(map_path) else { return stages_list; };

    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() { continue; }

        let Some(os_folder) = stage_path.file_name() else { continue; };
        let Ok(stage_id) = os_folder.to_string_lossy().parse::<u32>() else { continue; };

        let Some(raw_layout) = find_battleground(&stage_path, ctx.lang_priority) else { continue; };

        let mut stage_struct = build_base_stage(
            category, map_id, stage_id, max_crowns, &raw_layout, stage_names, ctx, global_map_id
        );

        stage_struct.base_id = stage_id as i32;

        if let Some((energy_val, init_track_val, boss_track_val)) = story_data.get(&stage_id) {
            stage_struct.energy = *energy_val;
            stage_struct.xp = global_map_id.map(|id| get_hardcoded_xp(id, stage_id as usize)).unwrap_or(0);
            stage_struct.init_track = *init_track_val as u32;
            stage_struct.boss_track = *boss_track_val;
        }

        let stage_key = GlobalStageId { category: category.clone(), map: map_id, stage: stage_id };
        registry.stages.insert(stage_key, stage_struct);
        stages_list.push(stage_id);
    }

    stages_list
}

#[allow(clippy::too_many_arguments)]
fn load_legend_stages(
    registry: &mut StageRegistry,
    map_path: &Path,
    category: &Category,
    map_id: u32,
    max_crowns: u8,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> Vec<u32> {
    let mut stages_list = Vec::new();
    let mut stage_data_entries = Vec::new();

    if let Ok(files_dir) = fs::read_dir(map_path) {
        for file_entry in files_dir.flatten() {
            let filename = file_entry.file_name().to_string_lossy().to_string();
            let is_valid_stage_data = filename.starts_with("MapStageData") && filename.ends_with(".csv");

            if !is_valid_stage_data { continue; }

            stage_data_entries = mapstagedata(map_path, &filename, ctx.lang_priority);
            if !stage_data_entries.is_empty() { break; }
        }
    }

    if stage_data_entries.is_empty() {
        stage_data_entries = mapstagedata(map_path, "stage.csv", ctx.lang_priority);
    }

    let Ok(stages_dir) = fs::read_dir(map_path) else { return stages_list; };

    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() { continue; }

        let Some(os_folder) = stage_path.file_name() else { continue; };
        let Ok(stage_id) = os_folder.to_string_lossy().parse::<u32>() else { continue; };

        let Some(raw_layout) = find_battleground(&stage_path, ctx.lang_priority) else { continue; };

        let mut stage_struct = build_base_stage(
            category, map_id, stage_id, max_crowns, &raw_layout, stage_names, ctx, global_map_id
        );

        if let Some(entry) = stage_data_entries.get(stage_id as usize) {
            stage_struct.energy = entry.energy;
            stage_struct.xp = entry.xp;
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
            stage_struct.rewards = entry.rewards.clone();
        }

        let stage_key = GlobalStageId { category: category.clone(), map: map_id, stage: stage_id };
        registry.stages.insert(stage_key, stage_struct);
        stages_list.push(stage_id);
    }

    stages_list
}

fn find_battleground(stage_path: &Path, lang_priority: &[String]) -> Option<nyanko::chapter::stage::Battleground> {
    let Ok(files_dir) = fs::read_dir(stage_path) else { return None; };

    for file_entry in files_dir.flatten() {
        let filename = file_entry.file_name().to_string_lossy().to_string();
        if !filename.ends_with(".csv") { continue; }

        let parsed = battleground(stage_path, &filename, lang_priority);
        if parsed.is_some() { return parsed; }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn build_base_stage(
    category: &Category,
    map_id: u32,
    stage_id: u32,
    max_crowns: u8,
    raw_layout: &nyanko::chapter::stage::Battleground,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> Stage {
    let stage_display_name = stage_names.get(&map_id)
        .and_then(|entry| entry.names.get(stage_id as usize))
        .filter(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| format!("{:02}", stage_id));

    let stage_opts = global_map_id
        .and_then(|id| ctx.stage_options.get(&id))
        .cloned()
        .unwrap_or_default();

    let mut final_opt = StageOptionEntry::default();
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

    let stage_diff = global_map_id
        .and_then(|id| ctx.difficulties.get(&id))
        .and_then(|diff_list| diff_list.get(stage_id as usize))
        .copied()
        .unwrap_or(0);

    let current_charagroup = ctx.charagroups.get(&final_opt.charagroup_id).cloned();

    let mut loaded_fixed_lineups = HashMap::new();
    let fixed_lineup_directory = Path::new(paths::DIR_STAGES).join("fixedlineup");

    for crown_index in 0..max_crowns {
        let Some(map_id_val) = global_map_id else { continue; };
        let Some(formation_data) = ctx.fixed_formations.get(&(map_id_val, crown_index, stage_id)) else { continue; };
        let Some(preset_lineup_json) = certification_preset(
            &fixed_lineup_directory,
            &formation_data.preset_file_name,
            ctx.lang_priority
        ) else { continue; };

        loaded_fixed_lineups.insert(crown_index, preset_lineup_json);
    }

    Stage {
        name: stage_display_name,
        category: category.clone(),
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
        enemies: raw_layout.entries.clone(),
        difficulty: stage_diff,
        max_crowns,
        target_crowns: final_opt.target_crowns,
        rarity_mask: final_opt.rarity_mask,
        deploy_limit: final_opt.deploy_limit,
        allowed_rows: final_opt.allowed_rows,
        min_cost: final_opt.min_cost,
        max_cost: final_opt.max_cost,
        charagroup: current_charagroup,
        fixed_lineups: loaded_fixed_lineups,
        ..Default::default()
    }
}