use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nyanko::cat::unit::{Battle, LevelCurve, Talent, TalentCost, UnitBuy, UnitEvolve};
use nyanko::graphics::rig::Animation;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::common::io::cache;
use crate::common::resolver;
use crate::modules::cat::paths;
use crate::modules::settings::ScannerConfig;

use crate::modules::cat::waiter::{
    skillacquisition, skilldescriptions, skilllevel, unitbuy, unitevolve, unitexplanation, unitlevel,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: Option<PathBuf>,
    pub deploy_icon_paths: [Option<PathBuf>; 4],
    pub names: [Option<String>; 4],
    pub description: [Option<Vec<String>>; 4],
    pub forms: [bool; 4],
    pub stats: [Option<Battle>; 4],
    pub curve: Option<LevelCurve>,
    pub atk_anim_frames: [i32; 4],
    pub egg_ids: Option<(i32, i32)>,
    pub talent_data: Option<Talent>,
    pub unitbuy: UnitBuy,
    pub evolve_text: UnitEvolve,
    #[serde(skip)] pub talent_costs: Arc<HashMap<u8, TalentCost>>,
    #[serde(skip)] pub skill_descriptions: Arc<Vec<String>>,
}

impl CatEntry {
    pub fn id_str(&self, form_index: usize) -> String { format!("{:03}-{}", self.id, form_index + 1) }

    pub fn display_name(&self, form_index: usize) -> String {
        if let Some(Some(name)) = self.names.get(form_index)
            && !name.is_empty() {
            return name.clone();
        }
        self.id_str(form_index)
    }

    pub fn base_id_str(&self) -> String { format!("{:03}", self.id) }
}

fn is_valid_png(path: &Path) -> bool {
    let Ok(mut file_handle) = fs::File::open(path) else { return false; };
    let mut buffer = [0u8; 25];
    if file_handle.read_exact(&mut buffer).is_err() { return false; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return false; }
    buffer[24] >= 8
}

struct CatCache;

impl cache::CacheSpec for CatCache {
    type Data = Vec<CatEntry>;
    const FILE: &'static str = "cats_cache.bin";
    const VERSION: u32 = 1;
}

pub fn load(config: ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> Vec<CatEntry> {
    let cats_directory = Path::new(paths::DIR_CATS);
    let priority = &config.language_priority;

    if !resolver::is_mod_active()
        && let Some((hash, cached_cats)) = cache::read::<CatCache>() {
        debug!(hash, count = cached_cats.len(), "loaded cats from cache fast-path");

        let talent_costs_arc = Arc::new(skilllevel(cats_directory, priority));
        let skill_descriptions_arc = Arc::new(skilldescriptions(cats_directory, priority));

        return cached_cats.into_iter().map(|mut cat| {
            cat.talent_costs = Arc::clone(&talent_costs_arc);
            cat.skill_descriptions = Arc::clone(&skill_descriptions_arc);
            cat
        }).collect();
    }

    scan(config, progress)
}

fn scan(config: ScannerConfig, progress: impl Fn(usize, usize) + Sync) -> Vec<CatEntry> {
    trace!("starting cat repository scan");
    let cats_directory = Path::new(paths::DIR_CATS);
    let priority = &config.language_priority;

    let unitbuy_resolved = resolver::get(cats_directory, [paths::UNIT_BUY], priority).into_iter().next();
    let unitlevel_resolved = resolver::get(cats_directory, [paths::UNIT_LEVEL], priority).into_iter().next();

    if unitbuy_resolved.is_none() || unitlevel_resolved.is_none() {
        warn!("cat scan aborted: unitbuy/unitlevel tables unavailable");
        return Vec::new();
    }

    let tables = ScanTables {
        level_curves: Arc::new(unitlevel(cats_directory, priority)),
        unit_buys: Arc::new(unitbuy(cats_directory, priority)),
        talents: Arc::new(skillacquisition(cats_directory, priority)),
        evolve_texts: Arc::new(unitevolve(cats_directory, priority)),
        talent_costs: Arc::new(skilllevel(cats_directory, priority)),
        skill_descriptions: Arc::new(skilldescriptions(cats_directory, priority)),
    };

    let folder_entries: Vec<PathBuf> = match fs::read_dir(cats_directory) {
        Ok(read_dir_iter) => read_dir_iter
            .filter_map(|entry_result| entry_result.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect(),
        Err(_) => Vec::new(),
    };

    let total_folders = folder_entries.len();
    let processed_count = AtomicUsize::new(0);

    let mut parsed_cats: Vec<CatEntry> = folder_entries.par_iter().filter_map(|folder_path| {
        let cat = process_cat_entry(folder_path, &tables, &config);

        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
        progress(done, total_folders);

        cat
    }).collect();

    parsed_cats.sort_by_key(|cat| cat.id);

    if !resolver::is_mod_active() {
        let current_hash = cache::get_game_hash(None);
        cache::write::<CatCache>(current_hash, &parsed_cats);
    }

    parsed_cats
}
struct ScanTables {
    level_curves: Arc<Vec<LevelCurve>>,
    unit_buys: Arc<HashMap<u32, UnitBuy>>,
    talents: Arc<HashMap<u16, Talent>>,
    evolve_texts: Arc<HashMap<u32, UnitEvolve>>,
    talent_costs: Arc<HashMap<u8, TalentCost>>,
    skill_descriptions: Arc<Vec<String>>,
}

fn process_cat_entry(
    original_folder_path: &Path,
    tables: &ScanTables,
    config: &ScannerConfig
) -> Option<CatEntry> {
    let folder_stem = original_folder_path.file_name()?.to_str()?;
    let cat_id = folder_stem.parse::<u32>().ok()?;
    let cats_root_dir = Path::new(paths::DIR_CATS);
    let priority = &config.language_priority;

    trace!(cat_id = cat_id, "processing cat entry data");

    let stats_path = paths::stats(cats_root_dir, cat_id);

    let stats_parent = stats_path.parent()?;
    let stats_file_name = stats_path.file_name().and_then(|name_str| name_str.to_str())?;

    let resolved_stats = resolver::get(stats_parent, [stats_file_name], priority).into_iter().next();

    if !config.show_invalid_cats && resolved_stats.is_none() {
        return None;
    }

    let ub_row = tables.unit_buys.get(&cat_id)?;
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);

    let mut forms_existence = [false; 4];
    let mut deploy_icon_paths: [Option<PathBuf>; 4] = Default::default();
    let mut final_image_path_opt = None;

    for form_idx in 0..4 {
        let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);

        let banner_stem = paths::image_stem(paths::AssetType::Banner, cat_id, form_idx, egg_ids);
        let banner_name = format!("{}.png", banner_stem);
        let mut resolved_banner = resolver::get(&dir, [banner_name.as_str()], priority).into_iter().next();

        if resolved_banner.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_stem = format!("udi{:03}_m00", egg_ids.1);
            let fallback_name = format!("{}.png", fallback_stem);
            resolved_banner = resolver::get(&dir, [fallback_name.as_str()], priority).into_iter().next();
        }

        let icon_stem = paths::image_stem(paths::AssetType::Icon, cat_id, form_idx, egg_ids);
        let icon_name = format!("{}.png", icon_stem);
        let mut resolved_icon = resolver::get(&dir, [icon_name.as_str()], priority).into_iter().next();

        if resolved_icon.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_stem = format!("uni{:03}_m00", egg_ids.1);
            let fallback_name = format!("{}.png", fallback_stem);
            resolved_icon = resolver::get(&dir, [fallback_name.as_str()], priority).into_iter().next();
        }

        let mut form_valid = false;
        match form_idx {
            0 | 1 => {
                if let Some(banner_file) = &resolved_banner {
                    if config.show_invalid_cats || is_valid_png(banner_file) {
                        form_valid = true;
                    }
                } else if config.show_invalid_cats {
                    form_valid = dir.exists();
                }
            }
            2 => form_valid = ub_row.true_form_id > 0,
            3 => form_valid = ub_row.ultra_form_id > 0,
            _ => unreachable!(),
        }

        forms_existence[form_idx] = form_valid;

        if form_valid {
            deploy_icon_paths[form_idx] = resolved_icon;
        }
    }

    if !config.show_invalid_cats && forms_existence.iter().all(|&is_valid| !is_valid) {
        return None;
    }

    for form_idx in (0..=config.preferred_form).rev() {
        if forms_existence[form_idx] {
            let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);
            let banner_stem = paths::image_stem(paths::AssetType::Banner, cat_id, form_idx, egg_ids);
            let banner_name = format!("{}.png", banner_stem);
            let mut resolved_fallback = resolver::get(&dir, [banner_name.as_str()], priority).into_iter().next();

            if resolved_fallback.is_none() && form_idx == 1 && egg_ids.1 != -1 {
                let fallback_stem = format!("udi{:03}_m00", egg_ids.1);
                let fallback_name = format!("{}.png", fallback_stem);
                resolved_fallback = resolver::get(&dir, [fallback_name.as_str()], priority).into_iter().next();
            }

            if resolved_fallback.is_some() {
                final_image_path_opt = resolved_fallback;
                break;
            }
        }
    }

    let mut attack_anim_frames = [0; 4];
    for i in 0..4 {
        if !forms_existence[i] { continue; }
        let anim_path = paths::maanim(cats_root_dir, cat_id, i, egg_ids, 2);

        if let (Some(parent_dir), Some(anim_file_name)) = (anim_path.parent(), anim_path.file_name().and_then(|name_str| name_str.to_str()))
            && let Some(resolved) = resolver::get(parent_dir, [anim_file_name], priority).into_iter().next()
            && let Ok(bytes) = fs::read(&resolved) {
            let content = String::from_utf8_lossy(&bytes);
            let duration = Animation::scan_duration(content.as_bytes());
            attack_anim_frames[i] = if duration > 0 { duration + 1 } else { 0 };
        }
    }

    let mut cat_stats: [Option<Battle>; 4] = [const { None }; 4];
    if let Some(resolved) = resolved_stats
        && let Ok(bytes) = fs::read(&resolved) {

        if let Ok(parsed_profiles) = Battle::parse(&bytes) {
            for (line_index, profile) in parsed_profiles.into_iter().enumerate().take(4) {
                cat_stats[line_index] = Some(profile);
            }
        } else if config.show_invalid_cats {
        }
    }

    let explanation = unitexplanation(cat_id, original_folder_path, priority);

    let egg_ids_opt = if egg_ids.0 != -1 || egg_ids.1 != -1 {
        Some(egg_ids)
    } else {
        None
    };

    Some(CatEntry {
        id: cat_id,
        image_path: final_image_path_opt,
        deploy_icon_paths,
        names: explanation.names,
        description: explanation.descriptions,
        forms: forms_existence,
        stats: cat_stats,
        curve: tables.level_curves.get(cat_id as usize).cloned(),
        atk_anim_frames: attack_anim_frames,
        egg_ids: egg_ids_opt,
        talent_data: tables.talents.get(&(cat_id as u16)).cloned(),
        unitbuy: ub_row.clone(),
        evolve_text: tables.evolve_texts.get(&{ cat_id }).cloned().unwrap_or_default(),
        talent_costs: Arc::clone(&tables.talent_costs),
        skill_descriptions: Arc::clone(&tables.skill_descriptions),
    })
}
