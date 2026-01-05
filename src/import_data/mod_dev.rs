use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::path::{Path, PathBuf};
use std::env;

use super::game_data_dev as game_data;
use super::sort;
use crate::settings::Settings; // NEW IMPORT

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
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

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DataTab {
    Import,
    Export,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    selected_folder: String,
    #[serde(skip)]
    censored_folder: String,
    #[serde(skip)]
    status_message: String,
    #[serde(skip)]
    log_content: String,
    #[serde(skip)]
    rx: Option<Receiver<String>>,
    #[serde(skip)]
    reset_trigger: Option<f64>,
    
    selected_region: GameRegion,
    compression_level: i32,
    active_tab: DataTab,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_folder: "No folder selected".to_owned(),
            censored_folder: "No folder selected".to_owned(),
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
            selected_region: GameRegion::Global,
            compression_level: 6,
            active_tab: DataTab::Import,
        }
    }
}

impl ImportState {
    pub fn set_folder(&mut self, path: String) {
        self.selected_folder = path;
        self.censored_folder = censor_path(&self.selected_folder);
    }

    // CHANGED: Now accepts settings
    pub fn update(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        if self.censored_folder.is_empty() && !self.selected_folder.is_empty() {
             self.censored_folder = censor_path(&self.selected_folder);
        }

        let mut finished_just_now = false;

        if let Some(rx) = self.rx.take() {
            finished_just_now = self.process_messages(ctx, &rx);
            self.rx = Some(rx);
        }

        if let Some(trigger_time) = self.reset_trigger {
            self.check_reset_trigger(ctx, trigger_time);
        }

        // TRIGGER: Auto-detect language on success
        if finished_just_now && self.status_message.contains("Success") {
            settings.validate_and_update_language();
        }

        finished_just_now
    }

    fn process_messages(&mut self, ctx: &egui::Context, rx: &Receiver<String>) -> bool {
        let mut count = 0;
        let mut finished = false;

        while let Ok(msg) = rx.try_recv() {
            self.status_message = msg.clone();
            self.log_content.push_str(&format!("{}\n", msg));

            if self.status_message.contains("Success") || self.status_message.contains("Error") {
                let current_time = ctx.input(|i| i.time);
                self.reset_trigger = Some(current_time + 5.0);
                finished = true;
            }
            
            count += 1;
            if count > 100 { break; }
        }
        ctx.request_repaint();
        finished
    }

    fn check_reset_trigger(&mut self, ctx: &egui::Context, trigger_time: f64) {
        let current_time = ctx.input(|i| i.time);
        
        if current_time >= trigger_time {
            self.status_message = "Ready".to_string();
            self.rx = None; 
            self.reset_trigger = None;
            self.set_folder("No folder selected".to_string());
        } else {
            ctx.request_repaint();
        }
    }
}

fn censor_path(path: &str) -> String {
    if path == "No folder selected" {
        return path.to_string();
    }
    let mut clean = path.to_string();
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() {
             clean = clean.replace(&user, "***");
        }
    }
    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().collect();
    if components.len() > 3 {
        let count = components.len();
        let last_parts: PathBuf = components.iter().skip(count.saturating_sub(3)).collect();
        return format!("...{}{}", std::path::MAIN_SEPARATOR, last_parts.display());
    }
    clean
}

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Game Data Management");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            let tabs = [(DataTab::Import, "Import"), (DataTab::Export, "Export")];
            for (tab, label) in tabs {
                let is_selected = state.active_tab == tab;
                let (fill, stroke, text_color) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                };
                if ui.add(egui::Button::new(egui::RichText::new(label).color(text_color)).fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(80.0, 30.0))).clicked() {
                    state.active_tab = tab;
                }
            }
        });

        ui.add_space(15.0);

        match state.active_tab {
            DataTab::Import => show_import_ui(ui, state),
            DataTab::Export => show_export_ui(ui, state),
        }

        ui.add_space(15.0);
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
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            ui.monospace(&state.log_content);
        })
    });
}

fn show_import_ui(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Extract game files from a local folder.").strong());
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let btn_enabled = state.rx.is_none();
        if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                state.set_folder(path.display().to_string());
                state.status_message = "Folder selected. Please confirm Region.".to_string();
                state.log_content.clear();
            }
        }
        ui.monospace(&state.censored_folder);
    });
    ui.add_space(5.0);
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
    ui.add_space(15.0);
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
            execute_import_task(folder, region_code, tx);
        });
    }
}

fn execute_import_task(folder: String, region_code: String, tx: Sender<String>) {
    if let Err(e) = game_data::import_all_from_folder(&folder, &region_code, tx.clone()) {
        let _ = tx.send(format!("Error Extracting: {}", e));
        return;
    }
    let _ = tx.send("Starting Sort...".to_string());
    if let Err(e) = sort::sort_game_files(tx.clone()) {
        let _ = tx.send(format!("Error Sorting: {}", e));
        return;
    }
    let _ = tx.send("Success! Files extracted and sorted.".to_string());
}

fn show_export_ui(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Package sorted files into a ZIP archive.").strong());
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
            execute_export_task(level, tx);
        });
    }
}

fn execute_export_task(level: i32, tx: Sender<String>) {
    if let Err(e) = game_data::create_game_zip(tx.clone(), level) {
         let _ = tx.send(format!("Error Zipping: {}", e));
    }
}