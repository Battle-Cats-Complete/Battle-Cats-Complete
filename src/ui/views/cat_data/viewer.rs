use eframe::egui;
use std::path::{Path, PathBuf};

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::ui::components::anim_viewer::AnimViewer;
use crate::core::settings::Settings;
use crate::paths::cat::{self, AnimType};

// --- Constants ---
const IDX_WALK: usize = 0;
const IDX_IDLE: usize = 1;
const IDX_ATTACK: usize = 2;
const IDX_KB: usize = 3;
const IDX_SPIRIT: usize = 4;
const IDX_MODEL: usize = 99;

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
    // =========================================================
    // 1. SETUP & PATHS
    // =========================================================
    
    let root = Path::new(cat::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    // Stable Parent ID (Used for AnimViewer state to prevent camera resets)
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let parent_viewer_id = format!("{}_{}", id_str, form_char);

    // -- Standard Unit Paths --
    let p_png = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let p_cut = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let p_model = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    let base_assets_available = p_png.exists() && p_cut.exists() && p_model.exists();

    // -- Spirit Paths --
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        if stats.conjure_unit_id > 0 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut spirit_pack = None;
    let mut spirit_available = false;
    // Unique ID for Spirit Textures (Ensures correct sprite loading)
    let mut spirit_sheet_id = String::new();

    if let Some(s_id) = conjure_id {
        let s_png = cat::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = cat::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = cat::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = cat::maanim(root, s_id, 0, (-1, -1), 2); 

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            spirit_pack = Some((s_png, s_cut, s_model, s_atk));
            spirit_available = true;
            spirit_sheet_id = format!("spirit_{}", s_id);
        }
    }

    // -- Animation Paths --
    let mut available_anims = Vec::new();
    let anim_defs = [
        (IDX_WALK, "Walk"), 
        (IDX_IDLE, "Idle"), 
        (IDX_ATTACK, "Attack"), 
        (IDX_KB, "Knockback") 
    ];
    
    for (idx, label) in anim_defs {
        let path = cat::maanim(root, cat_entry.id, current_form, egg_ids, idx);
        if path.exists() {
            available_anims.push((idx, label, path));
        }
    }

    // =========================================================
    // 2. STATE VALIDATION (Fixes "Revert to Walk" bug)
    // =========================================================

    let current_idx = anim_viewer.loaded_anim_index;
    let mut valid_idx = current_idx;
    let mut needs_fallback = false;

    if current_idx == IDX_SPIRIT {
        if !spirit_available { needs_fallback = true; }
    } else if current_idx == IDX_MODEL {
        if !base_assets_available { needs_fallback = true; }
    } else {
        if !available_anims.iter().any(|(i, _, _)| *i == current_idx) { needs_fallback = true; }
    }

    if needs_fallback {
        if let Some((first, _, _)) = available_anims.first() {
            valid_idx = *first;
        } else if spirit_available {
            valid_idx = IDX_SPIRIT;
        } else {
            valid_idx = IDX_MODEL;
        }
        // Sync UI immediately
        anim_viewer.loaded_anim_index = valid_idx;
    }

    // =========================================================
    // 3. CONTEXT SWITCHING (Unit A <-> Unit B)
    // =========================================================
    
    // We strictly enforce that the viewer ID matches the PARENT Unit ID.
    // This allows us to switch to Spirit (same parent) without changing viewer ID.
    if anim_viewer.loaded_id != parent_viewer_id {
        // New Unit Context -> Full Reset
        anim_viewer.reset();
        anim_viewer.loaded_id = parent_viewer_id.clone();
        *model_data = None;
        *anim_sheet = SpriteSheet::default();
        anim_viewer.loaded_anim_index = valid_idx;

        // Determine which assets to load based on the valid index
        let (png, cut, model_p, anim_p) = resolve_paths(valid_idx, &p_png, &p_cut, &p_model, &spirit_pack, &available_anims);
        
        // Determine the Sheet ID (Unique for Spirit vs Unit)
        let sheet_id = if valid_idx == IDX_SPIRIT { spirit_sheet_id.clone() } else { parent_viewer_id.clone() };

        if let (Some(p), Some(c), Some(m_path)) = (png, cut, model_p) {
             anim_sheet.load(ctx, p, c, sheet_id);
             if let Some(m) = Model::load(m_path) {
                 *model_data = Some(m);
             }
        }
        
        if let Some(a_path) = anim_p {
            anim_viewer.load_anim(a_path);
        } else {
            anim_viewer.current_anim = None;
        }
    }

    // Recovery for dropped textures (e.g. initial app load)
    if !anim_sheet.is_loading_active && !anim_sheet.is_ready() {
        let (png, cut, _, _) = resolve_paths(anim_viewer.loaded_anim_index, &p_png, &p_cut, &p_model, &spirit_pack, &available_anims);
        let sheet_id = if anim_viewer.loaded_anim_index == IDX_SPIRIT { spirit_sheet_id.clone() } else { parent_viewer_id.clone() };
        
        if let (Some(p), Some(c)) = (png, cut) {
            anim_sheet.load(ctx, p, c, sheet_id);
        }
    }

    // =========================================================
    // 4. UI RENDER PHASE
    // =========================================================
    
    let mut clicked_index: Option<usize> = None;

    ui.vertical(|ui| {
        ui.add_space(3.0);
        
        // --- Buttons ---
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            let btn_size = egui::vec2(70.0, 25.0);

            for (idx, label, _) in &available_anims {
                let is_active = anim_viewer.loaded_anim_index == *idx;
                if ui.add(egui::Button::new(egui::RichText::new(*label).color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked() 
                {
                    clicked_index = Some(*idx);
                }
            }

            if spirit_available {
                let is_active = anim_viewer.loaded_anim_index == IDX_SPIRIT;
                if ui.add(egui::Button::new(egui::RichText::new("Spirit").color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked()
                {
                    clicked_index = Some(IDX_SPIRIT);
                }
            }

            if base_assets_available {
                let is_active = anim_viewer.loaded_anim_index == IDX_MODEL;
                if ui.add(egui::Button::new(egui::RichText::new("Model").color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked() 
                {
                    clicked_index = Some(IDX_MODEL);
                }
            }
        });

        // =========================================================
        // 5. EVENT PROCESSING (Mode Switching)
        // =========================================================
        
        if let Some(target_idx) = clicked_index {
            let old_idx = anim_viewer.loaded_anim_index;
            
            // 1. Resolve Paths for Target
            let (t_png, t_cut, t_model_p, t_anim_p) = resolve_paths(target_idx, &p_png, &p_cut, &p_model, &spirit_pack, &available_anims);
            
            // 2. Load Data (RAM)
            let new_model = if let Some(mp) = t_model_p { Model::load(mp) } else { None };
            let new_anim = if let Some(ap) = t_anim_p { Animation::load(ap) } else { None };

            // 3. Detect Mode Switch (Unit <-> Spirit)
            let old_is_spirit = old_idx == IDX_SPIRIT;
            let new_is_spirit = target_idx == IDX_SPIRIT;
            let mode_changed = old_is_spirit != new_is_spirit;

            // 4. Apply Changes
            if t_model_p.is_none() || new_model.is_some() {
                
                // If switching modes, we MUST reload the sprite sheet with the correct ID.
                if mode_changed {
                    let sheet_id = if new_is_spirit { spirit_sheet_id.clone() } else { parent_viewer_id.clone() };
                    
                    if let (Some(png), Some(cut)) = (t_png, t_cut) {
                        *anim_sheet = SpriteSheet::default(); // Wipe old texture
                        anim_sheet.load(ctx, png, cut, sheet_id); // Load new texture with UNIQUE ID
                    }
                }

                // Update Index (Viewer ID stays constant "parent_viewer_id" to prevent camera reset)
                anim_viewer.loaded_anim_index = target_idx;
                
                if let Some(m) = new_model {
                    *model_data = Some(m);
                }
                
                if let Some(a) = new_anim {
                    anim_viewer.current_anim = Some(a);
                    anim_viewer.current_frame = 0.0;
                } else if target_idx == IDX_MODEL {
                    anim_viewer.current_anim = None;
                }
            }
        }

        ui.add_space(5.0);

        // --- Controls ---
        ui.horizontal(|ui| {
            let play_label = if anim_viewer.is_playing { "Pause" } else { "Play" };
            if ui.button(play_label).clicked() {
                anim_viewer.is_playing = !anim_viewer.is_playing;
            }

            if let Some(anim) = &anim_viewer.current_anim {
                ui.label(format!("F: {:.1} / {}", anim_viewer.current_frame, anim.max_frame));
            } else {
                 ui.label("Base Model");
            }

            ui.separator();

            if ui.button("Center View").clicked() {
                if let Some(model) = model_data {
                    anim_viewer.center_view(model, anim_sheet);
                }
            }
        });

        ui.add_space(5.0);

        // --- Viewport ---
        if model_data.is_some() {
            let safe_to_render = anim_viewer.loaded_anim_index == IDX_MODEL || anim_viewer.current_anim.is_some();
            
            if safe_to_render {
                anim_sheet.update(ctx);
                anim_viewer.render(
                    ui, 
                    anim_sheet, 
                    model_data.as_ref().unwrap(),
                    settings.animation_interpolation,
                    settings.animation_debug
                );
            } else {
                ui.allocate_space(ui.available_size());
            }
        } else {
             ui.allocate_space(ui.available_size());
        }
    });
}

// --- Helper: Path Resolution ---
fn resolve_paths<'a>(
    idx: usize,
    p_png: &'a PathBuf,
    p_cut: &'a PathBuf,
    p_model: &'a PathBuf,
    spirit_pack: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    anims: &'a Vec<(usize, &str, PathBuf)>
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    
    if idx == IDX_SPIRIT {
        if let Some((s_png, s_cut, s_model, s_anim)) = spirit_pack {
            return (Some(s_png), Some(s_cut), Some(s_model), Some(s_anim));
        }
    } else {
        // Standard Unit
        let anim_path = anims.iter().find(|(i, _, _)| *i == idx).map(|(_, _, p)| p);
        return (Some(p_png), Some(p_cut), Some(p_model), anim_path);
    }
    (None, None, None, None)
}