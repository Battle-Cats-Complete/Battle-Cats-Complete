use eframe::egui;

const SPACING_TOP: f32 = 0.0;
const SPACING_AFTER_TITLE: f32 = 2.0;
const SPACING_AFTER_SUBTITLE: f32 = 12.0;
const SPACING_AFTER_HEADER: f32 = 4.0;

pub fn show(ui: &mut egui::Ui) -> bool {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    ui.vertical(|ui| {
        ui.add_space(SPACING_TOP);

        ui.heading("About Battle Cats Complete");
        ui.add_space(SPACING_AFTER_TITLE);

        ui.label(egui::RichText::new("A high-performance Battle Cats toolkit built by Omochi").weak());

        ui.add_space(SPACING_AFTER_SUBTITLE);
        ui.separator();

        ui.add_space(SPACING_AFTER_HEADER);
        ui.label(egui::RichText::new("Open Source & Legal Info").strong());
        ui.add_space(SPACING_AFTER_HEADER);

        ui.separator();

        let license_text = include_str!("../../../assets/licenses.txt");

        egui::ScrollArea::vertical()
            .id_salt("about_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(license_text).monospace().size(11.0));
            });
    });

    false
}