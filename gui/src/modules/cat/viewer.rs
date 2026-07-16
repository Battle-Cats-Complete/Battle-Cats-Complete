use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use nyanko::graphics::rig::Unit;

use core::modules::animation::logic::constants::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB,
    IDX_SURFACE, IDX_WALK,
};
use core::modules::cat::logic::scanner::CatEntry;
use core::modules::cat::paths::{self, AnimType};
use core::modules::settings::logic::state::Settings;

use crate::modules::animation::viewer::AnimViewer;
use crate::common::shared::DragGuard;

thread_local! {
    static PATH_CACHE: RefCell<(String, String, Vec<(usize, PathBuf)>, Option<(PathBuf, PathBuf, PathBuf)>, Option<(PathBuf, PathBuf, PathBuf, PathBuf)>)> = Default::default();
}

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    unit_sync: &mut Option<Arc<Unit>>,
    settings: &mut Settings,
    drag_guard: &mut DragGuard,
) {
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let primary_id = format!("{:03}_{}_{}", cat_entry.id, form_char, anim_viewer.texture_version);

    PATH_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();

        if c.0 != primary_id {
            let root = Path::new(paths::DIR_CATS);
            let egg_ids = cat_entry.egg_ids.unwrap_or((-1, -1));
            let priority = &settings.general.language_priority;

            let mut available_anims = Vec::new();
            let anim_defs = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];

            for idx in anim_defs {
                let p = paths::maanim(root, cat_entry.id, current_form, egg_ids, idx);
                let Some(parent) = p.parent() else { continue; };
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else { continue; };

                if let Some(resolved) = core::common::get(parent, [name], priority).into_iter().next() {
                    available_anims.push((idx, resolved));
                }
            }

            let resolve = |p: PathBuf| {
                let parent = p.parent()?;
                let name = p.file_name()?.to_str()?;
                core::common::get(parent, [name], priority).into_iter().next()
            };

            let primary_assets = (|| {
                let png = resolve(paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png))?;
                let cut = resolve(paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut))?;
                let model = resolve(paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel))?;
                Some((png, cut, model))
            })();

            let mut secondary_assets = None;
            let mut secondary_id = String::new();

            let conjure_id = cat_entry.stats.get(current_form)
                .and_then(|s| s.as_ref())
                .map(|s| s.conjure_unit_id)
                .unwrap_or(0);

            if conjure_id > 0 {
                let s_id = conjure_id as u32;
                secondary_assets = (|| {
                    let png = resolve(paths::anim(root, s_id, 0, (-1, -1), AnimType::Png))?;
                    let cut = resolve(paths::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut))?;
                    let model = resolve(paths::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel))?;
                    let atk = resolve(paths::maanim(root, s_id, 0, (-1, -1), IDX_ATTACK))?;

                    Some((png, cut, model, atk))
                })();

                if secondary_assets.is_some() {
                    secondary_id = format!("spirit_{}_{}", s_id, anim_viewer.texture_version);
                }
            }

            c.0 = primary_id.clone();
            c.1 = secondary_id;
            c.2 = available_anims;
            c.3 = primary_assets;
            c.4 = secondary_assets;
        }

        anim_viewer.show(ui, ctx, &c.0, &c.1, &c.2, c.3.clone(), c.4.clone(), unit_sync, settings, drag_guard);
    });
}