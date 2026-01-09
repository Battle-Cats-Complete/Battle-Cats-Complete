use eframe::egui;
use std::sync::mpsc;
use std::thread;
use crate::core::import::{ImportState, ImportMode, game_data, sort};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Import/Extract game files.").strong());
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

    ui.add_space(5.0);
    
    // [OPTIONAL] You can re-add the Region selector here if you want it visible for Import too.
    // For now, defaulting to EN or using the state's selected_region if Dev.
    
    ui.add_space(15.0);
    let can_start = state.selected_path != "No source selected" && state.rx.is_none() && state.import_mode != ImportMode::None;
    
    if ui.add_enabled(can_start, egui::Button::new("Start Import")).clicked() {
        state.status_message = "Starting worker...".to_string();
        state.log_content.clear();
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        
        let path = state.selected_path.clone();
        let mode = state.import_mode;
        
        // Simple default for public build, or use state.selected_region for dev
        #[cfg(feature = "dev")]
        let region = state.selected_region.code().to_string();
        #[cfg(not(feature = "dev"))]
        let region = "en".to_string();

        thread::spawn(move || {
            let extract_res = match mode {
                ImportMode::Folder => game_data::import_all_from_folder(&path, &region, tx.clone()),
                ImportMode::Zip => game_data::import_all_from_zip(&path, &region, tx.clone()),
                _ => Err("Invalid mode".to_string()),
            };

            if let Err(e) = extract_res {
                let _ = tx.send(format!("Error Extracting: {}", e));
                return;
            }
            
            let _ = tx.send("Starting Sort...".to_string());
            if let Err(e) = sort::sort_game_files(tx.clone()) {
                let _ = tx.send(format!("Error Sorting: {}", e));
                return;
            }
            let _ = tx.send("Success! Files extracted and sorted.".to_string());
        });
    }
}