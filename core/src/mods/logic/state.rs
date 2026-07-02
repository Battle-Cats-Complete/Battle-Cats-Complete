use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use serde::{Deserialize, Serialize};

use crate::addons::adb::mods::ModAdbEvent;
use crate::global::region::Region;

use super::metadata::ModMetadata;

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum ExportType {
    #[default]
    Apk,
    Bcm,
    Pack,
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum ModImportTab {
    #[default]
    Adb,
    Bcm,
    Pack,
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum PatchMode {
    #[default]
    Update,
    Create,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExportState {
    #[serde(skip)] pub is_open: bool,
    #[serde(skip)] pub is_busy: bool,
    pub tab: ExportType,
    pub patch_mode: PatchMode,
    pub target_region: Region,
    pub app_title: String,
    pub package_suffix: String,
    pub pack_name: String,
    #[serde(skip)] pub selected_apk: Option<PathBuf>,
    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_busy: false,
            tab: ExportType::Apk,
            patch_mode: PatchMode::Update,
            target_region: Region::En,
            app_title: String::new(),
            package_suffix: String::new(),
            pack_name: String::new(),
            selected_apk: None,
            status_message: "Ready to export.".to_string(),
            log_content: String::new(),
        }
    }
}

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ModPackType {
    Apk,
    Pack,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModData {
    pub folder_name: String,
    pub enabled: bool,
    #[serde(skip)] pub metadata: ModMetadata,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ModImportState {
    pub is_open: bool,
    pub tab: ModImportTab,
    pub package_suffix: String,
    pub pack_type: ModPackType,
    #[serde(skip)] pub is_busy: bool,
    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    #[serde(skip)] pub adb_rx: Option<Receiver<ModAdbEvent>>,
    #[serde(skip)] pub pack_rx: Option<Receiver<String>>,
}

impl Default for ModImportState {
    fn default() -> Self {
        Self {
            is_open: false,
            tab: ModImportTab::Adb,
            package_suffix: String::new(),
            pack_type: ModPackType::Apk,
            is_busy: false,
            status_message: String::new(),
            log_content: String::new(),
            adb_rx: None,
            pack_rx: None,
        }
    }
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
    #[serde(skip)] pub needs_rescan: bool,
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
        crate::global::resolver::set_active_mod(active);
    }
}