use eframe::egui;
use std::path::{Path, PathBuf};

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
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
    // 1. DISCOVERY PHASE: What does this unit actually have?
    // =========================================================
    
    let root = Path::new(cat::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let unit_base_id = format!("{}_{}", id_str, form_char);

    // A. Check Standard Unit Assets
    let p_png = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let p_cut = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let p_model = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    let base_assets_available = p_png.exists() && p_cut.exists() && p_model.exists();

    // B. Check Standard Animations (Walk, Idle, etc.)
    // We map existing files to their Indices
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

    // C. Check Spirit (Conjure) Assets
    // Get ID from stats (if valid)
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        // unitid.rs returns valid ID or -1 (or 0 if unset in some versions, but usually checked via > -1)
        if stats.conjure_unit_id > -1 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut spirit_pack = None;
    let mut spirit_available = false;

    if let Some(s_id) = conjure_id {
        // Spirits are separate units (Form 0), no eggs.
        let s_png = cat::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = cat::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = cat::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = cat::maanim(root, s_id, 0, (-1, -1), 2); // Spirit Attack is Index 2

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            spirit_pack = Some((s_png, s_cut, s_model, s_atk));
            spirit_available = true;
        }
    }

    // =========================================================
    // 2. VALIDATION PHASE: Reconcile Intent with Reality
    // =========================================================

    // What does the viewer *currently* want to show?
    let current_intent_idx = anim_viewer.loaded_anim_index;
    
    // Is this intent valid for the NEW/CURRENT unit context?
    let mut is_intent_valid = false;
    
    if current_intent_idx == IDX_SPIRIT {
        is_intent_valid = spirit_available;
    } else if current_intent_idx == IDX_MODEL {
        is_intent_valid = base_assets_available;
    } else {
        // Check if the specific animation index exists in our available list
        is_intent_valid = available_anims.iter().any(|(i, _, _)| *i == current_intent_idx);
    }

    // Calculate the Final Index (Persist if valid, Fallback if invalid)
    let final_anim_index = if is_intent_valid {
        current_intent_idx
    } else {
        // Fallback Priority: Walk -> Idle -> Attack -> Spirit -> Model
        if let Some((first_idx, _, _)) = available_anims.first() {
            *first_idx
        } else if spirit_available {
            IDX_SPIRIT
        } else {
            IDX_MODEL
        }
    };

    // Calculate the Target Texture ID (The "loaded_id" string)
    // This determines if we are looking at the Unit or the Spirit
    let target_loaded_id = if final_anim_index == IDX_SPIRIT {
        format!("spirit_{}", conjure_id.unwrap_or(0))
    } else {
        unit_base_id.clone()
    };

    // =========================================================
    // 3. EXECUTION PHASE: Apply State Changes
    // =========================================================

    let id_changed = anim_viewer.loaded_id != target_loaded_id;
    let index_changed = anim_viewer.loaded_anim_index != final_anim_index;

    // A. Handle Context Switch (Unit Change OR Mode Change)
    if id_changed {
        // 1. Reset Viewer (Clear textures/models)
        anim_viewer.reset();
        anim_viewer.loaded_id = target_loaded_id.clone();
        anim_viewer.loaded_anim_index = final_anim_index;
        *model_data = None;
        *anim_sheet = SpriteSheet::default();

        // 2. IMMEDIATE ANIMATION LOAD
        // We load the animation *now* so there is no frame where Model exists but Animation doesn't.
        if final_anim_index != IDX_MODEL {
            let path_to_load = if final_anim_index == IDX_SPIRIT {
                spirit_pack.as_ref().map(|(_, _, _, a)| a)
            } else {
                available_anims.iter().find(|(i, _, _)| *i == final_anim_index).map(|(_, _, p)| p)
            };

            if let Some(p) = path_to_load {
                anim_viewer.load_anim(p);
            }
        } else {
            anim_viewer.current_anim = None;
        }
    } 
    // B. Handle Index Correction (Fallback applied without ID change)
    else if index_changed {
        anim_viewer.loaded_anim_index = final_anim_index;
        anim_viewer.current_anim = None; // Force reload on next logic pass
    }

    // =========================================================
    // 4. RESOURCE LOADING PHASE (Fill in the gaps)
    // =========================================================

    // Load Textures (Sprite/Cuts)
    if !anim_sheet.is_loading_active && !anim_sheet.is_ready() {
        if final_anim_index == IDX_SPIRIT {
            if let Some((s_png, s_cut, _, _)) = &spirit_pack {
                anim_sheet.load(ctx, s_png, s_cut, target_loaded_id.clone());
            }
        } else if base_assets_available {
            anim_sheet.load(ctx, &p_png, &p_cut, target_loaded_id.clone());
        }
    }

    // Load Model Structure
    if model_data.is_none() {
        let path = if final_anim_index == IDX_SPIRIT {
            spirit_pack.as_ref().map(|(_, _, m, _)| m)
        } else if base_assets_available {
            Some(&p_model)
        } else { None };

        if let Some(p) = path {
            if let Some(m) = Model::load(p) {
                *model_data = Some(m);
            }
        }
    }

    // Load Animation (If missing, e.g. after a button click reset)
    if anim_viewer.current_anim.is_none() && final_anim_index != IDX_MODEL {
        let path = if final_anim_index == IDX_SPIRIT {
            spirit_pack.as_ref().map(|(_, _, _, a)| a)
        } else {
            available_anims.iter().find(|(i, _, _)| *i == final_anim_index).map(|(_, _, p)| p)
        };

        if let Some(p) = path {
            anim_viewer.load_anim(p);
        }
    }

    // =========================================================
    // 5. RENDER PHASE: Draw UI
    // =========================================================
    
    ui.vertical(|ui| {
        ui.add_space(3.0);
        
        // --- Buttons ---
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            let btn_size = egui::vec2(70.0, 25.0);

            // 1. Standard Animations
            for (idx, label, path) in &available_anims {
                let is_active = anim_viewer.loaded_anim_index == *idx;
                if ui.add(egui::Button::new(egui::RichText::new(*label).color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked() 
                {
                    // MANUAL SWITCH:
                    // We update the index and load IMMEDIATELY to prevent flashing.
                    // The ID check logic in Phase 3 will handle context switching on the next frame if needed,
                    // but for simple anim switching, we handle it here.
                    anim_viewer.loaded_anim_index = *idx;
                    anim_viewer.load_anim(path);
                }
            }

            // 2. Spirit Button
            if spirit_available {
                let is_active = anim_viewer.loaded_anim_index == IDX_SPIRIT;
                if ui.add(egui::Button::new(egui::RichText::new("Spirit").color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked()
                {
                    // Switching to Spirit implies a Context (ID) change next frame.
                    // We set the index, and let Phase 3 (Validation/Execution) handle the texture reload next frame.
                    anim_viewer.loaded_anim_index = IDX_SPIRIT;
                    // We can't load the anim yet because we don't have the Spirit Model loaded (ID mismatch).
                    // We let the loop handle the reset.
                }
            }

            // 3. Model Button
            if base_assets_available {
                let is_active = anim_viewer.loaded_anim_index == IDX_MODEL;
                if ui.add(egui::Button::new(egui::RichText::new("Model").color(egui::Color32::WHITE).size(13.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked() 
                {
                    anim_viewer.loaded_anim_index = IDX_MODEL;
                    anim_viewer.current_anim = None;
                }
            }
        });

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
        if model_data.is_none() {
             ui.label("Loading...");
        } else {
            anim_sheet.update(ctx);
            anim_viewer.render(
                ui, 
                anim_sheet, 
                model_data.as_ref().unwrap(),
                settings.animation_interpolation,
                settings.animation_debug
            );
        }
    });
}