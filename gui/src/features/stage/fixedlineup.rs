use std::collections::HashMap;
use std::path::Path;

use eframe::egui;

use core::global::utils::autocrop;
use core::stage::logic::fixedlineup::{ResolvedFixedLineup, ResolvedSlot};

const ICON_SCALE: f32 = 0.45;
const ICON_SPACING: f32 = 8.0;

fn load_cat_icon(path: &Path) -> Option<egui::ColorImage> {
    let Ok(image_file) = image::open(path) else { return None; };
    let cropped_image = autocrop(image_file.to_rgba8());
    let dimensions = [cropped_image.width() as usize, cropped_image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(dimensions, cropped_image.as_flat_samples().as_slice()))
}

pub fn draw(
    context: &egui::Context,
    ui: &mut egui::Ui,
    lineup_data: &ResolvedFixedLineup,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
) {
    ui.strong("Fixed Lineup");
    ui.separator();
    ui.add_space(4.0);

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = ICON_SPACING;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ICON_SPACING;
            for slot_data in lineup_data.slots.iter().take(5) {
                draw_slot(context, ui, slot_data, texture_cache);
            }
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ICON_SPACING;
            for slot_data in lineup_data.slots.iter().skip(5).take(5) {
                draw_slot(context, ui, slot_data, texture_cache);
            }
        });
    });
}

fn draw_slot(
    context: &egui::Context,
    ui: &mut egui::Ui,
    slot_data: &ResolvedSlot,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
) {
    let mut is_rendered = false;

    if let Some(image_path) = &slot_data.image_path {
        let path_string = image_path.to_string_lossy().to_string();

        if !texture_cache.contains_key(&path_string) {
            if let Some(image_data) = load_cat_icon(image_path) {
                let texture_handle = context.load_texture(&path_string, image_data, egui::TextureOptions::LINEAR);
                texture_cache.insert(path_string.clone(), texture_handle);
            }
        }

        if let Some(texture_handle) = texture_cache.get(&path_string) {
            let max_image_size = egui::vec2(128.0 * ICON_SCALE, 128.0 * ICON_SCALE);

            let cat_image = egui::Image::new(texture_handle).max_size(max_image_size);
            let image_response = ui.add(cat_image);

            if let (Some(unit_id), Some(unit_level)) = (slot_data.unit_id, slot_data.level) {
                let plus_level = slot_data.plus_level.unwrap_or(0);
                let plus_string = if plus_level > 0 { format!(" +{}", plus_level) } else { "".to_string() };

                image_response.on_hover_text(format!("Unit ID: {}\nLevel: {}{}", unit_id, unit_level, plus_string));
            }

            is_rendered = true;
        }
    }

    if !is_rendered {
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(128.0 * ICON_SCALE, 128.0 * ICON_SCALE),
            egui::Sense::hover()
        );
        ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GRAY);
    }
}