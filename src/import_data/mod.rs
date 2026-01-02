use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::path::{Path, PathBuf};
use std::env;

pub mod game_data;
pub mod crypto;
pub mod sort;
pub mod global;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum GameRegion {
    Japan,
    Taiwan,
    Korean,
    Global,
}

impl GameRegion {
    fn code(&self) -> &'static str {
        match self {
            GameRegion::Japan => "ja",
            GameRegion::Taiwan => "tw",
            GameRegion::Korean => "ko",
            GameRegion::Global => "en",
        }
    }
}

pub struct ImportState {
    selected_folder: String,
    status_message: String,
    log_content: String,
    rx: Option<Receiver<String>>,
    reset_trigger: Option<f64>,
    selected_region: GameRegion,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_folder: "No folder selected".to_owned(),
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
            selected_region: GameRegion::Global,
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
            } else {
                ctx.request_repaint();
            }
        }

        finished_just_now
    }
}

// Helper to obfuscate sensitive user paths
fn censor_path(path: &str) -> String {
    if path == "No folder selected" {
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
        ui.add_space(20.0);

        // Select folder
        ui.horizontal(|ui| {
            let btn_enabled = state.rx.is_none();
            
            if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    state.selected_folder = path.display().to_string();
                    state.status_message = "Folder selected. Please confirm Region.".to_string();
                    state.log_content.clear();
                }
            }
            ui.monospace(censor_path(&state.selected_folder));
        });
        
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

        ui.add_space(10.0);

        // Start import
        let can_start = state.selected_folder != "No folder selected" && state.rx.is_none();
        
        if ui.add_enabled(can_start, egui::Button::new("Start Import")).clicked() {
            state.status_message = "Starting worker...".to_string();
            state.log_content.clear();
            state.log_content.push_str(&format!("Starting import for region: {:?}\n", state.selected_region));

            let (tx, rx) = mpsc::channel();
            state.rx = Some(rx);

            let folder = state.selected_folder.clone();
            let region_code = state.selected_region.code().to_string();

            thread::spawn(move || {
                match game_data::import_all_from_folder(&folder, &region_code, tx.clone()) {
                    Ok(_) => {
                        let _ = tx.send("Starting Sort...".to_string());
                        match sort::sort_game_files(tx.clone()) {
                            Ok(_) => { let _ = tx.send("Success! Files extracted and sorted.".to_string()); },
                            Err(e) => { let _ = tx.send(format!("Error Sorting: {}", e)); }
                        }
                    },
                    Err(e) => { let _ = tx.send(format!("Error Extracting: {}", e)); }
                }
            });
        }

        ui.separator();
        
        if state.rx.is_some() && !state.status_message.contains("Success") && !state.status_message.contains("Error") {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.status_message);
            });
        } else {
            if state.status_message.contains("Error") {
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
                ui.monospace(&state.log_content);
            })
    });
}