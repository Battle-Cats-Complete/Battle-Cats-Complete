use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::features::settings::logic::Settings;

pub fn show(
    _ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy_entry: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    _settings: &mut Settings
) {
    ui.horizontal(|ui| {
        let display_name = if enemy_entry.name.is_empty() {
            format!("Enemy {:03}", enemy_entry.id)
        } else {
            enemy_entry.name.clone()
        };
        ui.heading(display_name);
    });

    ui.add_space(5.0);
    
    ui.horizontal(|ui| {
        ui.selectable_value(current_tab, EnemyDetailTab::Stats, "Stats");
        ui.selectable_value(current_tab, EnemyDetailTab::Description, "Description");
        ui.selectable_value(current_tab, EnemyDetailTab::Animation, "Animation");
    });
    
    ui.separator();

    match current_tab {
        EnemyDetailTab::Stats => {
            ui.label(format!("HP: {}", enemy_entry.stats.hitpoints));
            ui.label(format!("Speed: {}", enemy_entry.stats.speed));
            // We will build out the full stats view here shortly!
        },
        EnemyDetailTab::Description => {
            for line in &enemy_entry.description {
                ui.label(line);
            }
            if enemy_entry.description.is_empty() {
                ui.label(egui::RichText::new("No description available.").weak());
            }
        },
        EnemyDetailTab::Animation => {
            ui.label("Animation Viewer Coming Soon!");
        }
    }
}