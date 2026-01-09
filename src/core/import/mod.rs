use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::env;
use eframe::egui;

pub mod game_data; 
pub mod sort;

use crate::core::settings::Settings;

#[cfg(feature = "dev")]
#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum GameRegion {
    Japan, Taiwan, Korean, Global,
}

#[cfg(feature = "dev")]
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
    #[cfg(feature = "dev")] Decrypt, 
    Import, 
    Export 
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode { None, Folder, Zip }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    pub selected_path: String,
    #[serde(skip)] pub censored_path: String,
    
    pub active_tab: DataTab,
    pub import_mode: ImportMode,
    
    pub export_filename: String,
    
    #[cfg(feature = "dev")] pub selected_region: GameRegion,
    pub compression_level: i32,

    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    #[serde(skip)] pub rx: Option<Receiver<String>>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_path: String::new(),
            censored_path: String::new(),
            
            #[cfg(feature = "dev")]
            active_tab: DataTab::Decrypt,
            #[cfg(not(feature = "dev"))]
            active_tab: DataTab::Import,

            import_mode: ImportMode::Zip,
            
            export_filename: String::new(),
            
            #[cfg(feature = "dev")] selected_region: GameRegion::Global,
            compression_level: 6,
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
        }
    }
}

impl ImportState {
    pub fn set_path(&mut self, path: String) {
        self.selected_path = path;
        self.censored_path = censor_path(&self.selected_path);
    }

    pub fn update(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        if self.censored_path.is_empty() && !self.selected_path.is_empty() {
             self.censored_path = censor_path(&self.selected_path);
        }

        let mut finished_just_now = false;

        if let Some(rx) = self.rx.take() {
            let mut job_finished = false;
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));
                
                if self.status_message.contains("Success") || self.status_message.contains("Error") {
                    job_finished = true;
                }
            }
            
            if job_finished {
                finished_just_now = true; 
            } else {
                self.rx = Some(rx);
                ctx.request_repaint();
            }
        }

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