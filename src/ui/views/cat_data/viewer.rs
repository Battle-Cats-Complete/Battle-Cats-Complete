use eframe::egui;
use std::path::Path;

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::ui::components::anim_viewer::AnimViewer;
use crate::core::settings::Settings; // Import Settings

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &Settings, // Added settings parameter
) {
    // --- ID & Path Generation ---
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let unique_id = format!("{}_{}", id_str, form_char);

    // --- Reset Check ---
    if anim_viewer.loaded_id != unique_id {
        anim_viewer.reset();
        anim_viewer.loaded_id = unique_id.clone();
        *model_data = None; 
        *anim_sheet = SpriteSheet::default(); 
    }

    // --- File Paths ---
    let base_dir = Path::new("game/cats").join(&id_str).join(form_char.to_string()).join("anim");
    let base_name = format!("{}_{}", id_str, form_char);
    
    let png_path = base_dir.join(format!("{}.png", base_name));
    let imgcut_path = base_dir.join(format!("{}.imgcut", base_name));
    let mamodel_path = base_dir.join(format!("{}.mamodel", base_name));
    
    let walk_path = base_dir.join(format!("{}00.maanim", base_name));
    let idle_path = base_dir.join(format!("{}01.maanim", base_name));
    let attack_path = base_dir.join(format!("{}02.maanim", base_name));
    let knockback_path = base_dir.join(format!("{}03.maanim", base_name));

    // --- Loading Logic ---
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
    
    // --- UI Layout ---
    ui.vertical(|ui| {
        ui.add_space(3.0);
        
        // 1. Animation Selection Buttons
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            let btn_size = egui::vec2(80.0, 30.0);

            let anims = [
                ("Walk", 0, &walk_path),
                ("Idle", 1, &idle_path),
                ("Attack", 2, &attack_path),
                ("Knockback", 3, &knockback_path),
            ];

            for (label, index, path) in anims {
                let is_active = anim_viewer.loaded_anim_index == index;
                if ui.add(egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(14.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(btn_size))
                    .clicked() 
                {
                    anim_viewer.load_anim(path);
                    anim_viewer.loaded_anim_index = index;
                }
            }
        });

        ui.add_space(5.0);

        // 2. Playback Controls & Settings
        ui.horizontal(|ui| {
            // Play/Pause Button
            let play_label = if anim_viewer.is_playing { "Pause" } else { "Play" };
            if ui.button(play_label).clicked() {
                anim_viewer.is_playing = !anim_viewer.is_playing;
            }

            // Frame Counter
            if let Some(anim) = &anim_viewer.current_anim {
                ui.label(format!("F: {:.1} / {}", anim_viewer.current_frame, anim.max_frame));
            }

            ui.separator();

            // Center View Button
            if ui.button("Center View").clicked() {
                if let Some(model) = model_data {
                    anim_viewer.center_view(model, anim_sheet);
                }
            }

            // Toggles Removed (Moved to Settings)
        });

        ui.add_space(5.0);

        // 3. Error or Render
        if model_data.is_none() {
            ui.label(format!("Model missing or loading... {:?}", mamodel_path));
            if ui.button("Retry").clicked() { *model_data = None; }
        } else {
            anim_sheet.update(ctx);
            // Pass the model directly + Settings flags
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