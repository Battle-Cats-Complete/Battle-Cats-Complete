use eframe::egui;
use std::sync::mpsc;
use std::thread;
use crate::core::import::{ImportState, ImportMode, game_data, sort};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Import/Restore game files.").strong());
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.label("Source:");
        ui.radio_value(&mut state.import_mode, ImportMode::Zip, "game.zip");
        ui.radio_value(&mut state.import_mode, ImportMode::Folder, "Folder");
    });
    
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let enabled = state.rx.is_none() && state.import_mode != ImportMode::None;
        if ui.add_enabled(enabled, egui::Button::new("Select Source")).clicked() {
            let res = match state.import_mode {
                ImportMode::Zip => rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file(),
                ImportMode::Folder => rfd::FileDialog::new().pick_folder(),
                _ => None,
            };
            if let Some(path) = res {
                state.set_path(path.display().to_string());
                state.status_message = "Source selected.".to_string();
                state.log_content.clear();
            }
        }
        ui.monospace(&state.censored_path);
    });

    ui.add_space(15.0);
    let can_start = state.selected_path != "No source selected" && state.rx.is_none() && state.import_mode != ImportMode::None;
    
    if ui.add_enabled(can_start, egui::Button::new("Start Import")).clicked() {
        state.status_message = "Starting worker...".to_string();
        state.log_content.clear();
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        
        let path = state.selected_path.clone();
        let mode = state.import_mode;

        thread::spawn(move || {
            let import_res = match mode {
                ImportMode::Folder => game_data::import_standard_folder(&path, tx.clone()),
                ImportMode::Zip => game_data::import_standard_zip(&path, tx.clone()),
                _ => Err("Invalid mode".to_string()),
            };

            if let Err(e) = import_res {
                let _ = tx.send(format!("Error Importing: {}", e));
                return;
            }
            
            let _ = tx.send("Starting Sort...".to_string());
            if let Err(e) = sort::sort_game_files(tx.clone()) {
                let _ = tx.send(format!("Error Sorting: {}", e));
                return;
            }
            let _ = tx.send("Success! Files imported and sorted.".to_string());
        });
    }
}