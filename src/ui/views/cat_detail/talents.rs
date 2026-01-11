use eframe::egui;
use crate::core::files::skillacquisition::{self, TalentRaw}; // Imported self for the function
use crate::core::files::imgcut::SpriteSheet;

pub fn render(
    ui: &mut egui::Ui,
    talent_data: &TalentRaw,
    sheet: &SpriteSheet,
) {
    ui.add_space(5.0);
    ui.heading("Talent Icons");
    ui.add_space(5.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

        for group in &talent_data.groups {
            // Call the shared dictionary in the core file
            if let Some(icon_id) = skillacquisition::map_ability_to_icon(group.ability_id) {
                
                if let Some(sprite) = sheet.get_sprite_by_line(icon_id) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(40.0, 40.0)));
                } else {
                    // Fallback if sprite missing but ID is mapped
                    ui.label(format!("ID:{}", group.ability_id));
                }
            } else {
                // If mapping fails (Ability ID not in dictionary)
                ui.label(format!("Unmapped:{}", group.ability_id));
            }
        }
    });
}