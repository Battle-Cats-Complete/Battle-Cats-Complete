use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::ui::components::name_box;
use image::imageops;

// Unified spacing for the input fields
pub const INPUT_SPACING: f32 = 4.0;

pub fn render(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String, 
    magnification: &mut i32, 
    texture_cache: &mut Option<egui::TextureHandle>, 
    current_key: &mut String
) {
    ui.vertical(|ui| {
        // --- 1. Tab Buttons (Exact Cat Replication) ---
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            ui.horizontal(|ui| {
                let tabs = [
                    (EnemyDetailTab::Abilities, "Abilities"), 
                    (EnemyDetailTab::Details, "Details"), 
                    (EnemyDetailTab::Animation, "Animation")
                ];

                for (tab_enum, label) in tabs {
                    let is_selected = *current_tab == tab_enum;
                    let (fill, stroke, text) = if is_selected {
                        (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                    } else {
                        (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                    };

                    // Exact 60px min width button
                    let btn = egui::Button::new(egui::RichText::new(label).color(text))
                        .fill(fill)
                        .stroke(stroke)
                        .rounding(egui::Rounding::from(5.0))
                        .min_size(egui::vec2(60.0, 30.0));

                    if ui.add(btn).clicked() { *current_tab = tab_enum; }
                }
            });
        });

        ui.separator();
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            // Container 110x85 footprint
            let container_size = egui::vec2(110.0, 85.0);
            let (rect, _) = ui.allocate_exact_size(container_size, egui::Sense::hover());
            
            let expected_path = enemy.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            if *current_key != expected_path {
                *current_key = expected_path.clone();
                *texture_cache = if !expected_path.is_empty() { load_icon_texture(ctx, &expected_path) } else { None };
            }

            if let Some(tex) = texture_cache {
                // Horizontal Centering
                let icon_size = egui::vec2(85.0, 85.0);
                let x_off = (container_size.x - icon_size.x) / 2.0;
                let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(x_off, 0.0), icon_size);
                ui.painter().image(tex.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            } else { 
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(30)); 
            }

            ui.add_space(3.0);

            // Info Box (Synced spacing from Cat Header)
            ui.vertical(|ui| {
                ui.set_width(name_box::NAME_BOX_WIDTH);
                let disp_name = if enemy.name.is_empty() { format!("Enemy {:03}", enemy.id) } else { enemy.name.clone() };

                ui.add_space(15.0); 
                name_box::render(ui, &disp_name);
                ui.spacing_mut().item_spacing.y = 0.0;
                
                ui.add_space(10.0);
                ui.label(egui::RichText::new(format!("ID: {:03}", enemy.id)).color(egui::Color32::from_gray(100)).size(12.0));
                
                ui.add_space(3.0); // Exact space as Cat ID -> Row below

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = INPUT_SPACING; 
                    ui.label("Magnification:");
                    if ui.add(egui::TextEdit::singleline(mag_input).desired_width(40.0)).changed() {
                        *magnification = mag_input.trim().parse::<i32>().unwrap_or(100);
                    }
                    ui.label("%");
                });
            });
        });
    });
}

fn load_icon_texture(ctx: &egui::Context, path_str: &str) -> Option<egui::TextureHandle> {
    let path = std::path::Path::new(path_str);
    let img = image::open(path).ok()?;
    let rgba = imageops::resize(&img.to_rgba8(), 85, 85, imageops::FilterType::Lanczos3);
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ctx.load_texture("enemy_detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR))
}