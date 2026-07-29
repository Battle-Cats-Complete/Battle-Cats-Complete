use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::Category;
use nyanko::chapter::map::{DropItemEntry, LockSkipDataEntry, MapOptionEntry, RuleType, ScoreBonusMapEntry, SpecialRulesMapEntry, SpecialRulesMapOptionEntry};
use nyanko::chapter::stage::{CharaGroupEntry, FixedFormationEntry, ScatCpuSetting, StageNameEntry, StageOptionEntry, get_hardcoded_xp};
use nyanko::common::tools::file;
use rayon::prelude::*;
use tracing::{debug, info, instrument, trace, warn};

use crate::common::formats::{gatyaitembuy, gatyaitemname, GatyaItemBuy, GatyaItemName};
use crate::common::io::cache;
use crate::common::resolver;
use crate::modules::cat::waiter::{unitbuy, unitexplanation};
use crate::modules::enemy::waiter::enemyname;
use crate::modules::settings::ScannerConfig;

use super::paths;
use super::waiter::{
    battleground, certification_preset, charagroup, difficulty_level, drop_chara, dropitem,
    ex_option, fixed_formation, lockskipdata, map_name, map_option, mapstagedata,
    scatcpusetting, scorebonusmap, specialrulesmap, specialrulesmapoption, stage_option, stagename
};
use super::{GlobalMapId, GlobalStageId, Map, Stage, StageRegistry};

struct ScanContext<'a> {
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

struct StageCache;

impl cache::CacheSpec for StageCache {
    type Data = StageRegistry;
    const FILE: &'static str = "stages_cache.bin";
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug)]
pub struct StageDictionaries {
    pub enemy_name_registry: Vec<String>,
    pub item_buy_registry: HashMap<u32, GatyaItemBuy>,
    pub item_name_registry: HashMap<usize, GatyaItemName>,
    pub drop_chara_registry: HashMap<u32, u32>,
    pub unit_buy_registry: HashMap<u32, UnitBuy>,
    pub cat_name_registry: HashMap<u32, Vec<String>>,
    pub lock_skip_registry: HashMap<u32, LockSkipDataEntry>,
    pub scat_cpu_setting: ScatCpuSetting,
    pub active_language_priority: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StageBundle {
    pub registry: StageRegistry,
    pub dictionaries: StageDictionaries,
}

struct CategoryInfo {
    prefix: String,
    category: Category,
    cat_path: PathBuf,
    stage_names: HashMap<u32, StageNameEntry>,
}

struct MapJob {
    category_index: usize,
    map_id: u32,
    map_path: PathBuf,
}

pub fn load(config: ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> StageBundle {
    let dictionaries = build_dictionaries(&config);

    if !resolver::is_mod_active()
        && let Some((hash, registry)) = cache::read::<StageCache>() {
        debug!(hash, maps = registry.maps.len(), stages = registry.stages.len(), "loaded stages from cache fast-path");
        return StageBundle { registry, dictionaries };
    }

    StageBundle { registry: scan(&config, progress), dictionaries }
}

#[instrument(level = "debug", skip(config))]
fn build_dictionaries(config: &ScannerConfig) -> StageDictionaries {
    trace!("Loading auxiliary stage dictionaries");
    let langs = &config.language_priority;

    let enemy_dir = Path::new(paths::DIR_ENEMIES);
    let enemy_name_registry = enemyname(enemy_dir, langs);

    let tables_dir = Path::new(paths::DIR_TABLES);
    let item_buy_registry = gatyaitembuy::load(tables_dir, "Gatyaitembuy.csv", langs);

    let names_dir = paths::gatya_name_dir();
    let item_name_registry = gatyaitemname::load(&names_dir, "GatyaitemName.csv", langs);

    let stages_dir = Path::new(paths::DIR_STAGES);
    let drop_chara_registry = drop_chara(stages_dir, "drop_chara.csv", langs);
    let lock_skip_registry = lockskipdata(stages_dir, "LockSkipData.csv", langs);
    let scat_cpu_setting = scatcpusetting(stages_dir, "ScatCPUsetting.csv", langs);

    let cats_dir = Path::new(paths::DIR_CATS);
    let unit_buy_registry = unitbuy(cats_dir, langs);

    let mut cat_name_registry = HashMap::new();
    for &unit_id in unit_buy_registry.keys() {
        let cat_folder = paths::cat_folder(unit_id);
        let expl = unitexplanation(unit_id, &cat_folder, langs);
        let names: Vec<String> = expl.names
            .into_iter()
            .flatten()
            .map(|n| n.to_lowercase())
            .collect();
        cat_name_registry.insert(unit_id, names);
    }

    StageDictionaries {
        enemy_name_registry,
        item_buy_registry,
        item_name_registry,
        drop_chara_registry,
        unit_buy_registry,
        cat_name_registry,
        lock_skip_registry,
        scat_cpu_setting,
        active_language_priority: config.language_priority.clone(),
    }
}

fn scan(config: &ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> StageRegistry {
    info!("--- STAGE SCANNER INITIATED ---");
    let registry = scan_all(&config.language_priority, progress);
    info!("--- STAGE SCANNER COMPLETE: Found {} maps and {} stages ---", registry.maps.len(), registry.stages.len());

    if !resolver::is_mod_active() {
        if registry.maps.is_empty() {
            warn!("Registry is empty! Skipping cache save to prevent overwriting with blank data.");
            return registry;
        }

        let hash = cache::get_game_hash(None);
        cache::write::<StageCache>(hash, &registry);
    }

    registry
}

#[instrument(skip_all)]
fn scan_all(langs: &[String], progress: impl Fn(usize, usize) + Sync) -> StageRegistry {
    let reg_mtx = Mutex::new(StageRegistry::default());
    let root = Path::new(paths::DIR_STAGES);

    info!("Loading global table dictionaries into ScanContext...");
    let ctx = ScanContext {
        lang_priority: langs,
        map_names: map_name(&paths::stage_sub("Map_Name"), "Map_Name.csv", langs),
        map_options: map_option(root, "Map_option.csv", langs),
        stage_options: stage_option(root, "Stage_option.csv", langs),
        charagroups: charagroup(root, "Charagroup.csv", langs),
        drop_items: dropitem(root, "DropItem.csv", langs),
        score_bonuses: scorebonusmap(&Path::new(paths::DIR_CATEGORIES).join("R"), "ScoreBonusMap.json", langs),
        special_rules: specialrulesmap(&Path::new(paths::DIR_CATEGORIES).join("SR"), "SpecialRulesMap.json", langs),
        special_rule_options: specialrulesmapoption(&Path::new(paths::DIR_CATEGORIES).join("SR"), "SpecialRulesMapOption.json", langs),
        ex_options: ex_option(root, "EX_option.csv", langs),
        difficulties: difficulty_level(root, "difficulty_level.tsv", langs),
        fixed_formations: fixed_formation(&paths::stage_sub("fixedlineup"), "fixed_formation.csv", langs),
    };

    let (categories, jobs) = enumerate_maps(langs);

    let total = jobs.len();
    let done = AtomicUsize::new(0);

    jobs.par_iter().for_each(|job| {
        if let Some(info) = categories.get(job.category_index) {
            process_map(&reg_mtx, info, job.map_id, &job.map_path, &ctx);
        }

        let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
        progress(finished, total);
    });

    reg_mtx.into_inner().unwrap_or_default()
}

fn enumerate_maps(lang_priority: &[String]) -> (Vec<CategoryInfo>, Vec<MapJob>) {
    let cat_dir = Path::new(paths::DIR_CATEGORIES);
    info!("Targeting base categories directory: {}", cat_dir.display());

    let Ok(categories_dir) = fs::read_dir(cat_dir) else {
        warn!("Failed to read categories directory! Make sure it exists.");
        return (Vec::new(), Vec::new());
    };

    let cat_entries: Vec<_> = categories_dir.flatten().collect();
    info!("Found {} items in categories directory to evaluate", cat_entries.len());

    let mut categories = Vec::new();
    let mut jobs = Vec::new();

    for cat_entry in cat_entries {
        let cat_path = cat_entry.path();
        if !cat_path.is_dir() { continue; }

        let Some(os_name) = cat_path.file_name() else { continue; };
        let cat_prefix = os_name.to_string_lossy().to_string();
        let category = Category::from_prefix(&cat_prefix);

        debug!("Scanning category folder: {}", cat_prefix);

        let mut stage_names = HashMap::new();

        for (dir, file) in paths::stage_name_targets(&cat_path, &cat_prefix) {
            stage_names = stagename(&dir, &file, lang_priority);
            if !stage_names.is_empty() {
                break;
            }
        }

        let Ok(maps_dir) = fs::read_dir(&cat_path) else {
            warn!("Failed to read contents of category directory: {}", cat_path.display());
            continue;
        };

        let mut map_entries = Vec::new();

        for map_entry in maps_dir.flatten() {
            let map_path = map_entry.path();
            if !map_path.is_dir() { continue; }

            let Some(os_folder) = map_path.file_name() else { continue; };
            let Ok(map_id) = os_folder.to_string_lossy().parse::<u32>() else { continue; };
            map_entries.push((map_id, map_path));
        }

        if cat_prefix == "EC" {
            let base_path = cat_path.join("000");
            if base_path.exists() {
                if !map_entries.iter().any(|(id, _)| *id == 1) {
                    map_entries.push((1, base_path.clone()));
                }
                if !map_entries.iter().any(|(id, _)| *id == 2) {
                    map_entries.push((2, base_path.clone()));
                }
            }
        }

        let category_index = categories.len();
        categories.push(CategoryInfo { prefix: cat_prefix, category, cat_path, stage_names });

        for (map_id, map_path) in map_entries {
            jobs.push(MapJob { category_index, map_id, map_path });
        }
    }

    (categories, jobs)
}

fn process_map(reg_mtx: &Mutex<StageRegistry>, info: &CategoryInfo, map_id: u32, map_path: &Path, ctx: &ScanContext) {
    let category = &info.category;
    let cat_prefix = info.prefix.as_str();
    let mut global_map_id = category.global_map_id(map_id);

    if global_map_id.is_none() || global_map_id == Some(map_id) {
        let routed_id = match (cat_prefix, map_id) {
            ("EC", 0) | ("Z", 0) => Some(3000),
            ("EC", 1) | ("Z", 1) => Some(3001),
            ("EC", 2) | ("Z", 2) => Some(3002),
            ("W", 4)  | ("Z", 4) => Some(3003),
            ("W", 5)  | ("Z", 5) => Some(3004),
            ("W", 6)  | ("Z", 6) => Some(3005),
            ("Space", 7) | ("Z", 7) => Some(3006),
            ("Space", 8) | ("Z", 8) => Some(3007),
            ("Space", 9) | ("Z", 9) => Some(3008),
            _ => global_map_id,
        };
        if routed_id.is_some() && routed_id != global_map_id {
            global_map_id = routed_id;
        }
    }

    let mut active_stage_names = info.stage_names.clone();

    if cat_prefix == "Z" {
        let proxy_prefix = match map_id {
            0..=2 => "EC",
            4..=6 => "W",
            7..=9 => "Space",
            _ => "",
        };

        if !proxy_prefix.is_empty() {
            let parent_dir = info.cat_path.parent().unwrap_or(&info.cat_path);
            let proxy_path = parent_dir.join(proxy_prefix);

            debug!("Fetching proxy names from {} for Z map {}", proxy_prefix, map_id);

            for (dir, file) in paths::stage_name_targets(&proxy_path, proxy_prefix) {
                let names = stagename(&dir, &file, ctx.lang_priority);
                if names.is_empty() { continue; }

                active_stage_names = names;
                break;
            }
        }
    }

    let map_display_name = global_map_id
        .and_then(|id| ctx.map_names.get(&id))
        .filter(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| format!("{:03}", map_id));

    load_map(
        reg_mtx,
        category,
        map_id,
        map_path,
        &map_display_name,
        &active_stage_names,
        ctx,
        global_map_id
    );
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
fn load_map(
    reg_mtx: &Mutex<StageRegistry>,
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

    let is_story = global_map_id
        .map(|id| (3000..=3008).contains(&id))
        .unwrap_or(true);

    let (stage_ids, stage_structs) = if is_story {
        load_story_stages(map_path, category, map_id, map_opt.max_crowns, stage_names, ctx, global_map_id)
    } else {
        load_legend_stages(map_path, category, map_id, map_opt.max_crowns, stage_names, ctx, global_map_id)
    };

    if stage_ids.is_empty() {
        warn!("Map {:03} (Category: {:?}) returned zero parsed stages!", map_id, category);
    }

    map_struct.stages = stage_ids;

    if map_struct.stages.is_empty() {
        return;
    }

    map_struct.stages.sort();

    if let Ok(mut reg) = reg_mtx.lock() {
        reg.maps.insert(map_key, map_struct);
        reg.stages.extend(stage_structs);
    }
}

#[allow(clippy::too_many_arguments)]
fn load_story_stages(
    map_path: &Path,
    category: &Category,
    map_id: u32,
    max_crowns: u8,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> (Vec<u32>, Vec<(GlobalStageId, Stage)>) {
    let mut id_list = Vec::new();
    let mut stage_list = Vec::new();
    let mut story_data = HashMap::new();
    let mut inv_story_data = None;

    let story_file = if category.map_prefix() == "Z" {
        match global_map_id {
            Some(3000) => "stageNormal0_0_Z.csv",
            Some(3001) => "stageNormal0_1_Z.csv",
            Some(3002) => "stageNormal0_2_Z.csv",
            Some(3003) => "stageNormal1_0_Z.csv",
            Some(3004) => "stageNormal1_1_Z.csv",
            Some(3005) => "stageNormal1_2_Z.csv",
            Some(3006) => "stageNormal2_0_Z.csv",
            Some(3007) => "stageNormal2_1_Z.csv",
            Some(3008) => "stageNormal2_2_Z.csv",
            _ => "",
        }
    } else {
        match global_map_id {
            Some(3000..=3002) => "stageNormal0.csv",
            Some(3003) => "stageNormal1_0.csv",
            Some(3004) => "stageNormal1_1.csv",
            Some(3005) => "stageNormal1_2.csv",
            Some(3006) => "stageNormal2_0.csv",
            Some(3007) => "stageNormal2_1.csv",
            Some(3008) => "stageNormal2_2.csv",
            _ => "",
        }
    };

    let inv_story_file = if category.map_prefix() == "Z" {
        match global_map_id {
            Some(3008) => "stageNormal2_2_Invasion_Z.csv",
            _ => "",
        }
    } else {
        match global_map_id {
            Some(3008) => "stageNormal2_2_Invasion.csv",
            _ => "",
        }
    };

    if !story_file.is_empty() {
        let resolved_paths = resolver::get(map_path, [story_file], ctx.lang_priority);
        if let Some(story_path) = resolved_paths.first() {
            if let Ok(content) = fs::read_to_string(story_path) {
                let sep = file::detect_separator(&content);
                for (idx, line) in content.lines().skip(2).enumerate() {
                    let clean = line.split("//").next().unwrap_or("").trim();
                    if clean.is_empty() { continue; }

                    let parts: Vec<&str> = clean.split(sep).collect();
                    if parts.len() < 6 { continue; }

                    let energy = parts[0].trim().parse().unwrap_or(0);
                    let init_track: i16 = parts[2].trim().parse().unwrap_or(0);
                    let boss_track: i16 = parts[5].trim().parse().unwrap_or(-1);
                    story_data.insert(idx as u32, (energy, init_track, boss_track));
                }
            } else {
                warn!("Expected story data file missing or unreadable: {}", story_path.display());
            }
        } else {
            warn!("Expected story data file missing: {}", map_path.join(story_file).display());
        }
    }

    if !inv_story_file.is_empty() {
        let resolved_paths = resolver::get(map_path, [inv_story_file], ctx.lang_priority);
        if let Some(inv_story_path) = resolved_paths.first()
            && let Ok(content) = fs::read_to_string(inv_story_path) {
            let sep = file::detect_separator(&content);
            for line in content.lines().skip(2) {
                let clean = line.split("//").next().unwrap_or("").trim();
                if clean.is_empty() { continue; }

                let parts: Vec<&str> = clean.split(sep).collect();
                if parts.len() >= 6 {
                    let energy = parts[0].trim().parse().unwrap_or(0);
                    let init_track: i16 = parts[2].trim().parse().unwrap_or(0);
                    let boss_track: i16 = parts[5].trim().parse().unwrap_or(-1);
                    inv_story_data = Some((energy, init_track, boss_track));
                    break;
                }
            }
        }
    }

    let Ok(stages_dir) = fs::read_dir(map_path) else { return (id_list, stage_list); };
    let mut invasion_path = None;

    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() { continue; }

        let Some(os_folder) = stage_path.file_name() else { continue; };
        let folder_str = os_folder.to_string_lossy();

        if folder_str == "Invasion" {
            invasion_path = Some(stage_path);
            continue;
        }

        let Ok(stage_id) = folder_str.parse::<u32>() else { continue; };

        let is_ec_group = category.map_prefix() == "EC" || (category.map_prefix() == "Z" && map_id <= 2);

        if is_ec_group {
            if map_id == 0 && stage_id > 47 { continue; }
            if map_id == 1 && (stage_id == 47 || stage_id == 48 || stage_id > 49) { continue; }
            if map_id == 2 && (stage_id == 47 || stage_id == 48 || stage_id == 49 || stage_id > 50) { continue; }
        }

        let Some(raw_layout) = find_battleground(&stage_path, ctx.lang_priority) else {
            warn!("Failed to find battleground CSV in story stage directory: {}", stage_path.display());
            continue;
        };

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
        stage_list.push((stage_key, stage_struct));
        id_list.push(stage_id);
    }

    if let Some(inv_path) = invasion_path {
        let inv_stage_id = id_list.iter().max().copied().unwrap_or(0) + 1;

        if let Some(raw_layout) = find_battleground(&inv_path, ctx.lang_priority) {
            let mut stage_struct = build_base_stage(
                category, map_id, inv_stage_id, max_crowns, &raw_layout, stage_names, ctx, global_map_id
            );

            stage_struct.name = "Invasion".to_string();
            stage_struct.base_id = inv_stage_id as i32;

            if let Some((energy_val, init_track_val, boss_track_val)) = inv_story_data {
                stage_struct.energy = energy_val;
                stage_struct.xp = global_map_id.map(|id| get_hardcoded_xp(id, inv_stage_id as usize)).unwrap_or(0);
                stage_struct.init_track = init_track_val as u32;
                stage_struct.boss_track = boss_track_val;
            }

            let stage_key = GlobalStageId { category: category.clone(), map: map_id, stage: inv_stage_id };
            stage_list.push((stage_key, stage_struct));
            id_list.push(inv_stage_id);
        } else {
            warn!("Failed to find battleground CSV in invasion stage directory: {}", inv_path.display());
        }
    }

    (id_list, stage_list)
}

#[allow(clippy::too_many_arguments)]
fn load_legend_stages(
    map_path: &Path,
    category: &Category,
    map_id: u32,
    max_crowns: u8,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> (Vec<u32>, Vec<(GlobalStageId, Stage)>) {
    let mut id_list = Vec::new();
    let mut stage_list = Vec::new();
    let mut data_entries = Vec::new();

    if let Ok(files_dir) = fs::read_dir(map_path) {
        for file_entry in files_dir.flatten() {
            let path = file_entry.path();

            let is_csv = path.extension().is_some_and(|ext| ext == OsStr::new("csv"));
            if !is_csv { continue; }

            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else { continue; };
            if !file_name.starts_with("MapStageData") { continue; }

            data_entries = mapstagedata(map_path, file_name, ctx.lang_priority);
            if !data_entries.is_empty() { break; }
        }
    }

    if data_entries.is_empty() {
        data_entries = mapstagedata(map_path, "stage.csv", ctx.lang_priority);
    }

    let Ok(stages_dir) = fs::read_dir(map_path) else { return (id_list, stage_list); };

    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() { continue; }

        let Some(os_folder) = stage_path.file_name() else { continue; };
        let Ok(stage_id) = os_folder.to_string_lossy().parse::<u32>() else { continue; };

        let Some(raw_layout) = find_battleground(&stage_path, ctx.lang_priority) else {
            warn!("Failed to find battleground CSV in legend stage directory: {}", stage_path.display());
            continue;
        };

        let mut stage_struct = build_base_stage(
            category, map_id, stage_id, max_crowns, &raw_layout, stage_names, ctx, global_map_id
        );

        if let Some(entry) = data_entries.get(stage_id as usize) {
            stage_struct.energy = entry.energy;
            stage_struct.xp = entry.xp;
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
            stage_struct.rewards = entry.rewards.clone();
        }

        let stage_key = GlobalStageId { category: category.clone(), map: map_id, stage: stage_id };
        stage_list.push((stage_key, stage_struct));
        id_list.push(stage_id);
    }

    (id_list, stage_list)
}

fn find_battleground(stage_path: &Path, lang_priority: &[String]) -> Option<nyanko::chapter::stage::Battleground> {
    let Ok(files_dir) = fs::read_dir(stage_path) else { return None; };

    for file_entry in files_dir.flatten() {
        let path = file_entry.path();

        let is_csv = path.extension().is_some_and(|ext| ext == OsStr::new("csv"));
        if !is_csv { continue; }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else { continue; };

        let parsed = battleground(stage_path, file_name, lang_priority);
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
    let is_story_name = matches!(category.map_prefix().as_str(), "EC" | "W" | "Space" | "Z");

    let stage_display_name = if is_story_name {
        let mut lookup_id = stage_id;

        let is_ec = category.map_prefix() == "EC";
        let is_z_ec = category.map_prefix() == "Z" && map_id <= 2;

        if (is_ec || is_z_ec) && matches!(stage_id, 48..=50) {
            lookup_id = 47;
        }

        stage_names.get(&lookup_id)
            .and_then(|entry| entry.names.first())
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id))
    } else {
        stage_names.get(&map_id)
            .and_then(|entry| entry.names.get(stage_id as usize))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id))
    };

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
    let fixed_lineup_directory = paths::stage_sub("fixedlineup");

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