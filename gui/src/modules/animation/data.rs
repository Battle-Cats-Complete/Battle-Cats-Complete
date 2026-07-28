use std::path::{Path, PathBuf};
use std::sync::Arc;

use nyanko::graphics::rig::{Animation, Unit};

use core::modules::animation::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT, IDX_SURFACE,
    IDX_WALK,
};
use core::modules::cat::paths::{self, AnimType};
use core::modules::cat::scanner::CatEntry;
use core::modules::settings::Settings;

const ANIM_SLOTS: [usize; 6] = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
const FALLBACK_PRIORITY: [usize; 6] = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];

type PrimaryAssets = (PathBuf, PathBuf, PathBuf);
type SecondaryAssets = (PathBuf, PathBuf, PathBuf, PathBuf);

pub struct State {
    pub held_unit: Option<Arc<Unit>>,
    pub current_anim: Option<Arc<Animation>>,
    pub loaded_anim_index: usize,
    pub available_anims: Vec<(usize, PathBuf)>,

    primary_id: String,
    secondary_id: String,
    primary_assets: Option<PrimaryAssets>,
    secondary_assets: Option<SecondaryAssets>,

    loaded_id: String,
    failed_load_id: String,
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
        }
    }
}

impl State {
    pub fn base_assets_available(&self) -> bool {
        self.primary_assets.is_some()
    }

    pub fn select(&mut self, index: usize) {
        self.loaded_anim_index = index;
    }

    pub fn secondary_available(&self) -> bool {
        self.secondary_assets.is_some()
    }

    pub fn sync(&mut self, cat: &CatEntry, form: usize, settings: &Settings) {
        let form_char = match form {
            0 => 'f',
            1 => 'c',
            2 => 's',
            _ => 'u',
        };
        let primary_id = format!("{:03}_{}", cat.id, form_char);

        if self.primary_id != primary_id {
            self.rescan_paths(cat, form, &primary_id, settings);
        }

        self.select_valid_index();
        self.load_active(settings);
    }

    fn rescan_paths(&mut self, cat: &CatEntry, form: usize, primary_id: &str, settings: &Settings) {
        let root = Path::new(paths::DIR_CATS);
        let egg_ids = cat.egg_ids.unwrap_or((-1, -1));
        let priority = &settings.general.language_priority;

        let resolve = |path: PathBuf| -> Option<PathBuf> {
            let parent = path.parent()?;
            let name = path.file_name()?.to_str()?;
            core::common::get(parent, [name], priority).into_iter().next()
        };

        let mut available_anims = Vec::new();
        for idx in ANIM_SLOTS {
            let candidate = paths::maanim(root, cat.id, form, egg_ids, idx);
            if let Some(resolved) = resolve(candidate) {
                available_anims.push((idx, resolved));
            }
        }

        let primary_assets = (|| {
            let png = resolve(paths::anim(root, cat.id, form, egg_ids, AnimType::Png))?;
            let cut = resolve(paths::anim(root, cat.id, form, egg_ids, AnimType::Imgcut))?;
            let model = resolve(paths::anim(root, cat.id, form, egg_ids, AnimType::Mamodel))?;
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
                let png = resolve(paths::anim(root, spirit_id, 0, (-1, -1), AnimType::Png))?;
                let cut = resolve(paths::anim(root, spirit_id, 0, (-1, -1), AnimType::Imgcut))?;
                let model = resolve(paths::anim(root, spirit_id, 0, (-1, -1), AnimType::Mamodel))?;
                let atk = resolve(paths::maanim(root, spirit_id, 0, (-1, -1), IDX_ATTACK))?;
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

    fn load_active(&mut self, settings: &Settings) {
        let valid_index = self.loaded_anim_index;

        if valid_index == IDX_NONE {
            return;
        }

        let target_id = if valid_index == IDX_SPIRIT { self.secondary_id.clone() } else { self.primary_id.clone() };
        if target_id.is_empty() {
            return;
        }

        let is_stable = self.loaded_id == target_id;
        let has_failed = self.failed_load_id == target_id;

        if is_stable {
            if has_failed {
                return;
            }
            self.sync_animation(valid_index, settings);
            return;
        }

        let (png, cut, model, anim_path) = self.resolve_paths(valid_index);

        let loaded_unit = match (png, cut, model) {
            (Some(png), Some(cut), Some(model)) => {
                match (std::fs::read(png), std::fs::read(cut), std::fs::read(model)) {
                    (Ok(png_bytes), Ok(cut_bytes), Ok(model_bytes)) => Unit::parse(&png_bytes, &cut_bytes, &model_bytes),
                    _ => None,
                }
            }
            _ => None,
        };

        match loaded_unit {
            Some(unit) => {
                self.held_unit = Some(Arc::new(unit));
                self.loaded_id = target_id;
                self.failed_load_id.clear();
                self.load_anim(anim_path);
            }
            None => {
                self.loaded_id = target_id.clone();
                self.failed_load_id = target_id;
                self.held_unit = None;
                self.current_anim = None;
            }
        }
    }

    fn sync_animation(&mut self, valid_index: usize, _settings: &Settings) {
        let (_, _, _, anim_path) = self.resolve_paths(valid_index);
        let needs_reload = match (&self.current_anim, &anim_path) {
            (None, Some(_)) | (Some(_), None) => true,
            (Some(_), Some(_)) => false,
            (None, None) => false,
        };

        if needs_reload {
            self.load_anim(anim_path);
        }
    }

    fn load_anim(&mut self, anim_path: Option<PathBuf>) {
        let parsed = anim_path
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| Animation::parse(&bytes));

        self.current_anim = parsed.map(Arc::new);
    }

    fn resolve_paths(&self, target_index: usize) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
        if target_index == IDX_SPIRIT {
            if let Some((png, cut, model, anim)) = &self.secondary_assets {
                return (Some(png.clone()), Some(cut.clone()), Some(model.clone()), Some(anim.clone()));
            }
            return (None, None, None, None);
        }

        let anim_path = self.available_anims.iter().find(|(index, _)| *index == target_index).map(|(_, path)| path.clone());
        if let Some((png, cut, model)) = &self.primary_assets {
            return (Some(png.clone()), Some(cut.clone()), Some(model.clone()), anim_path);
        }

        (None, None, None, None)
    }
}
