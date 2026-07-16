use std::env;
use std::path::Path;

use eframe::egui;
use serde::{Deserialize, Serialize};

use core::modules::data::state::DataConfigState;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    pub config: DataConfigState,
    #[serde(skip)] pub import_censored: String,
    #[serde(skip)] pub decrypt_censored: String,
}

impl ImportState {
    pub fn update(&mut self, egui_context: &egui::Context) -> bool {
        self.import_censored = censor_path(&self.config.import_path);
        self.decrypt_censored = censor_path(&self.config.decrypt_path);

        let flags = self.config.tick_threads();

        if flags.needs_repaint {
            egui_context.request_repaint();
        }

        flags.import_finished_just_now
    }
}

pub fn censor_path(path_string: &str) -> String {
    if path_string.is_empty() || path_string == "No source selected" { return String::new(); }

    let mut clean_string = path_string.to_string();
    if let Ok(username) = env::var("USERNAME").or_else(|_| env::var("USER"))
        && !username.is_empty() { clean_string = clean_string.replace(&username, "***"); }

    let path_object = Path::new(&clean_string);
    let path_components: Vec<_> = path_object.components().map(|component| component.as_os_str().to_string_lossy()).collect();

    if path_components.len() < 2 {
        if clean_string.chars().count() > 20 {
            return format!("...{}", clean_string.chars().skip(clean_string.chars().count() - 20).collect::<String>());
        }
        return clean_string;
    }

    let mut parent_folder = path_components[path_components.len()-2].to_string();
    let mut target_file = path_components[path_components.len()-1].to_string();

    let total_length = parent_folder.chars().count() + target_file.chars().count();

    if total_length > 20 {
        if target_file.chars().count() >= 20 {
            target_file = format!("{}...", target_file.chars().take(18).collect::<String>());
            parent_folder = String::new();
        } else {
            let allowed_parent_length = 20 - target_file.chars().count();
            if allowed_parent_length > 2 {
                parent_folder = format!("{}...", parent_folder.chars().take(allowed_parent_length - 2).collect::<String>());
            } else {
                parent_folder = String::new();
            }
        }
    }

    let ellipsis_prefix = if path_components.len() > 2 { "...\\" } else { "" };

    if parent_folder.is_empty() {
        format!("{}{}", ellipsis_prefix, target_file)
    } else {
        format!("{}{}\\{}", ellipsis_prefix, parent_folder, target_file)
    }
}