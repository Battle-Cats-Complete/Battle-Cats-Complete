use std::collections::HashMap;
use std::path::Path;

use eframe::egui;

use core::global::utils::autocrop;
use core::stage::data::certification_preset::{AbilityType, CannonType, PresetLineup, TreasureType};
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
    resolved_lineup: &ResolvedFixedLineup,
    preset_data: &PresetLineup,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
) {
    ui.strong("Fixed Lineup");
    ui.separator();
    ui.add_space(4.0);

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = ICON_SPACING;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ICON_SPACING;
                for slot_data in resolved_lineup.slots.iter().take(5) {
                    draw_slot(context, ui, slot_data, texture_cache);
                }
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ICON_SPACING;
                for slot_data in resolved_lineup.slots.iter().skip(5).take(5) {
                    draw_slot(context, ui, slot_data, texture_cache);
                }
            });
        });

        ui.add_space(24.0);

        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("fixed_lineup_upgrades_scroll")
                .max_height(260.0)
                .show(ui, |ui| {
                    egui::Grid::new("fixed_lineup_upgrades_grid")
                        .striped(true)
                        .spacing([32.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("Upgrade Name");
                            ui.strong("Level");
                            ui.end_row();

                            draw_ability_rows(ui, preset_data);
                            draw_cannon_rows(ui, preset_data);
                            draw_treasure_rows(ui, preset_data);
                        });
                });
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

fn draw_ability_rows(ui: &mut egui::Ui, preset_data: &PresetLineup) {
    const ABILITIES: [(AbilityType, &str); 10] = [
        (AbilityType::CatCannonAttack, "Cat Cannon Attack"),
        (AbilityType::CatCannonRange, "Cat Cannon Range"),
        (AbilityType::CatCannonCharge, "Cat Cannon Charge"),
        (AbilityType::WorkerCatRate, "Worker Cat Efficiency"),
        (AbilityType::WorkerCatWallet, "Worker Cat Wallet"),
        (AbilityType::BaseDefense, "Cat Base Health"),
        (AbilityType::Research, "Research"),
        (AbilityType::BountyUp, "Accounting"),
        (AbilityType::Study, "Study"),
        (AbilityType::CatEnergy, "Cat Energy"),
    ];

    for (ability_type, name) in ABILITIES {
        let level_string = if let Some(ability_data) = preset_data.abilities.get(&ability_type) {
            if ability_data.plus_level > 0 {
                format!("{} +{}", ability_data.level, ability_data.plus_level)
            } else {
                ability_data.level.to_string()
            }
        } else {
            "0".to_string()
        };

        ui.label(name);
        ui.label(level_string);
        ui.end_row();
    }
}

fn draw_cannon_rows(ui: &mut egui::Ui, preset_data: &PresetLineup) {
    let cannon_name = match preset_data.slot_cannon_type {
        CannonType::Basic => "Basic Cannon",
        CannonType::SlowBeam => "Slow Beam",
        CannonType::IronWall => "Iron Wall",
        CannonType::Thunderbolt => "Thunderbolt",
        CannonType::Waterblast => "Waterblast",
        CannonType::HolyBlast => "HolyBlast",
        CannonType::Breakerblast => "Breakerblast",
        CannonType::Curseblast => "Curseblast",
        CannonType::Unknown(_) => "Unknown Cannon",
    };

    let cannon_level = preset_data.cannon_levels.get(&preset_data.slot_cannon_type).copied().unwrap_or(0);

    ui.label(format!("Equipped: {}", cannon_name));
    ui.label(cannon_level.to_string());
    ui.end_row();
}

fn draw_treasure_rows(ui: &mut egui::Ui, preset_data: &PresetLineup) {
    const TREASURES: [(TreasureType, &str); 9] = [
        (TreasureType::EoC1, "EoC Ch. 1 Treasures"),
        (TreasureType::EoC2, "EoC Ch. 2 Treasures"),
        (TreasureType::EoC3, "EoC Ch. 3 Treasures"),
        (TreasureType::ItF1, "ItF Ch. 1 Treasures"),
        (TreasureType::ItF2, "ItF Ch. 2 Treasures"),
        (TreasureType::ItF3, "ItF Ch. 3 Treasures"),
        (TreasureType::CotC1, "CotC Ch. 1 Treasures"),
        (TreasureType::CotC2, "CotC Ch. 2 Treasures"),
        (TreasureType::CotC3, "CotC Ch. 3 Treasures"),
    ];

    for (treasure_type, name) in TREASURES {
        let count_string = if let Some(treasure_data) = preset_data.treasures.get(&treasure_type) {
            format!("{}/{}/{}", treasure_data.superior_count, treasure_data.normal_count, treasure_data.inferior_count)
        } else {
            "0/0/0".to_string()
        };

        ui.label(name);
        ui.label(count_string);
        ui.end_row();
    }
}