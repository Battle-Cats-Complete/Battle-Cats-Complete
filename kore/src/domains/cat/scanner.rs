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

use crate::common::job::Ticker;
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

const PLACEHOLDER_EDGE: u32 = 1;

fn is_valid_png(path: &Path) -> bool {
    let Ok(mut file_handle) = fs::File::open(path) else { return false; };
    let mut buffer = [0u8; 25];
    if file_handle.read_exact(&mut buffer).is_err() { return false; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return false; }
    if buffer[24] < 8 { return false; }

    let width = u32::from_be_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
    let height = u32::from_be_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);

    width > PLACEHOLDER_EDGE && height > PLACEHOLDER_EDGE
}

struct Images {
    forms: [bool; 4],
    deploy_icons: [Option<PathBuf>; 4],
    banner: Option<PathBuf>,
    art: bool,
}

const EVOLVED_FORM: usize = 1;

fn egg_fallback(vfs: &Vfs, asset: files::AssetType, form: usize, egg_ids: (i32, i32)) -> Option<PathBuf> {
    if form != EVOLVED_FORM || egg_ids.1 == -1 {
        return None;
    }

    vfs.find(&format!("{}{:03}_m00.png", asset.prefix(), egg_ids.1))
}

fn resolve_banner(vfs: &Vfs, id: u32, form: usize, egg_ids: (i32, i32)) -> Option<PathBuf> {
    vfs.find(&files::banner_file(id, form, egg_ids))
        .or_else(|| egg_fallback(vfs, files::AssetType::Banner, form, egg_ids))
}

fn resolve_icon(vfs: &Vfs, id: u32, form: usize, egg_ids: (i32, i32)) -> Option<PathBuf> {
    vfs.find(&files::icon_file(id, form, egg_ids))
        .or_else(|| egg_fallback(vfs, files::AssetType::Icon, form, egg_ids))
}

fn resolve_images(vfs: &Vfs, id: u32, egg_ids: (i32, i32), ub_row: &UnitBuy, config: &ScannerConfig) -> Images {
    let mut forms = [false; 4];
    let mut deploy_icons: [Option<PathBuf>; 4] = Default::default();
    let mut banners: [Option<PathBuf>; 4] = Default::default();
    let mut real = [false; 4];

    for form in 0..forms.len() {
        let banner = resolve_banner(vfs, id, form, egg_ids);
        let icon = resolve_icon(vfs, id, form, egg_ids);

        real[form] = !config.show_invalid_cats && banner.as_deref().is_some_and(is_valid_png);

        forms[form] = match form {
            0 | 1 => base_form_present(form, banner.is_some(), icon.is_some(), real[form], config.show_invalid_cats),
            2 => ub_row.true_form_id > 0,
            _ => ub_row.ultra_form_id > 0,
        };

        if forms[form] {
            deploy_icons[form] = icon;
        }

        banners[form] = banner;
    }

    let picked = (0..=config.preferred_form.min(forms.len() - 1))
        .rev()
        .find(|form| forms[*form] && banners[*form].is_some());

    let banner = picked.and_then(|form| banners[form].take());
    let art = config.show_invalid_cats || real.iter().any(|found| *found);

    Images { forms, deploy_icons, banner, art }
}

fn base_form_present(form: usize, banner: bool, icon: bool, real: bool, show_invalid: bool) -> bool {
    if banner {
        return show_invalid || real;
    }

    show_invalid && (form == 0 || icon)
}

fn valid_forms(images: &Images, config: &ScannerConfig) -> bool {
    config.show_invalid_cats || (images.art && images.forms.iter().any(|valid| *valid))
}

fn valid_stats(found: bool, config: &ScannerConfig) -> bool {
    config.show_invalid_cats || found
}

pub fn listable(vfs: &Vfs, entry: &CatEntry, config: &ScannerConfig) -> bool {
    let egg_ids = entry.egg_ids.unwrap_or((-1, -1));
    let images = resolve_images(vfs, entry.id, egg_ids, &entry.unitbuy, config);

    valid_stats(vfs.find(&files::stats_file(entry.id)).is_some(), config) && valid_forms(&images, config)
}

pub fn revalidate(vfs: &Vfs, entry: &mut CatEntry, config: &ScannerConfig) -> bool {
    let egg_ids = entry.egg_ids.unwrap_or((-1, -1));
    let images = resolve_images(vfs, entry.id, egg_ids, &entry.unitbuy, config);

    let listable = valid_stats(vfs.find(&files::stats_file(entry.id)).is_some(), config)
        && valid_forms(&images, config);

    entry.forms = images.forms;
    entry.deploy_icon_paths = images.deploy_icons;
    entry.image_path = images.banner;

    listable
}

struct CatCache;

impl cache::CacheSpec for CatCache {
    type Data = Vec<CatEntry>;
    const FILE: &'static str = "cats_cache.bin";
}

pub fn purge() {
    cache::purge::<CatCache>();
}

pub fn hydrate(vault: &Vault) -> Option<(u64, Vec<CatEntry>)> {
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
    let ticker = Ticker::default();

    let mut parsed_cats: Vec<CatEntry> = unit_ids.par_iter().filter_map(|&cat_id| {
        let cat = process_cat_entry(cat_id, vfs, &tables, &config);

        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;

        if ticker.ready(done, total_units) {
            progress(done, total_units);
        }

        cat
    }).collect();

    parsed_cats.sort_by_key(|cat| cat.id);

    let key = Some(cache::content_hash(vfs.fingerprint(), &config));
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

    if !valid_stats(resolved_stats.is_some(), config) {
        return None;
    }

    let ub_row = tables.unit_buys.get(&cat_id)?;
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);

    let images = resolve_images(vfs, cat_id, egg_ids, ub_row, config);

    if !valid_forms(&images, config) {
        return None;
    }

    let mut attack_anim_frames = [0; 4];
    for (i, frames) in attack_anim_frames.iter_mut().enumerate() {
        if !images.forms[i] { continue; }
        let anim_name = files::maanim_file(cat_id, i, egg_ids, 2);

        if let Some(resolved) = vfs.find(&anim_name)
            && let Ok(bytes) = fs::read(&resolved) {
            let content = String::from_utf8_lossy(&bytes);
            *frames = Animation::scan_length(content.as_bytes()).unwrap_or(0).max(0);
        }
    }

    let mut cat_stats: [Option<Entity>; 4] = [const { None }; 4];
    if let Some(resolved) = resolved_stats
        && let Ok(bytes) = fs::read(&resolved) {

        if let Ok(parsed_profiles) = unitid::parse(&bytes, None) {
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
        image_path: images.banner,
        deploy_icon_paths: images.deploy_icons,
        names: explanation.names,
        description: explanation.descriptions,
        forms: images.forms,
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

#[cfg(test)]
mod tests {
    use super::base_form_present;

    // A conjured spirit carries neither a banner nor a deploy icon. It used to leave every
    // form false, and the list filter drops a form-less entry outright, so "show invalid
    // cats" listed nothing for it no matter how the setting was set.
    #[test]
    fn an_artless_unit_still_claims_its_first_form_when_invalid_cats_are_shown() {
        assert!(base_form_present(0, false, false, false, true));
        assert!(!base_form_present(1, false, false, false, true), "only the first form is assumed");
    }

    #[test]
    fn hiding_invalid_cats_leaves_every_artless_form_absent() {
        for form in 0..2 {
            assert!(!base_form_present(form, false, false, false, false));
            assert!(!base_form_present(form, false, true, false, false));
        }
    }

    #[test]
    fn a_banner_still_decides_a_form_the_way_it_always_did() {
        assert!(base_form_present(0, true, false, true, false), "a valid banner lists the form");
        assert!(!base_form_present(0, true, false, false, false), "a junk banner stays hidden");
        assert!(base_form_present(0, true, false, false, true), "unless invalid cats are shown");
    }
}
