pub mod export;
pub mod import;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::common::resolver;

use super::mods::export::ExportState;
use super::mods::import::ModImportState;

fn default_source() -> String {
    "Battle Cats Complete".to_string()
}

fn default_package() -> String {
    "".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)] pub title: String,
    #[serde(default)] pub author: String,
    #[serde(default)] pub version: String,
    #[serde(default)] pub description: String,
    #[serde(default = "default_package")] pub package: String,
    #[serde(default = "default_source")] pub source: String,
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            package: default_package(),
            source: default_source(),
        }
    }
}

impl ModMetadata {
    pub fn load<P: AsRef<Path>>(mod_folder_path: P) -> Self {
        let meta_path = mod_folder_path.as_ref().join("patch").join("metadata.json");
        if let Ok(data) = fs::read_to_string(meta_path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save<P: AsRef<Path>>(&self, mod_folder_path: P) -> Result<(), std::io::Error> {
        let dl_dir = mod_folder_path.as_ref().join("patch");

        if !dl_dir.exists() {
            let _ = fs::create_dir_all(&dl_dir);
        }

        let meta_path = dl_dir.join("metadata.json");
        let data = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, data)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModData {
    pub folder_name: String,
    pub enabled: bool,
    #[serde(skip)] pub metadata: ModMetadata,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModDataState {
    pub search_query: String,
    pub selected_mod: Option<String>,
    #[serde(skip)] pub loaded_mods: Vec<ModData>,
    #[serde(skip)] pub rename_buffer: String,
    pub import: ModImportState,
    pub export: ExportState,
}

impl ModDataState {
    pub fn refresh_mods(&mut self) {
        let mods_dir = std::path::Path::new("mods");
        if !mods_dir.exists() { return; }

        let mut current_folders = std::collections::HashSet::new();

        if let Ok(entries) = std::fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.file_name() != "packages" {
                    let folder_name = entry.file_name().to_string_lossy().to_string();
                    current_folders.insert(folder_name.clone());

                    if !self.loaded_mods.iter().any(|m| m.folder_name == folder_name) {
                        let metadata = ModMetadata::load(mods_dir.join(&folder_name));

                        self.loaded_mods.push(ModData {
                            folder_name,
                            enabled: false,
                            metadata,
                        });
                    }
                }
            }
        }

        self.loaded_mods.retain(|m| current_folders.contains(&m.folder_name));
        let active = self.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone());
        resolver::set_active_mod(active);
    }
}