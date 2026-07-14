use std::collections::HashMap;

use eframe::egui::{self, RichText};
use nyanko::chapter::stage::{AbilityType, CannonType, EvolutionForm, CertificationPreset, TreasureType};
use tracing::{error, warn, trace};

use core::cat::waiter::unitexplanation;
use core::global::utils::autocrop;
use core::stage::paths;
use core::stage::logic::fixedlineup::{ResolvedFixedLineup, ResolvedSlot};

const ICON_SCALE: f32 = 0.45;
const ICON_SPACING: f32 = 8.0;
const SCROLL_AREA_HEIGHT: f32 = 90.0;

fn load_cat_icon(path: &std::path::Path) -> Option<egui::ColorImage> {
    let Ok(image_file) = image::open(path) else {
        error!(?path, "Failed to open cat icon image file");
        return None;
    };

    let cropped_image = autocrop(image_file.to_rgba8());
    let dimensions = [cropped_image.width() as usize, cropped_image.height() as usize];

    trace!(?path, width = dimensions[0], height = dimensions[1], "Loaded and cropped cat icon");
    Some(egui::ColorImage::from_rgba_unmultiplied(dimensions, cropped_image.as_flat_samples().as_slice()))
}

pub fn draw(
    context: &egui::Context,
    ui: &mut egui::Ui,
    resolved_lineup: &ResolvedFixedLineup,
    preset_data: &CertificationPreset,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    langs: &[String],
) {
    ui.label(RichText::new("Fixed Lineup").strong().heading());
    ui.separator();

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = ICON_SPACING;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ICON_SPACING;
                for slot_data in resolved_lineup.slots.iter().take(5) {
                    draw_slot(context, ui, slot_data, preset_data, texture_cache, langs);
                }
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ICON_SPACING;
                for slot_data in resolved_lineup.slots.iter().skip(5).take(5) {
                    draw_slot(context, ui, slot_data, preset_data, texture_cache, langs);
                }
            });
        });

        ui.add_space(24.0);

        ui.vertical(|ui| {
            let frame_response = egui::Frame::none()
                .stroke(egui::Stroke::new(2.0, egui::Color32::DARK_GRAY))
                .inner_margin(4.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("fixed_lineup_upgrades_scroll")
                        .max_height(SCROLL_AREA_HEIGHT)
                        .show(ui, |ui| {
                            draw_upgrades_section(ui, preset_data);
                        });
                });

            let interact_rect = frame_response.response.rect;
            let hover_response = ui.interact(interact_rect, ui.id().with("fl_scroll_hover"), egui::Sense::hover());
            let is_hovering_upgrades = hover_response.hovered();

            ui.ctx().data_mut(|data_map| {
                data_map.insert_temp(egui::Id::new("fixed_lineup_hovered"), is_hovering_upgrades);
            });
        });
    });
}

fn draw_empty_slot(ui: &mut egui::Ui) {
    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(128.0 * ICON_SCALE, 128.0 * ICON_SCALE),
        egui::Sense::hover()
    );
    ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GRAY);
}

fn draw_slot(
    context: &egui::Context,
    ui: &mut egui::Ui,
    slot_data: &ResolvedSlot,
    preset_data: &CertificationPreset,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    langs: &[String],
) {
    let Some(image_path) = &slot_data.image_path else {
        draw_empty_slot(ui);
        return;
    };

    let path_string = image_path.to_string_lossy().to_string();

    if !texture_cache.contains_key(&path_string) {
        let Some(image_data) = load_cat_icon(image_path) else {
            warn!(?image_path, "Could not load cat icon; rendering empty slot fallback");
            draw_empty_slot(ui);
            return;
        };
        let texture_handle = context.load_texture(&path_string, image_data, egui::TextureOptions::LINEAR);
        texture_cache.insert(path_string.clone(), texture_handle);
    }

    let Some(texture_handle) = texture_cache.get(&path_string) else {
        draw_empty_slot(ui);
        return;
    };

    let max_image_size = egui::vec2(128.0 * ICON_SCALE, 128.0 * ICON_SCALE);
    let cat_image = egui::Image::new(texture_handle).max_size(max_image_size);
    let image_response = ui.add(cat_image);

    let (Some(unit_id), Some(unit_level)) = (slot_data.unit_id, slot_data.level) else {
        return;
    };

    let padded_id = format!("{:03}", unit_id);
    let cat_folder = paths::cat_folder(unit_id);
    let explanation = unitexplanation(unit_id, &cat_folder, langs);

    let form_index = preset_data.characters.get(&unit_id).map_or(0, |chara| {
        match chara.evolution_form {
            EvolutionForm::Normal => 0,
            EvolutionForm::Evolved => 1,
            EvolutionForm::True => 2,
            EvolutionForm::Ultra => 3,
            _ => 0,
        }
    });

    let mut display_name = format!("Unit {}", padded_id);
    if let Some(Some(name)) = explanation.names.get(form_index).or_else(|| explanation.names.get(0)) {
        display_name = name.clone();
    }

    let plus_level = slot_data.plus_level.unwrap_or(0);
    let plus_string = if plus_level > 0 { format!("+{}", plus_level) } else { String::new() };

    image_response.on_hover_ui(|tooltip_ui| {
        tooltip_ui.horizontal(|row_ui| {
            row_ui.label(RichText::new("[Name]").weak());
            row_ui.label(&display_name);
        });

        tooltip_ui.horizontal(|row_ui| {
            row_ui.label(RichText::new("[ID]").weak());
            row_ui.label(&padded_id);
        });

        tooltip_ui.horizontal(|row_ui| {
            row_ui.label(RichText::new("[Level]").weak());
            row_ui.label(format!("{}{}", unit_level, plus_string));
        });
    });
}

fn draw_upgrades_section(ui: &mut egui::Ui, preset_data: &CertificationPreset) {
    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let estimated_grid_width = 140.0;

        if available_width > estimated_grid_width {
            ui.add_space((available_width - estimated_grid_width) / 2.0);
        }

        egui::Grid::new("fixed_lineup_cannon_grid")
            .striped(true)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                let cannon_name = match preset_data.slot_cannon_type {
                    CannonType::Basic => "Basic",
                    CannonType::SlowBeam => "Slow Beam",
                    CannonType::IronWall => "Iron Wall",
                    CannonType::Thunderbolt => "Thunderbolt",
                    CannonType::Waterblast => "Waterblast",
                    CannonType::HolyBlast => "HolyBlast",
                    CannonType::Breakerblast => "Breakerblast",
                    CannonType::Curseblast => "Curseblast",
                    CannonType::Unknown(_) => "Unknown",
                };

                let cannon_level = preset_data.cannon_levels.get(&preset_data.slot_cannon_type).copied().unwrap_or(0);

                ui.strong("Cannon");
                ui.centered_and_justified(|ui| {
                    ui.strong("Level");
                });
                ui.end_row();

                ui.label(cannon_name);
                ui.centered_and_justified(|ui| {
                    ui.label(cannon_level.to_string());
                });
                ui.end_row();
            });
    });

    ui.add_space(16.0);

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            egui::Grid::new("fixed_lineup_abilities_grid")
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Upgrade");
                    ui.centered_and_justified(|ui| {
                        ui.strong("Level");
                    });
                    ui.end_row();

                    draw_ability_rows(ui, preset_data);
                });
        });

        ui.add_space(24.0);

        ui.vertical(|ui| {
            egui::Grid::new("fixed_lineup_treasures_grid")
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Chapter");
                    ui.centered_and_justified(|ui| {
                        ui.strong("Grades");
                    });
                    ui.end_row();

                    draw_treasure_rows(ui, preset_data);
                });
        });
    });
}

fn draw_ability_rows(ui: &mut egui::Ui, preset_data: &CertificationPreset) {
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
        ui.centered_and_justified(|ui| {
            ui.label(level_string);
        });
        ui.end_row();
    }
}

fn draw_treasure_rows(ui: &mut egui::Ui, preset_data: &CertificationPreset) {
    const TREASURES: [(TreasureType, &str); 9] = [
        (TreasureType::EoC1, "EoC Ch. 1"),
        (TreasureType::EoC2, "EoC Ch. 2"),
        (TreasureType::EoC3, "EoC Ch. 3"),
        (TreasureType::ItF1, "ItF Ch. 1"),
        (TreasureType::ItF2, "ItF Ch. 2"),
        (TreasureType::ItF3, "ItF Ch. 3"),
        (TreasureType::CotC1, "CotC Ch. 1"),
        (TreasureType::CotC2, "CotC Ch. 2"),
        (TreasureType::CotC3, "CotC Ch. 3"),
    ];

    for (treasure_type, name) in TREASURES {
        let grades_string = if let Some(treasure_data) = preset_data.treasures.get(&treasure_type) {
            format!("{}/{}/{}", treasure_data.superior_count, treasure_data.normal_count, treasure_data.inferior_count)
        } else {
            "0/0/0".to_string()
        };

        ui.label(name);
        ui.centered_and_justified(|ui| {
            ui.label(grades_string);
        });
        ui.end_row();
    }
}