use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::path::{Path, PathBuf};
use std::env;
use eframe::egui;

// Logic Modules
pub mod game_data; 
pub mod sort;

use crate::core::settings::Settings;

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum GameRegion {
    Japan, Taiwan, Korean, Global,
}

impl GameRegion {
    pub fn code(&self) -> &'static str {
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
    #[cfg(feature = "dev")] Extract, 
    Import, 
    Export 
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode { None, Folder, Zip }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    // [UNIFIED STATE] Replaces separate folder/zip/extract variables
    pub selected_path: String,
    #[serde(skip)] pub censored_path: String,
    
    pub active_tab: DataTab,
    pub import_mode: ImportMode,
    
    #[cfg(feature = "dev")] pub selected_region: GameRegion,
    pub compression_level: i32,

    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    #[serde(skip)] pub rx: Option<Receiver<String>>,
    #[serde(skip)] pub reset_trigger: Option<f64>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_path: "No source selected".to_owned(),
            censored_path: "No source selected".to_owned(),
            active_tab: DataTab::Import,
            import_mode: ImportMode::None,
            #[cfg(feature = "dev")] selected_region: GameRegion::Global,
            compression_level: 6,
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
        }
    }
}

impl ImportState {
    // Unified Setter
    pub fn set_path(&mut self, path: String) {
        self.selected_path = path;
        self.censored_path = censor_path(&self.selected_path);
    }

    pub fn update(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        // Ensure path is censored if loaded from save file
        if self.censored_path.is_empty() && !self.selected_path.is_empty() {
             self.censored_path = censor_path(&self.selected_path);
        }

        let mut finished_just_now = false;

        if let Some(rx) = self.rx.take() {
            let mut done = false;
            // [CRITICAL] The working "Drain Loop"
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));
                
                if self.status_message.contains("Success") || self.status_message.contains("Error") {
                    let current_time = ctx.input(|i| i.time);
                    self.reset_trigger = Some(current_time + 5.0);
                    done = true;
                }
            }
            if done { finished_just_now = true; }
            self.rx = Some(rx); // Put it back for next frame
            ctx.request_repaint();
        }

        if let Some(trigger_time) = self.reset_trigger {
            let current_time = ctx.input(|i| i.time);
            if current_time >= trigger_time {
                self.status_message = "Ready".to_string();
                self.rx = None; 
                self.reset_trigger = None;
                self.set_path("No source selected".to_string());
                self.import_mode = ImportMode::None;
            } else {
                ctx.request_repaint();
            }
        }

        // Trigger settings scan on success
        if finished_just_now && self.status_message.contains("Success") {
            settings.rx_lang = Some(crate::core::settings::lang::start_scan());
        }

        finished_just_now
    }
}

fn censor_path(path: &str) -> String {
    if path == "No source selected" { return path.to_string(); }
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() { return path.replace(&user, "***"); }
    }
    path.to_string()
}