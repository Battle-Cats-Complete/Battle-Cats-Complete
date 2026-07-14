use eframe::egui;

use crate::global::shared::DragGuard;
use crate::app::frame::Page;

use super::{changelog, notice};
const SPACE_TOP: f32 = 20.0;
const SPACE_TITLE_SUBTITLE: f32 = 2.0;
const SPACE_SUBTITLE_SECTION: f32 = 50.0;
const SPACE_SECTION_HEADER_ROW: f32 = 10.0;
const SPACE_BETWEEN_SECTIONS: f32 = 20.0;

const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 35.0;
const BUTTON_SPACING: f32 = 10.0;

pub fn show(ctx: &egui::Context, current_page: &mut Page, drag_guard: &mut DragGuard) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(SPACE_TOP);

            ui.heading(
                egui::RichText::new("Battle Cats Complete")
                    .size(40.0)
                    .color(egui::Color32::WHITE)
                    .strong()
            );

            ui.add_space(SPACE_TITLE_SUBTITLE);
            ui.label(egui::RichText::new("All-In-One Battle Cats Toolkit").size(16.0).weak());

            ui.add_space(SPACE_SUBTITLE_SECTION);

            let mut nav_row = |ui: &mut egui::Ui, page_state: &mut Page, buttons: &[(&str, Page)]| {
                let count = buttons.len();
                if count == 0 {
                    return;
                }

                let total_width = (BUTTON_WIDTH * count as f32) + (BUTTON_SPACING * (count - 1) as f32);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = BUTTON_SPACING;

                    let center_padding = (ui.available_width() - total_width) / 2.0;
                    if center_padding > 0.0 {
                        ui.add_space(center_padding);
                    }

                    for (label, target) in buttons {
                        let btn_text = egui::RichText::new(*label).size(15.0);
                        let btn = egui::Button::new(btn_text)
                            .fill(egui::Color32::from_rgb(31, 106, 165)); // Theme blue

                        if ui.add_sized([BUTTON_WIDTH, BUTTON_HEIGHT], btn).clicked() {
                            tracing::debug!("Home screen navigated to target page: {}", label);
                            *page_state = *target;
                        }
                    }
                });
            };

            ui.heading(egui::RichText::new("Information").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
            ui.add_space(SPACE_SECTION_HEADER_ROW);
            nav_row(ui, current_page, &[
                ("Cats", Page::Cats),
                ("Enemies", Page::Enemies),
                ("Stages", Page::Stages),
            ]);

            ui.add_space(SPACE_BETWEEN_SECTIONS);

            ui.heading(egui::RichText::new("Database").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
            ui.add_space(SPACE_SECTION_HEADER_ROW);
            nav_row(ui, current_page, &[
                ("Mods", Page::Mods),
                ("Data", Page::Data),
            ]);

            ui.add_space(SPACE_BETWEEN_SECTIONS);

            ui.heading(egui::RichText::new("Other").size(18.0).strong().color(egui::Color32::LIGHT_GRAY));
            ui.add_space(SPACE_SECTION_HEADER_ROW);
            nav_row(ui, current_page, &[
                ("Settings", Page::Settings),
            ]);
        });
    });

    egui::Area::new("version_area".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );

            let current_version = env!("CARGO_PKG_VERSION");
            let tag = format!("v{}", current_version);
            let release_url = format!("https://github.com/omochikaeri15/battle-cats-complete/releases/tag/{}", tag);

            ui.horizontal(|ui| {
                ui.hyperlink_to(&tag, release_url);
                ui.label("|");

                changelog::link(ui, ctx);
            });
        });

    egui::Area::new("social_links_area".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(13.0, egui::FontFamily::Proportional),
                );

                ui.hyperlink_to("Discord", "https://discord.com/invite/SNSE8HNhmP").clicked();
                ui.label("|");
                ui.hyperlink_to("GitHub", "https://github.com/omochikaeri15/battle-cats-complete");
            });
        });

    changelog::window(ctx, drag_guard);
    notice::check_and_show(ctx, drag_guard);
}