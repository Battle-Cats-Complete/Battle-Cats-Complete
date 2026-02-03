use eframe::egui;
use std::collections::HashMap;
use std::path::Path;

use crate::core::cat::scanner::CatEntry;
use crate::core::cat::DetailTab;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::img015;
use crate::core::settings::Settings;
use crate::core::cat::talents as talent_logic; 
use crate::data::global::mamodel::Model;
use crate::ui::views::cat_data::anim::AnimViewer;

mod header;
mod stats;
mod abilities;
mod talents;
mod details;
pub mod anim;
pub mod list;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat_entry: &CatEntry, 
    current_form: &mut usize,
    current_tab: &mut DetailTab, 
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    
    icon_sheet: &mut SpriteSheet,   
    anim_sheet: &mut SpriteSheet,   
    
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,

    multihit_texture: &mut Option<egui::TextureHandle>,
    kamikaze_texture: &mut Option<egui::TextureHandle>,
    boss_wave_immune_texture: &mut Option<egui::TextureHandle>,
    talent_name_cache: &mut HashMap<String, egui::TextureHandle>,
    gatya_item_textures: &mut HashMap<i32, Option<egui::TextureHandle>>,
    skill_descriptions: Option<&Vec<String>>, 
    settings: &Settings, 
    talent_levels: &mut HashMap<u8, u8>,
    cache_version: u64,
) {
    img015::ensure_loaded(ctx, icon_sheet, settings);

    header::render(
        ctx, ui, cat_entry, current_form, current_tab, current_level, level_input, texture_cache, current_key, settings
    );

    ui.separator(); 
    ui.add_space(0.0);

    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture("multihit_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }

    let base_stats = cat_entry.stats.get(*current_form).and_then(|opt| opt.as_ref());
    let form_allows_talents = *current_form >= 2;

    let patched_stats_owned = if form_allows_talents {
        if let (Some(base), Some(t_data)) = (base_stats, &cat_entry.talent_data) {
            Some(talent_logic::apply_talent_stats(base, t_data, talent_levels))
        } else { None }
    } else { None };
    let current_stats = patched_stats_owned.as_ref().or(base_stats);

    match current_tab {
        DetailTab::Abilities => {
            if let Some(s) = current_stats {
                stats::render(ui, cat_entry, s, *current_form, *current_level);
                ui.spacing_mut().item_spacing.y = 7.0;
                ui.separator(); 
            }
             egui::ScrollArea::vertical()
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                     if let Some(s) = current_stats {
                        abilities::render(
                            ui, s, cat_entry, *current_level, icon_sheet, 
                            multihit_texture, kamikaze_texture, boss_wave_immune_texture, 
                            settings, 
                            if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
                            if form_allows_talents { Some(&*talent_levels) } else { None }
                        );
                     }
                });
        },
        DetailTab::Talents => {
             if let Some(raw) = &cat_entry.talent_data {
                talents::render(ui, raw, icon_sheet, talent_name_cache, skill_descriptions, settings, base_stats, cat_entry.curve.as_ref(), *current_level, talent_levels, cat_entry.id);
             }
        },
        DetailTab::Details => {
             let fallback = Vec::new();
             let desc = cat_entry.description.get(*current_form).unwrap_or(&fallback);
             details::render(ui, desc);
             let text_fallback = Vec::new();
             let ev_text = cat_entry.evolve_text.get(*current_form).unwrap_or(&text_fallback);
             details::render_evolve(ui, ctx, &cat_entry.unit_buy, ev_text, *current_form, gatya_item_textures, cache_version);
        },
        DetailTab::Animation => {
            // --- ID GENERATION ---
            let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
            let id_str = format!("{:03}", cat_entry.id);
            let unique_id = format!("{}_{}", id_str, form_char);

            // --- RESET CHECK (Fix for Crash) ---
            if anim_viewer.loaded_id != unique_id {
                anim_viewer.reset();
                anim_viewer.loaded_id = unique_id.clone();
                *model_data = None; 
                *anim_sheet = SpriteSheet::default(); 
            }

            // --- PATHS ---
            let base_dir = Path::new("game/cats").join(&id_str).join(form_char.to_string()).join("anim");
            let base_name = format!("{}_{}", id_str, form_char);
            
            let png_path = base_dir.join(format!("{}.png", base_name));
            let imgcut_path = base_dir.join(format!("{}.imgcut", base_name));
            let mamodel_path = base_dir.join(format!("{}.mamodel", base_name));
            
            let walk_path = base_dir.join(format!("{}00.maanim", base_name));
            let idle_path = base_dir.join(format!("{}01.maanim", base_name));
            let attack_path = base_dir.join(format!("{}02.maanim", base_name));
            let knockback_path = base_dir.join(format!("{}03.maanim", base_name));

            if model_data.is_none() && mamodel_path.exists() {
                 if let Some(m) = Model::load(&mamodel_path) {
                     *model_data = Some(m);
                 }
            }

            if !anim_sheet.is_loading_active && !anim_sheet.is_ready() {
                 if png_path.exists() && imgcut_path.exists() {
                     anim_sheet.load(ctx, &png_path, &imgcut_path, unique_id.clone());
                 }
            }
            
            if anim_viewer.current_anim.is_none() {
                if walk_path.exists() {
                    anim_viewer.load_anim(&walk_path);
                    anim_viewer.loaded_anim_index = 0;
                } else if attack_path.exists() {
                    anim_viewer.load_anim(&attack_path);
                    anim_viewer.loaded_anim_index = 2;
                }
            }
            
            ui.vertical(|ui| {
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Animations:");
                    if ui.selectable_label(anim_viewer.loaded_anim_index == 0, "Walk").clicked() {
                        anim_viewer.load_anim(&walk_path);
                        anim_viewer.loaded_anim_index = 0;
                    }
                    if ui.selectable_label(anim_viewer.loaded_anim_index == 1, "Idle").clicked() {
                        anim_viewer.load_anim(&idle_path);
                        anim_viewer.loaded_anim_index = 1;
                    }
                    if ui.selectable_label(anim_viewer.loaded_anim_index == 2, "Attack").clicked() {
                        anim_viewer.load_anim(&attack_path);
                        anim_viewer.loaded_anim_index = 2;
                    }
                    if ui.selectable_label(anim_viewer.loaded_anim_index == 3, "KB").clicked() {
                        anim_viewer.load_anim(&knockback_path);
                        anim_viewer.loaded_anim_index = 3;
                    }
                });

                if model_data.is_none() {
                    ui.label(format!("Model missing or loading... {:?}", mamodel_path));
                    if ui.button("Retry").clicked() { *model_data = None; }
                } else {
                    anim_sheet.update(ctx);
                    anim_viewer.render(ui, anim_sheet, model_data.as_ref().unwrap());
                }
            });
        }
    }
}