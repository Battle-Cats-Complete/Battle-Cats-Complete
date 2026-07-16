use std::collections::HashMap;
use std::path::Path;

use eframe::egui::{self,  RichText};
use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::stage::RewardStructure;

use core::common::formats::gatyaitembuy::GatyaItemBuy;
use core::common::formats::gatyaitemname::GatyaItemName;
use core::common::utils::autocrop;
use core::modules::stage::logic::treasure;
use core::modules::stage::registry::Stage;

pub const TREASURE_TABLE_WIDTH: f32 = 345.0;

fn format_drop_chance(raw_chance: u32, drop_rule: i32) -> String {
    if drop_rule == -3 || drop_rule == -4 {
        return "100%".to_string();
    }
    format!("{}%", raw_chance)
}

fn process_item_icon_texture(icon_file_path: &Path) -> Option<egui::ColorImage> {
    let Ok(loaded_raw_image_data) = image::open(icon_file_path) else {
        return None;
    };

    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let (crop_width, crop_height) = autocropped_rgba_image.dimensions();
    let max_dimension = crop_width.max(crop_height) as f32;
    let scale_factor = 32.0 / max_dimension;

    let target_width = (crop_width as f32 * scale_factor).round() as u32;
    let target_height = (crop_height as f32 * scale_factor).round() as u32;

    let resized_rgba_image = image::imageops::resize(
        &autocropped_rgba_image,
        target_width.max(1),
        target_height.max(1),
        image::imageops::FilterType::Triangle
    );

    let image_dimensions = [resized_rgba_image.width() as usize, resized_rgba_image.height() as usize];

    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, resized_rgba_image.as_flat_samples().as_slice()))
}

fn format_treasure_rule(drop_rule: i32) -> &'static str {
    match drop_rule {
        1 => "Once, Then Unlimited",
        0 => "Unlimited",
        -1 => "Unlimited (%)",
        -3 => "Guaranteed (Once)",
        -4 => "Guaranteed (Unlimited)",
        _ => "Unknown Rule",
    }
}

pub fn center_header(ui: &mut egui::Ui, display_text: &str) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(RichText::new(display_text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn center_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    egui_context: &egui::Context,
    ui: &mut egui::Ui,
    stage_data: &Stage,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuy>,
    item_texture_cache: &mut HashMap<u32, egui::TextureHandle>,
    active_language_priority_array: &[String]
) {
    match &stage_data.rewards {
        RewardStructure::Treasure { drop_rule, drops } => {
            let valid_drops_array: Vec<_> = drops.iter().filter(|drop_data| drop_data.chance > 0).collect();

            if valid_drops_array.is_empty() {
                return;
            }

            ui.set_max_width(TREASURE_TABLE_WIDTH);

            let rule_description = format_treasure_rule(*drop_rule);
            ui.label(RichText::new(format!("Treasure | {}", rule_description)).strong().heading());
            ui.separator();

            egui::Grid::new("reward_treasure_grid")
                .striped(true)
                .spacing([15.0, 4.0])
                .min_row_height(32.0)
                .show(ui, |grid| {
                    center_header(grid, "Chance");
                    center_header(grid, "Item");
                    center_header(grid, "Amount");
                    grid.end_row();

                    for drop_data in valid_drops_array {
                        let drop_info = treasure::resolve_drop(
                            drop_data.item_id,
                            drop_data.amount,
                            item_buy_registry,
                            item_name_registry,
                            drop_chara_registry,
                            unit_buy_registry,
                            active_language_priority_array
                        );

                        let chance_display = format_drop_chance(drop_data.chance, *drop_rule);
                        center_text(grid, chance_display);

                        grid.centered_and_justified(|icon_layout| {
                            let mut has_rendered_icon = false;

                            if let Some(resolved_image_path) = drop_info.image_path {
                                if !item_texture_cache.contains_key(&drop_data.item_id) {
                                    if let Some(processed_color_image) = process_item_icon_texture(&resolved_image_path) {
                                        let generated_texture_handle = egui_context.load_texture(
                                            format!("treasure_item_icon_{}", drop_data.item_id),
                                            processed_color_image,
                                            egui::TextureOptions::LINEAR
                                        );
                                        item_texture_cache.insert(drop_data.item_id, generated_texture_handle);
                                    }
                                }

                                if let Some(cached_texture_handle) = item_texture_cache.get(&drop_data.item_id) {
                                    let image_response = icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                                    image_response.on_hover_text(drop_info.name.clone());
                                    has_rendered_icon = true;
                                }
                            }

                            if !has_rendered_icon {
                                icon_layout.add(egui::Label::new(&drop_info.name).wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });

                        center_text(grid, drop_info.amount_display);
                        grid.end_row();
                    }
                });
        }
        RewardStructure::Timed(timed_scores) => {
            if timed_scores.is_empty() {
                return;
            }

            ui.set_max_width(TREASURE_TABLE_WIDTH);

            ui.strong("Timed Score Rewards");
            ui.separator();

            egui::Grid::new("reward_timed_grid")
                .striped(true)
                .spacing([15.0, 4.0])
                .min_row_height(32.0)
                .show(ui, |grid| {
                    center_header(grid, "Score");
                    center_header(grid, "Item");
                    center_header(grid, "Amount");
                    grid.end_row();

                    for score_data in timed_scores {
                        let drop_info = treasure::resolve_drop(
                            score_data.item_id,
                            score_data.amount,
                            item_buy_registry,
                            item_name_registry,
                            drop_chara_registry,
                            unit_buy_registry,
                            active_language_priority_array
                        );

                        center_text(grid, score_data.score.to_string());

                        grid.centered_and_justified(|icon_layout| {
                            let mut has_rendered_icon = false;

                            if let Some(resolved_image_path) = drop_info.image_path {
                                if !item_texture_cache.contains_key(&score_data.item_id) {
                                    if let Some(processed_color_image) = process_item_icon_texture(&resolved_image_path) {
                                        let generated_texture_handle = egui_context.load_texture(
                                            format!("treasure_item_icon_{}", score_data.item_id),
                                            processed_color_image,
                                            egui::TextureOptions::LINEAR
                                        );
                                        item_texture_cache.insert(score_data.item_id, generated_texture_handle);
                                    }
                                }

                                if let Some(cached_texture_handle) = item_texture_cache.get(&score_data.item_id) {
                                    let image_response = icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                                    image_response.on_hover_text(drop_info.name.clone());
                                    has_rendered_icon = true;
                                }
                            }

                            if !has_rendered_icon {
                                icon_layout.add(egui::Label::new(&drop_info.name).wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });

                        center_text(grid, drop_info.amount_display);
                        grid.end_row();
                    }
                });
        }
        RewardStructure::None => {
        }
    }
}