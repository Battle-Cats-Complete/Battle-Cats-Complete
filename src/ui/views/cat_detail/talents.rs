use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use crate::core::files::skillacquisition::{self, TalentRaw};
use crate::core::files::imgcut::SpriteSheet;
use crate::core::utils::autocrop;

pub fn render(
    ui: &mut egui::Ui,
    talent_data: &TalentRaw,
    sheet: &SpriteSheet,
    name_cache: &mut HashMap<String, egui::TextureHandle>,
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

                    // Main Row Layout
                    ui.horizontal(|ui| {
                        
                        // --- Header Group (Icon + Name) ---
                        // Wrapped in its own scope to keep them tightly bound top-left
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
                                let file_name = format!("Skill_name_{:03}.png", image_id_to_use);
                                
                                if !name_cache.contains_key(&file_name) {
                                    let path_str = format!("game/assets/Skill_name/{}", file_name);
                                    let path = Path::new(&path_str);

                                    if path.exists() {
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
                                }

                                if let Some(texture) = name_cache.get(&file_name) {
                                    ui.image(&*texture);
                                } 
                            }
                        }); // End Header Group
                        
                        // Space for future content (Cost, Description, etc.) can go here
                    });
                });
        }
    });
}