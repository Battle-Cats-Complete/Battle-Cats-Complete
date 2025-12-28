use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
pub mod game_data;
pub mod crypto;
pub mod sort;

// We need a struct to hold the state SPECIFIC to this page
pub struct ImportState {
    selected_folder: String,
    status_message: String,
    log_content: String,
    rx: Option<Receiver<String>>,
    reset_trigger: Option<f64>,
}

// Default values for this page
impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_folder: "No folder selected".to_owned(),
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
        }
    }
}

pub fn show(ctx: &egui::Context, state: &mut ImportState) -> bool {
    let mut import_just_finished = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Import Game Data");
        ui.add_space(20.0);

        // If we have a receiver (rx), check for new messages
        if let Some(rx) = &state.rx {
            while let Ok(msg) = rx.try_recv() {
                state.status_message = msg.clone();

                state.log_content.push_str(&format!("{}\n", msg));

                // If we receive "Done!", we can kill the connection
                if state.status_message.contains("Success") || state.status_message.contains("Error") {
                    let current_time = ctx.input(|i| i.time);
                    state.reset_trigger = Some(current_time + 5.0);
                    import_just_finished = true;
                }
            }
            // Force the screen to redraw so we see the text update instantly
            ctx.request_repaint();
        }

        if let Some(trigger_time) = state.reset_trigger {
            let current_time = ctx.input(|i| i.time);

            if current_time >= trigger_time {
                state.status_message = "Ready".to_string();
                state.rx = None;
                state.reset_trigger = None;
                state.selected_folder = "No folder selected".to_string();
            } else {
                ctx.request_repaint();
            }
        }

        ui.horizontal(|ui| {
            let btn_enabled = state.rx.is_none();
            
            if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    state.selected_folder = path.display().to_string();
                    state.status_message = "Starting worker...".to_string();

                    state.log_content.clear();
                    state.log_content.push_str("Starting import process\n");

                    let (tx, rx) = mpsc::channel();
                    
                    state.rx = Some(rx);

                    let folder = state.selected_folder.clone();

                    // Spawn the thread
                    thread::spawn(move || {
                        // Run decryption/extraction
                        match game_data::import_all_from_folder(&folder, tx.clone()) {
                            Ok(_) => {
                                let _ = tx.send("Starting Sort".to_string());

                                match sort::sort_game_files(tx.clone()) {
                                    Ok(_) => { let _ = tx.send("Success! Files extracted and storted".to_string()); },
                                    Err(e) => { let _ = tx.send(format!("Error Sorting: {}", e)); }
                                }
                            },
                            Err(e) => { let _ = tx.send(format!("Error Extracting: {}", e)); }
                        }
                    });
                }
            }
            ui.label(egui::RichText::new(&state.selected_folder).monospace());
        });

        ui.add_space(10.0);
        
        if state.rx.is_some() && !state.status_message.contains("Success") {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.status_message);
            });
        } else {
            ui.colored_label(egui::Color32::LIGHT_BLUE, &state.status_message);
        }

        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.monospace(&state.log_content);
            })
        });
        import_just_finished
    }
