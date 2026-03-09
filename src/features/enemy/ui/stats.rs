use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::ui::components::stat_grid::{grid_cell, grid_cell_custom, render_frames};

pub fn render(ui: &mut egui::Ui, enemy: &EnemyEntry, magnification: i32) {
    let stats = &enemy.stats;
    let mag_f = magnification as f32 / 100.0;

    let hp = (stats.hitpoints as f32 * mag_f).round() as i32;
    let atk1 = (stats.attack_1 as f32 * mag_f).round() as i32;
    let atk2 = (stats.attack_2 as f32 * mag_f).round() as i32;
    let atk3 = (stats.attack_3 as f32 * mag_f).round() as i32;

    let total_atk = atk1 + atk2 + atk3;
    let cycle = stats.attack_cycle(enemy.atk_anim_frames);
    let dps = if cycle > 0 { (total_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
    let atk_type = if stats.area_attack == 0 { "Single" } else { "Area" };

    // Calculate Endure (HP per Knockback). Safe fallback to full HP if KB is 0.
    let endure = if stats.knockbacks > 0 {
        (hp as f32 / stats.knockbacks as f32).round() as i32
    } else {
        hp
    };

    let cell_w = 60.0;

    ui.horizontal_top(|ui| {
        egui::Grid::new("enemy_stats_grid")
            .min_col_width(cell_w)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                grid_cell(ui, "Atk", true);
                grid_cell(ui, "Dps", true);
                grid_cell(ui, "Range", true);
                grid_cell(ui, "Atk Cycle", true);
                grid_cell(ui, "Atk Type", true); 
                ui.end_row();
                
                grid_cell(ui, &total_atk.to_string(), false); 
                grid_cell(ui, &dps.to_string(), false); 
                grid_cell(ui, &stats.standing_range.to_string(), false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cycle, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cycle, cell_w)
                ); 
                grid_cell(ui, atk_type, false); 
                ui.end_row();

                grid_cell(ui, "Hp", true);
                grid_cell(ui, "Kb", true);
                grid_cell(ui, "Speed", true);
                grid_cell(ui, "Endure", true); 
                grid_cell(ui, "Cash Drop", true);
                ui.end_row();
                
                grid_cell(ui, &hp.to_string(), false); 
                grid_cell(ui, &stats.knockbacks.to_string(), false); 
                grid_cell(ui, &stats.speed.to_string(), false);
                grid_cell(ui, &endure.to_string(), false); 
                grid_cell(ui, &format!("{}¢", stats.cash_drop), false); 
                ui.end_row();
            });
    });
}