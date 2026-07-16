use eframe::egui;

use core::common::utils::strip_markdown;

pub(crate) fn process_markdown(ui: &mut egui::Ui, raw_text: &str) {
    let content = strip_markdown(raw_text);

    for line in content.lines() {
        let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            ui.add_space(10.0);
            continue;
        }

        ui.horizontal_top(|ui| {
            if leading_spaces > 0 {
                ui.add_space(leading_spaces as f32 * 6.0);
            }

            if trimmed.starts_with('•') || trimmed.starts_with('-') || trimmed.starts_with('*') {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.label("•");
                let text = trimmed.trim_start_matches(['•', '-', '*']).trim();
                ui.add(egui::Label::new(text).wrap());
            } else if trimmed.starts_with('#') {
                let text = trimmed.trim_start_matches('#').trim();
                ui.add(egui::Label::new(
                    egui::RichText::new(text).heading().strong()
                ).wrap());
            } else {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.add(egui::Label::new(trimmed).wrap());
            }
        });
    }
}