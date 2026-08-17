use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nyanko::cat::unitid;
use nyanko::cat::unit::{LevelCurve, Talent, TalentCost, UnitBuy, UnitEvolve};
use nyanko::combat::Entity;
use nyanko::graphics::rig::Animation;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::common::io::cache::{self, Scan};
use crate::domains::cat::files;
use crate::domains::cat::waiter::unitexplanation;
use crate::domains::settings::ScannerConfig;
use crate::{Vfs, Vault};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: Option<PathBuf>,
    pub deploy_icon_paths: [Option<PathBuf>; 4],
    pub names: [Option<String>; 4],
    pub description: [Option<Vec<String>>; 4],
    pub forms: [bool; 4],
    pub stats: [Option<Entity>; 4],
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
}

pub fn purge() {
    cache::purge::<CatCache>();
}

pub fn hydrate(config: &ScannerConfig, vault: &Vault) -> Option<(u64, Vec<CatEntry>)> {
    if config.active_mod.is_some() {
        return None;
    }

    let (hash, cached_cats) = cache::read::<CatCache>()?;
    debug!(hash, count = cached_cats.len(), "hydrated cats from cache");

    let talent_costs_arc = vault.vds.cats.talent_costs(&vault.vfs);
    let skill_descriptions_arc = vault.vds.cats.descriptions(&vault.vfs);

    let cats = cached_cats.into_iter().map(|mut cat| {
        cat.talent_costs = Arc::clone(&talent_costs_arc);
        cat.skill_descriptions = Arc::clone(&skill_descriptions_arc);
        cat
    }).collect();

    Some((hash, cats))
}

pub fn persist(payload: &[u8]) {
    cache::store::<CatCache>(payload);
}

pub fn load(config: ScannerConfig, vault: Arc<Vault>, progress: impl Fn(usize, usize) + Sync) -> Scan<Vec<CatEntry>> {
    scan(config, &vault, progress)
}

pub fn scan_single(id: u32, vault: &Vault, config: &ScannerConfig) -> Option<CatEntry> {
    let vfs = &vault.vfs;

    let tables = ScanTables {
        level_curves: vault.vds.cats.curves(vfs),
        unit_buys: vault.vds.cats.unitbuy(vfs),
        talents: vault.vds.cats.talents(vfs),
        evolve_texts: vault.vds.cats.evolve(vfs),
        talent_costs: vault.vds.cats.talent_costs(vfs),
        skill_descriptions: vault.vds.cats.descriptions(vfs),
    };

    process_cat_entry(id, vfs, &tables, config)
}

fn scan(config: ScannerConfig, vault: &Vault, progress: impl Fn(usize, usize) + Sync) -> Scan<Vec<CatEntry>> {
    trace!("starting cat repository scan");
    let vfs = &vault.vfs;

    if vfs.find(files::UNIT_BUY).is_none() || vfs.find(files::UNIT_LEVEL).is_none() {
        warn!("cat scan aborted: unitbuy/unitlevel tables unavailable");
        return Scan { data: Vec::new(), key: None, payload: None };
    }

    let tables = ScanTables {
        level_curves: vault.vds.cats.curves(vfs),
        unit_buys: vault.vds.cats.unitbuy(vfs),
        talents: vault.vds.cats.talents(vfs),
        evolve_texts: vault.vds.cats.evolve(vfs),
        talent_costs: vault.vds.cats.talent_costs(vfs),
        skill_descriptions: vault.vds.cats.descriptions(vfs),
    };

    let unit_ids: Vec<u32> = tables.unit_buys.keys().copied().collect();

    let total_units = unit_ids.len();
    let processed_count = AtomicUsize::new(0);

    let mut parsed_cats: Vec<CatEntry> = unit_ids.par_iter().filter_map(|&cat_id| {
        let cat = process_cat_entry(cat_id, vfs, &tables, &config);

        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
        progress(done, total_units);

        cat
    }).collect();

    parsed_cats.sort_by_key(|cat| cat.id);

    let key = config.active_mod.is_none().then(|| cache::content_hash(&config));
    let payload = key.and_then(|key| cache::encode::<CatCache>(key, &parsed_cats));

    Scan { data: parsed_cats, key, payload }
}
struct ScanTables {
    level_curves: Arc<HashMap<u32, LevelCurve>>,
    unit_buys: Arc<HashMap<u32, UnitBuy>>,
    talents: Arc<HashMap<u32, Talent>>,
    evolve_texts: Arc<HashMap<u32, UnitEvolve>>,
    talent_costs: Arc<HashMap<u8, TalentCost>>,
    skill_descriptions: Arc<Vec<String>>,
}

fn process_cat_entry(
    cat_id: u32,
    vfs: &Vfs,
    tables: &ScanTables,
    config: &ScannerConfig
) -> Option<CatEntry> {
    trace!(cat_id = cat_id, "processing cat entry data");

    let resolved_stats = vfs.find(&files::stats_file(cat_id));

    if !config.show_invalid_cats && resolved_stats.is_none() {
        return None;
    }

    let ub_row = tables.unit_buys.get(&cat_id)?;
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);

    let mut forms_existence = [false; 4];
    let mut deploy_icon_paths: [Option<PathBuf>; 4] = Default::default();
    let mut final_image_path_opt = None;

    for form_idx in 0..4 {
        let banner_stem = files::image_stem(files::AssetType::Banner, cat_id, form_idx, egg_ids);
        let mut resolved_banner = vfs.find(&format!("{}.png", banner_stem));

        if resolved_banner.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_name = format!("udi{:03}_m00.png", egg_ids.1);
            resolved_banner = vfs.find(&fallback_name);
        }

        let icon_stem = files::image_stem(files::AssetType::Icon, cat_id, form_idx, egg_ids);
        let mut resolved_icon = vfs.find(&format!("{}.png", icon_stem));

        if resolved_icon.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_name = format!("uni{:03}_m00.png", egg_ids.1);
            resolved_icon = vfs.find(&fallback_name);
        }

        let mut form_valid = false;
        match form_idx {
            0 | 1 => {
                if let Some(banner_file) = &resolved_banner {
                    if config.show_invalid_cats || is_valid_png(banner_file) {
                        form_valid = true;
                    }
                } else if config.show_invalid_cats {
                    form_valid = resolved_icon.is_some();
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
            let banner_stem = files::image_stem(files::AssetType::Banner, cat_id, form_idx, egg_ids);
            let mut resolved_fallback = vfs.find(&format!("{}.png", banner_stem));

            if resolved_fallback.is_none() && form_idx == 1 && egg_ids.1 != -1 {
                let fallback_name = format!("udi{:03}_m00.png", egg_ids.1);
                resolved_fallback = vfs.find(&fallback_name);
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
        let anim_name = files::maanim_file(cat_id, i, egg_ids, 2);

        if let Some(resolved) = vfs.find(&anim_name)
            && let Ok(bytes) = fs::read(&resolved) {
            let content = String::from_utf8_lossy(&bytes);
            attack_anim_frames[i] = Animation::scan_duration(content.as_bytes())
                .map_or(0, |duration| if duration > 0 { duration + 1 } else { 0 });
        }
    }

    let mut cat_stats: [Option<Entity>; 4] = [const { None }; 4];
    if let Some(resolved) = resolved_stats
        && let Ok(bytes) = fs::read(&resolved) {

        if let Ok(parsed_profiles) = unitid::parse(&bytes) {
            for (line_index, profile) in parsed_profiles.into_iter().enumerate().take(4) {
                cat_stats[line_index] = Some(profile);
            }
        } else if config.show_invalid_cats {
        }
    }

    let explanation = unitexplanation(vfs, cat_id);

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
        curve: tables.level_curves.get(&cat_id).cloned(),
        atk_anim_frames: attack_anim_frames,
        egg_ids: egg_ids_opt,
        talent_data: tables.talents.get(&cat_id).cloned(),
        unitbuy: ub_row.clone(),
        evolve_text: tables.evolve_texts.get(&{ cat_id }).cloned().unwrap_or_default(),
        talent_costs: Arc::clone(&tables.talent_costs),
        skill_descriptions: Arc::clone(&tables.skill_descriptions),
    })
}
