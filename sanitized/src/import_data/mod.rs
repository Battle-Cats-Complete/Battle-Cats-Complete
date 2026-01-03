use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::path::{Path, PathBuf};
use std::env;

pub mod game_data;
pub mod sort;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ImportMode {
    None,
    Folder,
    Zip,
}

pub struct ImportState {
    selected_path: String,
    import_mode: ImportMode,
    status_message: String,
    log_content: String,
    rx: Option<Receiver<String>>,
    reset_trigger: Option<f64>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_path: "No source selected".to_owned(),
            import_mode: ImportMode::None,
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
        }
    }
}

impl ImportState {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut finished_just_now = false;

        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));

                if self.status_message.contains("Success") || self.status_message.contains("Aborted") || self.status_message.contains("Error") {
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
                self.selected_path = "No source selected".to_string();
                self.import_mode = ImportMode::None;
            } else {
                ctx.request_repaint();
            }
        }

        finished_just_now
    }
}

// Helper to obfuscate sensitive user paths
fn censor_path(path: &str) -> String {
    if path == "No source selected" {
        return path.to_string();
    }

    let mut clean = path.to_string();

    // Attempt to redact specific username
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() {
             clean = clean.replace(&user, "***");
        }
    }

    // Truncate long paths to show only the last 3 segments
    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().collect();
    
    if components.len() > 3 {
        let count = components.len();
        // Take the last 3 components
        let last_parts: PathBuf = components.iter().skip(count.saturating_sub(3)).collect();
        return format!("...{}{}", std::path::MAIN_SEPARATOR, last_parts.display());
    }

    clean
}

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Import Game Data");
        ui.add_space(7.5);

        let btn_enabled = state.rx.is_none();

        ui.horizontal(|ui| {
            // Button 1: Select Raw Folder
            let btn_folder = egui::Button::new("Select Raw Folder");
            let resp_folder = ui.add_enabled(btn_enabled, btn_folder);
            
            // Logic Split: Check click first...
            if resp_folder.clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    state.selected_path = path.display().to_string();
                    state.import_mode = ImportMode::Folder;
                    state.status_message = "Raw Folder selected.".to_string();
                    state.log_content.clear();
                }
            }
            // ...Then apply hover text. This prevents the "Z-fighting" flicker.
            resp_folder.on_hover_text("Select a folder containing raw decrypted game data so it can be sorted into the system!");

            ui.add_space(0.0);

            // Button 2: Select game.zip
            let btn_zip = egui::Button::new("Select game.zip");
            let resp_zip = ui.add_enabled(btn_enabled, btn_zip);

            if resp_zip.clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Archive", &["zip"])
                    .pick_file() 
                {
                    state.selected_path = path.display().to_string();
                    state.import_mode = ImportMode::Zip;
                    state.status_message = "Zip Archive selected.".to_string();
                    state.log_content.clear();
                }
            }
            resp_zip.on_hover_text("Select a 'game.zip' file provided by the community that contains only essential pre-sorted files!");
        });

        ui.add_space(5.0);
        ui.monospace(censor_path(&state.selected_path));
        
        ui.add_space(5.0);

        // Button 3: Start File Sort
        let can_start = state.import_mode != ImportMode::None && state.rx.is_none();
        
        if ui.add_enabled(can_start, egui::Button::new("Start File Sort")).clicked() {
            state.status_message = "Starting worker...".to_string();
            state.log_content.clear();
            
            let (tx, rx) = mpsc::channel();
            state.rx = Some(rx);

            let path = state.selected_path.clone();
            let mode = state.import_mode;

            thread::spawn(move || {
                let result = match mode {
                    ImportMode::Folder => game_data::import_from_folder(&path, tx.clone()),
                    ImportMode::Zip => game_data::import_from_zip(&path, tx.clone()),
                    _ => Err("Invalid mode".to_string()),
                };

                match result {
                    Ok(_) => {
                        if mode == ImportMode::Folder {
                            let _ = tx.send("Starting Sort...".to_string());
                            match sort::sort_game_files(tx.clone()) {
                                Ok(_) => { let _ = tx.send("Success! Files imported and sorted.".to_string()); },
                                Err(e) => { let _ = tx.send(format!("Error Sorting: {}", e)); }
                            }
                        } else {
                            let _ = tx.send("Success! Archive extracted.".to_string());
                        }
                    },
                    Err(e) => { let _ = tx.send(format!("Error: {}", e)); }
                }
            });
        }

        ui.separator();
        
        if state.rx.is_some() && !state.status_message.contains("Success") && !state.status_message.contains("Error") && !state.status_message.contains("Aborted") {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.status_message);
            });
        } else {
            if state.status_message.contains("Error") || state.status_message.contains("Aborted") {
                ui.colored_label(egui::Color32::RED, &state.status_message);
            } else if state.status_message.contains("Success") {
                ui.colored_label(egui::Color32::GREEN, &state.status_message);
            } else {
                ui.colored_label(egui::Color32::LIGHT_BLUE, &state.status_message);
            }
        }

        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Eliminate vertical gaps between log lines
                ui.spacing_mut().item_spacing.y = 0.0;

                for line in state.log_content.lines() {
                    if line.contains("was found!") {
                        ui.label(egui::RichText::new(line).color(egui::Color32::GREEN).monospace());
                    } else if line.contains("Error") || line.contains("Aborted") {
                        ui.label(egui::RichText::new(line).color(egui::Color32::RED).monospace());
                    } else {
                        ui.label(egui::RichText::new(line).monospace());
                    }
                }
            })
    });
}