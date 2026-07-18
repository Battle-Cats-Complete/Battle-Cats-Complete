use std::path::Path;

use eframe::egui;

use core::modules::settings::Settings;

use crate::common::DragGuard;
use crate::app::Page;

use super::{changelog, notice};

const SPACE_TOP: f32 = 20.0;
const SPACE_TITLE_SUBTITLE: f32 = 2.0;
const SPACE_SUBTITLE_SECTION: f32 = 50.0;
const SPACE_SECTION_HEADER_ROW: f32 = 10.0;
const SPACE_BETWEEN_SECTIONS: f32 = 20.0;

const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 35.0;
const BUTTON_SPACING: f32 = 10.0;

pub(crate) fn show(ctx: &egui::Context, current_page: &mut Page, settings: &mut Settings, drag_guard: &mut DragGuard) {
    let is_game_empty = ctx.data_mut(|data_map| {
        let state_id = egui::Id::new("is_game_empty");
        if let Some(val) = data_map.get_temp::<bool>(state_id) {
            val
        } else {
            let game_dir = Path::new("game");
            let empty = !game_dir.exists() || std::fs::read_dir(game_dir).map(|mut iter| iter.next().is_none()).unwrap_or(true);
            data_map.insert_temp(state_id, empty);
            empty
        }
    });

    egui::CentralPanel::default().show(ctx, |ui_container| {
        ui_container.vertical_centered(|ui_center| {
            ui_center.add_space(SPACE_TOP);

            ui_center.heading(
                egui::RichText::new("Battle Cats Complete")
                    .size(40.0)
                    .color(egui::Color32::WHITE)
                    .strong()
            );

            ui_center.add_space(SPACE_TITLE_SUBTITLE);
            ui_center.label(egui::RichText::new("All-In-One Battle Cats Toolkit").size(16.0).weak());

            ui_center.add_space(SPACE_SUBTITLE_SECTION);

            if is_game_empty {
                ui_center.add_space(10.0);

                ui_center.label(egui::RichText::new("To get started, you will need to populate the \"game\" folder with game files using the \"Data\" page.").size(15.0));
                ui_center.add_space(8.0);
                if ui_center.add_sized([BUTTON_WIDTH, BUTTON_HEIGHT], egui::Button::new(egui::RichText::new("Data").size(15.0)).fill(egui::Color32::from_rgb(31, 106, 165))).clicked() {
                    tracing::debug!("Start guide navigated to Data page");
                    *current_page = Page::Data;
                }

                ui_center.add_space(35.0);

                ui_center.label(egui::RichText::new("To import encrypted \"pack\" files, you will need to provide Decryption Keys and Initialization Vectors\nusing the \"Manage Keys\" button under the \"Data\" tab in the \"Settings\" page.").size(15.0));
                ui_center.add_space(8.0);
                if ui_center.add_sized([270.0, BUTTON_HEIGHT], egui::Button::new(egui::RichText::new("Settings > Data > Manage Keys").size(15.0)).fill(egui::Color32::from_rgb(31, 106, 165))).clicked() {
                    tracing::debug!("Start guide navigated to Manage Keys modal");
                    *current_page = Page::Settings;
                    settings.runtime.active_tab = "Data".to_string();
                    crate::modules::settings::keys::open(ctx);
                }

                ui_center.add_space(35.0);

                ui_center.label(egui::RichText::new("Importing from an Android device or Emulator requires Keys & IV, the Android bridge Add-on, and\nroot access. You can find the \"Android Bridge\" section under the \"Add-Ons\" tab in the \"Settings\" page.").size(15.0));
                ui_center.add_space(8.0);
                if ui_center.add_sized([180.0, BUTTON_HEIGHT], egui::Button::new(egui::RichText::new("Settings > Add-Ons").size(15.0)).fill(egui::Color32::from_rgb(31, 106, 165))).clicked() {
                    tracing::debug!("Start guide navigated to Add-Ons");
                    *current_page = Page::Settings;
                    settings.runtime.active_tab = "Add-Ons".to_string();
                }

            } else {
                let nav_row = |ui_row: &mut egui::Ui, page_state: &mut Page, buttons: &[(&str, Page)]| {
                    let count = buttons.len();
                    if count == 0 { return; }

                    let total_width = (BUTTON_WIDTH * count as f32) + (BUTTON_SPACING * (count - 1) as f32);

                    ui_row.horizontal(|ui_horizontal| {
                        ui_horizontal.spacing_mut().item_spacing.x = BUTTON_SPACING;

                        let center_padding = (ui_horizontal.available_width() - total_width) / 2.0;
                        if center_padding > 0.0 {
                            ui_horizontal.add_space(center_padding);
                        }

                        for (label, target) in buttons {
                            let button_text = egui::RichText::new(*label).size(15.0);
                            let nav_button = egui::Button::new(button_text)
                                .fill(egui::Color32::from_rgb(31, 106, 165));

                            if ui_horizontal.add_sized([BUTTON_WIDTH, BUTTON_HEIGHT], nav_button).clicked() {
                                tracing::debug!("Home screen navigated to target page: {}", label);
                                *page_state = *target;
                            }
                        }
                    });
                };

                ui_center.heading(egui::RichText::new("Information").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
                ui_center.add_space(SPACE_SECTION_HEADER_ROW);
                nav_row(ui_center, current_page, &[
                    ("Cats", Page::Cats),
                    ("Enemies", Page::Enemies),
                    ("Stages", Page::Stages),
                ]);

                ui_center.add_space(SPACE_BETWEEN_SECTIONS);

                ui_center.heading(egui::RichText::new("Database").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
                ui_center.add_space(SPACE_SECTION_HEADER_ROW);
                nav_row(ui_center, current_page, &[
                    ("Mods", Page::Mods),
                    ("Data", Page::Data),
                ]);

                ui_center.add_space(SPACE_BETWEEN_SECTIONS);

                ui_center.heading(egui::RichText::new("Other").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
                ui_center.add_space(SPACE_SECTION_HEADER_ROW);
                nav_row(ui_center, current_page, &[
                    ("Settings", Page::Settings),
                ]);
            }
        });
    });

    egui::Area::new("version_area".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .order(egui::Order::Background)
        .show(ctx, |ui_container| {
            ui_container.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );

            let current_version = env!("CARGO_PKG_VERSION");
            let package_tag = format!("v{}", current_version);
            let release_url = format!("https://github.com/omochikaeri15/battle-cats-complete/releases/tag/{}", package_tag);

            ui_container.horizontal(|ui_row| {
                ui_row.hyperlink_to(&package_tag, release_url);
                ui_row.label("|");

                changelog::link(ui_row, ctx);
            });
        });

    egui::Area::new("social_links_area".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .order(egui::Order::Background)
        .show(ctx, |ui_container| {
            ui_container.horizontal(|ui_row| {
                ui_row.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(13.0, egui::FontFamily::Proportional),
                );

                ui_row.hyperlink_to("Discord", "https://discord.com/invite/SNSE8HNhmP").clicked();
                ui_row.label("|");
                ui_row.hyperlink_to("GitHub", "https://github.com/omochikaeri15/battle-cats-complete");
            });
        });

    changelog::window(ctx, drag_guard);
    notice::check_and_show(ctx, drag_guard);
}