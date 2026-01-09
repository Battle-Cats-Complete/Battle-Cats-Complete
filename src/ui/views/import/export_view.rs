use eframe::egui;
use std::sync::mpsc;
use std::thread;
use crate::core::import::{ImportState, game_data};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label("Package sorted files into a ZIP archive.");
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label("Compression Level:");
        ui.add(egui::Slider::new(&mut state.compression_level, 0..=9));
    });
    ui.add_space(15.0);
    let can_zip = state.rx.is_none(); 
    if ui.add_enabled(can_zip, egui::Button::new("Create game.zip")).clicked() {
        state.status_message = "Preparing to zip...".to_string();
        state.log_content.clear();
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        let level = state.compression_level;
        thread::spawn(move || {
            if let Err(e) = game_data::create_game_zip(tx.clone(), level) {
                 let _ = tx.send(format!("Error Zipping: {}", e));
            }
        });
    }
}