#[cfg(feature = "dev")]
use eframe::egui;
#[cfg(feature = "dev")]
use crate::core::import::{ImportState, GameRegion, game_data, sort};
#[cfg(feature = "dev")]
use std::sync::mpsc;

#[cfg(feature = "dev")]
pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Extract and decrypt game files.").strong());
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Select Game Region:");
        let enabled = state.rx.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            ui.radio_value(&mut state.selected_region, GameRegion::Global, "Global");
            ui.radio_value(&mut state.selected_region, GameRegion::Japan, "Japan");
            ui.radio_value(&mut state.selected_region, GameRegion::Taiwan, "Taiwan");
            ui.radio_value(&mut state.selected_region, GameRegion::Korean, "Korea");
        });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let btn_enabled = state.rx.is_none();
        if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                // [FIX] Use unified setter
                state.set_path(path.display().to_string());
                state.status_message = "Folder selected.".to_string();
            }
        }
        // [FIX] Use unified field
        ui.monospace(&state.censored_path);
    });

    ui.add_space(15.0);

    // [FIX] Use unified field
    let can_start = state.selected_path != "No source selected" && state.rx.is_none();
    
    if ui.add_enabled(can_start, egui::Button::new("Start Extraction")).clicked() {
        state.status_message = "Initializing Decryptor...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let folder = state.selected_path.clone();
        let region = state.selected_region.code().to_string();

        std::thread::spawn(move || {
            // [FIX] Call the unified stable logic
            if let Err(e) = game_data::import_all_from_folder(&folder, &region, tx.clone()) {
                let _ = tx.send(format!("Error: {}", e));
                return; 
            }
            
            // Auto-sort after extract
            let _ = tx.send("Sorting extracted files...".to_string());
            if let Err(e) = sort::sort_game_files(tx.clone()) {
                let _ = tx.send(format!("Error Sorting: {}", e));
            } else {
                let _ = tx.send("Success! Extraction and sort complete.".to_string());
            }
        });
    }
}