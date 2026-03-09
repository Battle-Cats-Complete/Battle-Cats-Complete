use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::features::settings::logic::Settings;
use crate::global::imgcut::SpriteSheet;
use crate::global::img015;
use crate::global::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;

use super::{header, stats, abilities, details, viewer}; 

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy_entry: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String,
    magnification: &mut i32,
    settings: &mut Settings,
    icon_sheet: &mut SpriteSheet,
    anim_sheet: &mut SpriteSheet,
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,
    multihit_texture: &mut Option<egui::TextureHandle>,
    kamikaze_texture: &mut Option<egui::TextureHandle>,
    base_texture: &mut Option<egui::TextureHandle>,
    starred_alien_texture: &mut Option<egui::TextureHandle>,
    burrow_texture: &mut Option<egui::TextureHandle>,
    revive_texture: &mut Option<egui::TextureHandle>,
    detail_texture: &mut Option<egui::TextureHandle>,
    detail_key: &mut String,
) {
    img015::ensure_loaded(ctx, icon_sheet, settings);
    
    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture("multihit_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if kamikaze_texture.is_none() {
        const KAMIKAZE_BYTES: &[u8] = include_bytes!("../../../assets/kamikaze.png");
        if let Ok(img) = image::load_from_memory(KAMIKAZE_BYTES) {
            let rgba = img.to_rgba8();
            *kamikaze_texture = Some(ctx.load_texture("kamikaze_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if base_texture.is_none() {
        const BASE_BYTES: &[u8] = include_bytes!("../../../assets/base.png");
        if let Ok(img) = image::load_from_memory(BASE_BYTES) {
            let rgba = img.to_rgba8();
            *base_texture = Some(ctx.load_texture("base_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if starred_alien_texture.is_none() {
        const STARRED_ALIEN_BYTES: &[u8] = include_bytes!("../../../assets/starred_alien.png");
        if let Ok(img) = image::load_from_memory(STARRED_ALIEN_BYTES) {
            let rgba = img.to_rgba8();
            *starred_alien_texture = Some(ctx.load_texture("starred_alien_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if burrow_texture.is_none() {
        const BURROW_BYTES: &[u8] = include_bytes!("../../../assets/burrow.png");
        if let Ok(img) = image::load_from_memory(BURROW_BYTES) {
            let rgba = img.to_rgba8();
            *burrow_texture = Some(ctx.load_texture("burrow_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if revive_texture.is_none() {
        const REVIVE_BYTES: &[u8] = include_bytes!("../../../assets/revive.png");
        if let Ok(img) = image::load_from_memory(REVIVE_BYTES) {
            let rgba = img.to_rgba8();
            *revive_texture = Some(ctx.load_texture("revive_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }

    header::render(
        ctx,
        ui,
        enemy_entry,
        current_tab,
        mag_input,
        magnification,
    detail_texture,
    detail_key,
);

    ui.separator(); 
    ui.add_space(0.0);

    match current_tab {
        EnemyDetailTab::Abilities => {
            stats::render(ui, enemy_entry, *magnification);
            ui.spacing_mut().item_spacing.y = 7.0;
            ui.separator();
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                    abilities::render(
                        ui, 
                        enemy_entry, 
                        icon_sheet, 
                        multihit_texture, 
                        kamikaze_texture, 
                        base_texture, 
                        starred_alien_texture, 
                        burrow_texture, 
                        revive_texture, 
                        settings,
                        *magnification
                    );
                });
        },
        EnemyDetailTab::Details => {
            details::render(ui, &enemy_entry.description);
        },
        EnemyDetailTab::Animation => {
            viewer::show(ui, ctx, enemy_entry, anim_viewer, model_data, anim_sheet, settings);
        }
    }
}