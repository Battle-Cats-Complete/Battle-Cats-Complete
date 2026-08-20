use std::path::PathBuf;
use std::sync::Arc;

use tracing::warn;

use nyanko::graphics::rig::{Animation, Rig};

use kore::systems::animation::{
    loop_frame, IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT,
    IDX_SURFACE, IDX_WALK,
};
use kore::domains::cat::files::{self, AnimType};
use kore::domains::cat::scanner::CatEntry;
use kore::domains::enemy::files::{self as enemy_files, AnimType as EnemyAnimType};
use kore::domains::enemy::scanner::EnemyEntry;
use kore::Vfs;

const ANIM_SLOTS: [usize; 6] = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
const FALLBACK_PRIORITY: [usize; 6] = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];

const ENEMY_ANIM_SLOTS: [usize; 4] = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB];
const IDX_DIG: usize = 7;

type PrimaryAssets = (PathBuf, PathBuf, PathBuf);
type SecondaryAssets = (PathBuf, PathBuf, PathBuf, PathBuf);

pub struct State {
    pub held_unit: Option<Arc<Rig>>,
    pub current_anim: Option<Arc<Animation>>,
    pub loaded_anim_index: usize,
    pub available_anims: Vec<(usize, PathBuf)>,

    primary_id: String,
    secondary_id: String,
    primary_assets: Option<PrimaryAssets>,
    secondary_assets: Option<SecondaryAssets>,

    loaded_id: String,
    failed_load_id: String,
    current_anim_index: usize,
    cache: RigCache,
}

impl Default for State {
    fn default() -> Self {
        Self {
            held_unit: None,
            current_anim: None,
            loaded_anim_index: IDX_NONE,
            available_anims: Vec::new(),
            primary_id: String::new(),
            secondary_id: String::new(),
            primary_assets: None,
            secondary_assets: None,
            loaded_id: String::new(),
            failed_load_id: String::new(),
            current_anim_index: IDX_NONE,
            cache: RigCache::default(),
        }
    }
}

impl State {
    pub fn base_assets_available(&self) -> bool {
        self.primary_assets.is_some()
    }

    pub fn loop_bound(&self) -> Option<i32> {
        self.current_anim.as_ref().map_or(Some(0), |anim| {
            if self.current_anim_index <= IDX_IDLE { anim.calculate_true_loop() } else { Some(anim.max_frame) }
        })
    }

    pub fn active_index(&self) -> usize {
        self.current_anim_index
    }

    pub fn playback_frame(&self, frame: f32) -> f32 {
        if self.current_anim_index <= IDX_IDLE {
            return frame;
        }

        self.current_anim.as_ref().map_or(frame, |anim| loop_frame(anim, frame))
    }

    pub fn loaded_id(&self) -> &str {
        &self.loaded_id
    }

    fn is_loaded(&self, target_id: &str) -> bool {
        self.loaded_id == target_id && self.held_unit.is_some()
    }

    pub fn primary_id(&self) -> &str {
        &self.primary_id
    }

    pub fn select(&mut self, index: usize) {
        if self.loaded_anim_index == index {
            return;
        }

        self.loaded_anim_index = index;
        self.load_active();
    }

    pub fn invalidate_paths(&mut self) {
        self.primary_id.clear();
        self.secondary_id.clear();
        self.loaded_id.clear();
        self.failed_load_id.clear();
        self.cache.clear();
    }

    pub fn secondary_available(&self) -> bool {
        self.secondary_assets.is_some()
    }

    pub fn sync(&mut self, cat: &CatEntry, form: usize, vfs: &Vfs) {
        self.prepare(cat, form, vfs);
        self.load_active();
    }

    pub fn preload_request(&mut self, cat: &CatEntry, form: usize, vfs: &Vfs) -> Option<PreloadRequest> {
        self.prepare(cat, form, vfs);
        self.build_request()
    }

    pub fn preload_enemy_request(&mut self, enemy: &EnemyEntry, vfs: &Vfs) -> Option<PreloadRequest> {
        self.prepare_enemy(enemy, vfs);
        self.build_request()
    }

    pub fn apply_preload(&mut self, result: PreloadResult) {
        let index = self.loaded_anim_index;
        let desired_id = if index == IDX_SPIRIT { &self.secondary_id } else { &self.primary_id };
        let wanted = index != IDX_NONE
            && !desired_id.is_empty()
            && *desired_id == result.target_id
            && !self.is_loaded(&result.target_id);

        let Some(unit) = result.unit else {
            if wanted {
                self.loaded_id = result.target_id.clone();
                self.failed_load_id = result.target_id;
                self.held_unit = None;
                self.current_anim = None;
                self.current_anim_index = IDX_NONE;
            }
            return;
        };

        self.cache.insert(&result.target_id, unit.clone());
        if let Some(anim) = &result.anim {
            self.cache.store_anim(&result.target_id, result.anim_index, anim.clone());
        }

        if wanted {
            self.held_unit = Some(unit);
            self.loaded_id = result.target_id;
            self.failed_load_id.clear();
            self.current_anim = result.anim;
            self.current_anim_index = result.anim_index;
        }
    }

    fn prepare(&mut self, cat: &CatEntry, form: usize, vfs: &Vfs) {
        let form_char = match form {
            0 => 'f',
            1 => 'c',
            2 => 's',
            _ => 'u',
        };
        let primary_id = format!("{:03}_{}", cat.id, form_char);

        if self.primary_id != primary_id {
            self.rescan_paths(cat, form, &primary_id, vfs);
        }

        self.select_valid_index();
    }

    fn build_request(&mut self) -> Option<PreloadRequest> {
        let index = self.loaded_anim_index;
        if index == IDX_NONE {
            return None;
        }

        let target_id = if index == IDX_SPIRIT { self.secondary_id.clone() } else { self.primary_id.clone() };
        if target_id.is_empty() || self.is_loaded(&target_id) || self.failed_load_id == target_id {
            return None;
        }

        if self.apply_cached(&target_id, index) {
            return None;
        }

        let (png, cut, model, anim) = self.resolve_paths(index);
        Some(PreloadRequest {
            target_id,
            anim_index: index,
            png: png?,
            cut: cut?,
            model: model?,
            anim,
        })
    }

    fn apply_cached(&mut self, target_id: &str, index: usize) -> bool {
        let Some(unit) = self.cache.lookup(target_id) else {
            return false;
        };

        self.held_unit = Some(unit);
        self.loaded_id = target_id.to_string();
        self.failed_load_id.clear();
        let (_, _, _, anim_path) = self.resolve_paths(index);
        self.load_anim(index, anim_path);
        self.current_anim_index = index;
        true
    }

    fn rescan_paths(&mut self, cat: &CatEntry, form: usize, primary_id: &str, vfs: &Vfs) {
        let egg_ids = cat.egg_ids.unwrap_or((-1, -1));

        let resolve = |name: String| -> Option<PathBuf> { vfs.find(&name) };

        let mut available_anims = Vec::new();
        for idx in ANIM_SLOTS {
            let candidate = files::maanim_file(cat.id, form, egg_ids, idx);
            if let Some(resolved) = resolve(candidate) {
                available_anims.push((idx, resolved));
            }
        }

        let primary_assets = (|| {
            let png = resolve(files::anim_file(cat.id, form, egg_ids, AnimType::Png))?;
            let cut = resolve(files::anim_file(cat.id, form, egg_ids, AnimType::Imgcut))?;
            let model = resolve(files::anim_file(cat.id, form, egg_ids, AnimType::Mamodel))?;
            Some((png, cut, model))
        })();

        let conjure_id = cat.stats.get(form)
            .and_then(|s| s.as_ref())
            .map(|s| s.conjure_unit_id)
            .unwrap_or(0);

        let mut secondary_assets = None;
        let mut secondary_id = String::new();

        if conjure_id > 0 {
            let spirit_id = conjure_id as u32;
            secondary_assets = (|| {
                let png = resolve(files::anim_file(spirit_id, 0, (-1, -1), AnimType::Png))?;
                let cut = resolve(files::anim_file(spirit_id, 0, (-1, -1), AnimType::Imgcut))?;
                let model = resolve(files::anim_file(spirit_id, 0, (-1, -1), AnimType::Mamodel))?;
                let atk = resolve(files::maanim_file(spirit_id, 0, (-1, -1), IDX_ATTACK))?;
                Some((png, cut, model, atk))
            })();

            if secondary_assets.is_some() {
                secondary_id = format!("spirit_{}", spirit_id);
            }
        }

        self.primary_id = primary_id.to_string();
        self.secondary_id = secondary_id;
        self.available_anims = available_anims;
        self.primary_assets = primary_assets;
        self.secondary_assets = secondary_assets;
    }

    pub fn sync_enemy(&mut self, enemy: &EnemyEntry, vfs: &Vfs) {
        self.prepare_enemy(enemy, vfs);
        self.load_active();
    }

    fn prepare_enemy(&mut self, enemy: &EnemyEntry, vfs: &Vfs) {
        let primary_id = enemy.id_str();

        if self.primary_id != primary_id {
            self.rescan_enemy_paths(enemy, &primary_id, vfs);
        }

        self.select_valid_index();
    }

    fn rescan_enemy_paths(&mut self, enemy: &EnemyEntry, primary_id: &str, vfs: &Vfs) {
        let resolve = |name: String| -> Option<PathBuf> {
            let iname = format!("i{}", name);
            vfs.find(&[name.as_str(), iname.as_str()])
        };

        let mut available_anims = Vec::new();
        for idx in ENEMY_ANIM_SLOTS {
            let candidate = enemy_files::maanim_file(enemy.id, idx);
            if let Some(resolved) = resolve(candidate) {
                available_anims.push((idx, resolved));
            }
        }

        if let Some(p) = resolve(enemy_files::zombie_maanim_file(enemy.id, 0)) {
            available_anims.push((IDX_BURROW, p));
        }
        if let Some(p) = resolve(enemy_files::zombie_maanim_file(enemy.id, 1)) {
            available_anims.push((IDX_DIG, p));
        }
        if let Some(p) = resolve(enemy_files::zombie_maanim_file(enemy.id, 2)) {
            available_anims.push((IDX_SURFACE, p));
        }

        let primary_assets = (|| {
            let png = resolve(enemy_files::anim_file(enemy.id, EnemyAnimType::Png))?;
            let cut = resolve(enemy_files::anim_file(enemy.id, EnemyAnimType::Imgcut))?;
            let model = resolve(enemy_files::anim_file(enemy.id, EnemyAnimType::Mamodel))?;
            Some((png, cut, model))
        })();

        self.primary_id = primary_id.to_string();
        self.secondary_id = String::new();
        self.available_anims = available_anims;
        self.primary_assets = primary_assets;
        self.secondary_assets = None;
    }

    fn select_valid_index(&mut self) {
        let current_index = self.loaded_anim_index;
        let base_available = self.base_assets_available();
        let secondary_available = self.secondary_available();

        let is_current_valid = if current_index == IDX_NONE {
            false
        } else if current_index == IDX_SPIRIT {
            secondary_available
        } else if current_index == IDX_MODEL {
            base_available
        } else {
            base_available && self.available_anims.iter().any(|(index, _)| *index == current_index)
        };

        if is_current_valid {
            return;
        }

        let mut valid_index = IDX_NONE;

        if base_available {
            for check_index in FALLBACK_PRIORITY {
                if self.available_anims.iter().any(|(index, _)| *index == check_index) {
                    valid_index = check_index;
                    break;
                }
            }
        }

        if valid_index == IDX_NONE && secondary_available {
            valid_index = IDX_SPIRIT;
        }
        if valid_index == IDX_NONE && base_available {
            valid_index = IDX_MODEL;
        }

        self.loaded_anim_index = valid_index;
        if valid_index == IDX_NONE {
            self.held_unit = None;
            self.current_anim = None;
        }
    }

    fn load_active(&mut self) {
        let valid_index = self.loaded_anim_index;

        if valid_index == IDX_NONE {
            return;
        }

        let target_id = if valid_index == IDX_SPIRIT { self.secondary_id.clone() } else { self.primary_id.clone() };
        if target_id.is_empty() {
            return;
        }

        if self.is_loaded(&target_id) {
            self.sync_animation(valid_index);
            return;
        }

        if self.failed_load_id == target_id {
            return;
        }

        if self.apply_cached(&target_id, valid_index) {
            return;
        }

        let (png, cut, model, anim_path) = self.resolve_paths(valid_index);

        let loaded_unit = match (png, cut, model) {
            (Some(png), Some(cut), Some(model)) => {
                match (std::fs::read(png), std::fs::read(cut), std::fs::read(model)) {
                    (Ok(png_bytes), Ok(cut_bytes), Ok(model_bytes)) => Rig::parse(&png_bytes, &cut_bytes, &model_bytes).ok(),
                    _ => None,
                }
            }
            _ => None,
        };

        match loaded_unit {
            Some(unit) => {
                let unit = Arc::new(unit);
                self.cache.insert(&target_id, unit.clone());
                self.held_unit = Some(unit);
                self.loaded_id = target_id;
                self.failed_load_id.clear();
                self.load_anim(valid_index, anim_path);
                self.current_anim_index = valid_index;
            }
            None => {
                self.loaded_id = target_id.clone();
                self.failed_load_id = target_id;
                self.held_unit = None;
                self.current_anim = None;
                self.current_anim_index = IDX_NONE;
            }
        }
    }

    fn needs_animation_reload(loaded_for: usize, target: usize) -> bool {
        loaded_for != target
    }

    fn sync_animation(&mut self, valid_index: usize) {
        if !Self::needs_animation_reload(self.current_anim_index, valid_index) {
            return;
        }

        let (_, _, _, anim_path) = self.resolve_paths(valid_index);
        self.load_anim(valid_index, anim_path);
        self.current_anim_index = valid_index;
    }

    fn load_anim(&mut self, index: usize, anim_path: Option<PathBuf>) {
        if let Some(anim) = self.cache.anim(&self.loaded_id, index) {
            self.current_anim = Some(anim);
            return;
        }

        let parsed = anim_path
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| Animation::parse(&bytes).ok())
            .map(Arc::new);

        if let Some(anim) = &parsed {
            self.cache.store_anim(&self.loaded_id, index, anim.clone());
        }

        self.current_anim = parsed;
    }

    fn resolve_paths(&self, target_index: usize) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
        if target_index == IDX_SPIRIT {
            return self.secondary_assets.as_ref().map_or((None, None, None, None), |(png, cut, model, anim)| {
                (Some(png.clone()), Some(cut.clone()), Some(model.clone()), Some(anim.clone()))
            });
        }

        let anim_path = self.available_anims.iter().find(|(index, _)| *index == target_index).map(|(_, path)| path.clone());
        self.primary_assets
            .as_ref()
            .map_or((None, None, None, None), |(png, cut, model)| (Some(png.clone()), Some(cut.clone()), Some(model.clone()), anim_path))
    }
}

pub struct PreloadRequest {
    target_id: String,
    anim_index: usize,
    png: PathBuf,
    cut: PathBuf,
    model: PathBuf,
    anim: Option<PathBuf>,
}

impl PreloadRequest {
    pub fn run(self) -> PreloadResult {
        let unit = match (std::fs::read(&self.png), std::fs::read(&self.cut), std::fs::read(&self.model)) {
            (Ok(png_bytes), Ok(cut_bytes), Ok(model_bytes)) => Rig::parse(&png_bytes, &cut_bytes, &model_bytes).ok(),
            _ => None,
        };

        if unit.is_none() {
            warn!("Animation preload failed for {}", self.target_id);
        }

        let anim = self.anim
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| Animation::parse(&bytes).ok());

        PreloadResult {
            target_id: self.target_id,
            anim_index: self.anim_index,
            unit: unit.map(Arc::new),
            anim: anim.map(Arc::new),
        }
    }
}

#[derive(Clone)]
pub struct PreloadResult {
    target_id: String,
    anim_index: usize,
    unit: Option<Arc<Rig>>,
    anim: Option<Arc<Animation>>,
}

impl std::fmt::Debug for PreloadResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreloadResult")
            .field("target_id", &self.target_id)
            .field("anim_index", &self.anim_index)
            .field("loaded", &self.unit.is_some())
            .finish()
    }
}

const CACHE_CAP: usize = 12;

#[derive(Default)]
struct RigCache {
    slots: Vec<CacheSlot>,
}

struct CacheSlot {
    id: String,
    unit: Arc<Rig>,
    anims: Vec<(usize, Arc<Animation>)>,
}

impl RigCache {
    fn lookup(&mut self, id: &str) -> Option<Arc<Rig>> {
        let pos = self.slots.iter().position(|slot| slot.id == id)?;
        let slot = self.slots.remove(pos);
        let unit = slot.unit.clone();
        self.slots.push(slot);
        Some(unit)
    }

    fn anim(&self, id: &str, index: usize) -> Option<Arc<Animation>> {
        let slot = self.slots.iter().find(|slot| slot.id == id)?;
        slot.anims.iter().find(|(i, _)| *i == index).map(|(_, anim)| anim.clone())
    }

    fn insert(&mut self, id: &str, unit: Arc<Rig>) {
        if let Some(pos) = self.slots.iter().position(|slot| slot.id == id) {
            let mut slot = self.slots.remove(pos);
            slot.unit = unit;
            self.slots.push(slot);
            return;
        }

        if self.slots.len() >= CACHE_CAP {
            self.slots.remove(0);
        }

        self.slots.push(CacheSlot { id: id.to_string(), unit, anims: Vec::new() });
    }

    fn store_anim(&mut self, id: &str, index: usize, anim: Arc<Animation>) {
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.id == id) else {
            return;
        };

        if !slot.anims.iter().any(|(i, _)| *i == index) {
            slot.anims.push((index, anim));
        }
    }

    fn clear(&mut self) {
        self.slots.clear();
    }
}
