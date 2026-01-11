use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use crate::core::files::skillacquisition::{self, TalentRaw};
use crate::core::files::imgcut::SpriteSheet;
use crate::core::utils::{self, autocrop};
use crate::core::settings::Settings; 

pub fn render(
    ui: &mut egui::Ui,
    talent_data: &TalentRaw,
    sheet: &SpriteSheet,
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    descriptions: Option<&Vec<String>>,
    settings: &Settings, 
) {
    ui.add_space(5.0);
    
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0); 

        for group in &talent_data.groups {
            let bg_color = if group.limit == 1 {
                egui::Color32::from_rgb(120, 20, 20) 
            } else {
                egui::Color32::from_rgb(180, 140, 20) 
            };

            egui::Frame::none()
                .fill(bg_color)
                .rounding(5.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    ui.vertical(|ui| {
                        
                        // --- Header Group (Icon + Name) ---
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;

                            // 1. Icon
                            if let Some(icon_id) = skillacquisition::map_ability_to_icon(group.ability_id) {
                                if let Some(sprite) = sheet.get_sprite_by_line(icon_id) {
                                    ui.add(sprite.fit_to_exact_size(egui::vec2(40.0, 40.0)));
                                } else {
                                    ui.label(egui::RichText::new("?").strong());
                                }
                            } else {
                                ui.label(egui::RichText::new("?").weak());
                            }

                            // 2. Name Image
                            let image_id_to_use = if group.name_id > 0 {
                                group.name_id
                            } else {
                                group.ability_id as i16
                            };

                            if image_id_to_use > 0 {
                                let mut final_file_name = None;
                                let base_dir = "game/assets/Skill_name";

                                // --- LOGIC START ---
                                if !settings.game_language.is_empty() {
                                    // 1. STRICT MODE
                                    let candidate = format!("Skill_name_{:03}_{}.png", image_id_to_use, settings.game_language);
                                    let path_str = format!("{}/{}", base_dir, candidate);
                                    if Path::new(&path_str).exists() {
                                        final_file_name = Some(candidate);
                                    }
                                } else {
                                    // 2. AUTOMATIC MODE
                                    // Iterate shared priority list
                                    for code in utils::LANGUAGE_PRIORITY {
                                        if code.is_empty() { continue; } // Skip base/empty logic for talents, they always have a code

                                        let candidate = format!("Skill_name_{:03}_{}.png", image_id_to_use, code);
                                        let path_str = format!("{}/{}", base_dir, candidate);
                                        if Path::new(&path_str).exists() {
                                            final_file_name = Some(candidate);
                                            break; 
                                        }
                                    }
                                }
                                // --- LOGIC END ---

                                if let Some(file_name) = final_file_name {
                                    if !name_cache.contains_key(&file_name) {
                                        let path_str = format!("{}/{}", base_dir, file_name);
                                        let path = Path::new(&path_str);

                                        if let Ok(img) = image::open(path) {
                                            let rgba = autocrop(img.to_rgba8());
                                            let texture = ui.ctx().load_texture(
                                                &file_name,
                                                egui::ColorImage::from_rgba_unmultiplied(
                                                    [rgba.width() as usize, rgba.height() as usize],
                                                    rgba.as_flat_samples().as_slice()
                                                ),
                                                egui::TextureOptions::LINEAR
                                            );
                                            name_cache.insert(file_name.clone(), texture);
                                        }
                                    }

                                    if let Some(texture) = name_cache.get(&file_name) {
                                        ui.image(&*texture);
                                    } 
                                } 
                            }
                        }); 

                        // --- Description Text ---
                        ui.add_space(6.0);
                        
                        let mut text_to_display = if let Some(desc_list) = descriptions {
                            let tid = group.text_id as usize;
                            if let Some(text) = desc_list.get(tid) {
                                if text.trim().is_empty() {
                                    "No skill description found".to_string()
                                } else {
                                    text.clone()
                                }
                            } else {
                                "No skill description found".to_string()
                            }
                        } else {
                            "No skill description found".to_string()
                        };

                        if !text_to_display.contains('\n') {
                            text_to_display.push('\n');
                        }

                        // Render inside a Darker Sub-Area
                        egui::Frame::none()
                            .fill(egui::Color32::from_black_alpha(100)) 
                            .rounding(4.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(text_to_display).color(egui::Color32::WHITE).size(13.0));
                            });
                    });
                });
        }
    });
}