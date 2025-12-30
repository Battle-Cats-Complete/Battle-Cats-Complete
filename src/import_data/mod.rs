use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
pub mod game_data;
pub mod crypto;
pub mod sort;

pub struct ImportState {
    selected_folder: String,
    status_message: String,
    log_content: String,
    rx: Option<Receiver<String>>,
    reset_trigger: Option<f64>,
    detected_region: String, 
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_folder: "No folder selected".to_owned(),
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
            detected_region: "Unknown".to_owned(),
        }
    }
}

impl ImportState {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut finished_just_now = false;

        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                
                if msg.starts_with("REGION:") {
                    let parts: Vec<&str> = msg.split(':').collect();
                    if parts.len() > 1 {
                        self.detected_region = parts[1].to_string();
                    }
                    continue; 
                }

                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));

                if self.status_message.contains("Success") || self.status_message.contains("Error") {
                    let current_time = ctx.input(|i| i.time);
                    self.reset_trigger = Some(current_time + 5.0);
                    finished_just_now = true;
                }
            }
            
            ctx.request_repaint();
        }

        if let Some(trigger_time) = self.reset_trigger {
            let current_time = ctx.input(|i| i.time);
            if current_time >= trigger_time {
                self.status_message = "Ready".to_string();
                self.rx = None;
                self.reset_trigger = None;
                self.selected_folder = "No folder selected".to_string();
                self.detected_region = "Unknown".to_string();
            } else {
                ctx.request_repaint();
            }
        }

        finished_just_now
    }
}

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Import Game Data");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            let btn_enabled = state.rx.is_none();
            
            if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    state.selected_folder = path.display().to_string();
                    state.status_message = "Starting worker...".to_string();
                    state.detected_region = "Scanning...".to_string(); // Temporary state

                    state.log_content.clear();
                    state.log_content.push_str("Starting import process\n");

                    let (tx, rx) = mpsc::channel();
                    state.rx = Some(rx);

                    let folder = state.selected_folder.clone();

                    thread::spawn(move || {
                        match game_data::import_all_from_folder(&folder, tx.clone()) {
                            Ok(_) => {
                                let _ = tx.send("Starting Sort".to_string());
                                match sort::sort_game_files(tx.clone()) {
                                    Ok(_) => { let _ = tx.send("Success! Files extracted and sorted".to_string()); },
                                    Err(e) => { let _ = tx.send(format!("Error Sorting: {}", e)); }
                                }
                            },
                            Err(e) => { let _ = tx.send(format!("Error Extracting: {}", e)); }
                        }
                    });
                }
            }
            
            ui.label(
                egui::RichText::new(format!("Region: {}", state.detected_region))
                    .monospace()
                    .strong()
            );
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
}