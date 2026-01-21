use eframe::egui;

pub fn render(ui: &mut egui::Ui, description: &[String]) {
    ui.add_space(10.0);
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("Description").size(20.0).strong());
    });
    ui.add_space(8.0);

    ui.vertical_centered(|ui| {
        if description.is_empty() {
            ui.label(egui::RichText::new("No description available").weak().italics());
            return;
        }

        for line in description {
            if line.trim().is_empty() {
                ui.label(" "); 
            } else {
                ui.add(egui::Label::new(egui::RichText::new(line).size(15.0)).wrap());
            }
        }
    });
}