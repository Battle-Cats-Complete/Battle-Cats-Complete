use eframe::egui;

use core::modules::stage::registry::Stage;

pub(crate) fn draw(ui: &mut egui::Ui, stage: &Stage, selected_crown: &mut u8) {
    if stage.max_crowns <= 1 {
        return;
    }

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.horizontal(|ui| {
            for c in 0..stage.max_crowns {
                let is_selected = *selected_crown == c;

                let mut is_enabled = true;
                if stage.target_crowns != -1 && stage.target_crowns as u8 != c {
                    is_enabled = false;
                }

                let (fill, stroke, text) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else if is_enabled {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                } else {
                    (egui::Color32::from_gray(15), egui::Stroke::new(1.0, egui::Color32::from_gray(50)), egui::Color32::from_gray(120))
                };

                let label = format!("{}♔", c + 1);
                let btn = egui::Button::new(egui::RichText::new(label).color(text).strong())
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::ZERO)
                    .min_size(egui::vec2(50.0, 30.0));

                if ui.add_enabled(is_enabled, btn).clicked() {
                    *selected_crown = c;
                }
            }
        });
    });

    ui.add_space(8.0);
}