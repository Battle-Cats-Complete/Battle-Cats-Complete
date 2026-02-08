/*
type: uploaded file
fileName: viewer.rs
*/
use eframe::egui;
use std::path::{Path, PathBuf};

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::ui::components::anim_viewer::AnimViewer;
use crate::core::settings::Settings;
use crate::paths::cat::{self, AnimType};
use crate::ui::components::anim_controls::{
    IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_SPIRIT, IDX_MODEL, IDX_BURROW, IDX_SURFACE
};

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &Settings,
) {
    let root = Path::new(cat::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    // =========================================================
    // 0. PRE-CALCULATE AVAILABILITY (Hoisted)
    // =========================================================
    
    // Animation List
    let mut available_anims = Vec::new();
    let anim_defs = [
        (IDX_WALK, "Walk"), 
        (IDX_IDLE, "Idle"), 
        (IDX_ATTACK, "Attack"), 
        (IDX_KB, "Knockback"),
        (IDX_BURROW, "Burrow"),
        (IDX_SURFACE, "Surface")
    ];
    
    for (idx, label) in anim_defs {
        let path = cat::maanim(root, cat_entry.id, current_form, egg_ids, idx);
        if path.exists() { available_anims.push((idx, label, path)); }
    }

    let std_png = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let std_cut = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let std_model = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    let base_assets_available = std_png.exists() && std_cut.exists() && std_model.exists();

    // Spirit Logic
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        if stats.conjure_unit_id > 0 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut spirit_pack = None;
    let mut spirit_available = false;
    let spirit_sheet_id = if let Some(s_id) = conjure_id { format!("spirit_{}", s_id) } else { String::new() };

    if let Some(s_id) = conjure_id {
        let s_png = cat::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = cat::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = cat::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = cat::maanim(root, s_id, 0, (-1, -1), 2); 

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            spirit_pack = Some((s_png, s_cut, s_model, s_atk));
            spirit_available = true;
        }
    }

    // =========================================================
    // 1. VALIDATE & SANITIZE STATE
    // =========================================================
    
    // This block ensures we NEVER try to load an invalid animation
    let current_idx = anim_viewer.loaded_anim_index;
    let mut valid_idx = current_idx;

    if current_idx == IDX_SPIRIT {
        if !spirit_available { valid_idx = IDX_WALK; }
    } else if current_idx == IDX_MODEL {
        if !base_assets_available { valid_idx = IDX_WALK; }
    } else {
        // For standard animations, check if they exist in the list
        if !available_anims.iter().any(|(i, _, _)| *i == current_idx) {
            // Special case: If Walk itself is missing (unlikely but possible), we shouldn't crash, 
            // but we usually default to Walk (0) as the safe fallback.
            valid_idx = IDX_WALK;
        }
    }

    // APPLY FIX: If the index was invalid, update it immediately.
    // This prevents the "Stuck" state where the UI thinks we are on Spirit 
    // but the loader is trying to load Walk.
    if valid_idx != current_idx {
        anim_viewer.loaded_anim_index = valid_idx;
    }
    
    // =========================================================
    // 2. CALCULATE LOADING STATE
    // =========================================================

    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let form_viewer_id = format!("{}_{}", id_str, form_char);

    let target_viewer_id = if anim_viewer.loaded_anim_index == IDX_SPIRIT {
        spirit_sheet_id.clone()
    } else {
        form_viewer_id.clone()
    };

    let is_stable = anim_viewer.loaded_id == target_viewer_id;
    let is_loading_new = !is_stable && (anim_viewer.staging_model.is_some() || anim_viewer.staging_sheet.is_some());
    let is_first_launch = anim_viewer.held_model.is_none() && model_data.is_none();
    let mut just_swapped = false;

    if is_stable {
        if let Some(m) = model_data {
            anim_viewer.held_model = Some(m.clone());
        }
        anim_viewer.held_sheet = Some((*anim_sheet).clone());
    }

    // A. Start Transition
    // Note: We use valid_idx (which is just anim_viewer.loaded_anim_index) here
    if !is_stable && !is_loading_new && !is_first_launch {
        let (resolved_png, resolved_cut, resolved_model, _) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
        
        if let (Some(png), Some(cut)) = (resolved_png, resolved_cut) {
            let mut new_sheet = SpriteSheet::default();
            new_sheet.load(ctx, png, cut, target_viewer_id.clone());
            anim_viewer.staging_sheet = Some(new_sheet);
        }

        if let Some(model_path) = resolved_model {
            if let Some(loaded_model) = Model::load(model_path) {
                anim_viewer.staging_model = Some(loaded_model);
            }
        }
    }

    // B. First Launch
    if is_first_launch {
        let (resolved_png, resolved_cut, resolved_model, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);

        if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
             anim_sheet.image_data = None; 
             anim_sheet.load(ctx, png, cut, target_viewer_id.clone());
             if let Some(loaded_model) = Model::load(model_path) {
                 anim_viewer.held_model = Some(loaded_model.clone());
                 anim_viewer.held_sheet = Some((*anim_sheet).clone());
                 *model_data = Some(loaded_model);
             }
        }
        
        if let Some(anim_path) = resolved_anim { 
            anim_viewer.load_anim(anim_path); 
        } else { 
            anim_viewer.current_anim = None; 
        }
        
        anim_viewer.loaded_id = target_viewer_id.clone();
        anim_viewer.pending_initial_center = true; 
    }

    // C. Completion
    if is_loading_new {
        if let Some(staging_sheet) = &mut anim_viewer.staging_sheet {
            staging_sheet.update(ctx);

            let texture_is_ready = staging_sheet.sheet_name == target_viewer_id 
                                && !staging_sheet.is_loading_active 
                                && staging_sheet.image_data.is_some();

            if texture_is_ready {
                if let (Some(new_model), Some(new_sheet)) = (anim_viewer.staging_model.take(), anim_viewer.staging_sheet.take()) {
                    anim_viewer.held_model = Some(new_model.clone());
                    anim_viewer.held_sheet = Some(new_sheet.clone());
                    *model_data = Some(new_model);
                    *anim_sheet = new_sheet; 
                    anim_viewer.loaded_id = target_viewer_id.clone();
                    
                    // We can reuse valid_idx here because we sanitized it at the top
                    let (_, _, _, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
                    
                    if let Some(anim_path) = resolved_anim { 
                        anim_viewer.load_anim(anim_path); 
                    } else { 
                        anim_viewer.current_anim = None; 
                    }
                    
                    anim_viewer.pending_initial_center = true;
                    just_swapped = true;
                    ctx.request_repaint();
                }
            }
        }
    } else {
        anim_sheet.update(ctx);
    }

    // =========================================================
    // RENDER LOGIC
    // =========================================================
    
    if anim_viewer.is_expanded {
        // OVERLAY MODE
        egui::Window::new("expanded_anim_viewer")
            .fixed_rect(ctx.screen_rect())
            .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0).shadow(egui::epaint::Shadow::NONE)) 
            .title_bar(false)
            .order(egui::Order::Tooltip) 
            .show(ctx, |ui| {
                let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                ui.put(rect, |ui: &mut egui::Ui| {
                    if let (Some(model_to_draw), Some(sheet_to_draw)) = (anim_viewer.held_model.clone(), anim_viewer.held_sheet.clone()) {
                        let allow_texture_update = !is_loading_new || just_swapped;
                        anim_viewer.render(
                            ui, &sheet_to_draw, &model_to_draw,
                            settings.animation_interpolation, settings.animation_debug, settings.centering_behavior,
                            allow_texture_update,
                            &available_anims,
                            spirit_available,
                            base_assets_available,
                            is_loading_new,
                            &spirit_sheet_id,
                            &form_viewer_id,
                            &spirit_pack,
                        );
                    }
                    ui.allocate_rect(rect, egui::Sense::hover())
                });
            });

        // Placeholder in original spot
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(egui::RichText::new("Animation Expanded").size(16.0).weak());
            if ui.button("Restore View").clicked() {
                anim_viewer.is_expanded = false;
            }
        });

    } else {
        // NORMAL MODE
        ui.vertical(|ui| {
            let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            ui.put(rect, |ui: &mut egui::Ui| {
                if let (Some(model_to_draw), Some(sheet_to_draw)) = (anim_viewer.held_model.clone(), anim_viewer.held_sheet.clone()) {
                    let allow_texture_update = !is_loading_new || just_swapped;
                    anim_viewer.render(
                        ui, &sheet_to_draw, &model_to_draw,
                        settings.animation_interpolation, settings.animation_debug, settings.centering_behavior,
                        allow_texture_update,
                        &available_anims,
                        spirit_available,
                        base_assets_available,
                        is_loading_new,
                        &spirit_sheet_id,
                        &form_viewer_id,
                        &spirit_pack,
                    );
                }
                ui.allocate_rect(rect, egui::Sense::hover())
            });
        });
    }
}

fn resolve_paths<'a>(
    idx: usize,
    png_path_base: &'a PathBuf,
    cut_path_base: &'a PathBuf,
    model_path_base: &'a PathBuf,
    spirit_pack: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    anims: &'a Vec<(usize, &str, PathBuf)>
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    
    if idx == IDX_SPIRIT {
        if let Some((s_png, s_cut, s_model, s_anim)) = spirit_pack {
            return (Some(s_png), Some(s_cut), Some(s_model), Some(s_anim));
        }
    } else {
        let anim_path = anims.iter().find(|(i, _, _)| *i == idx).map(|(_, _, p)| p);
        return (Some(png_path_base), Some(cut_path_base), Some(model_path_base), anim_path);
    }
    (None, None, None, None)
}