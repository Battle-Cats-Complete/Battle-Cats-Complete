pub mod android;
pub mod engine;
pub mod pack;
pub mod raw;

use serde::{Deserialize, Serialize};

use crate::common::region::Region;

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbImportType {
    All,
    Update,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbTarget {
    Specific(Region),
    All,
}

impl AdbTarget {
    pub fn suffix(&self) -> &'static str {
        match self {
            AdbTarget::Specific(region) => region.metadata().package_suffix,
            AdbTarget::All => "all",
        }
    }

}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportSubTab {
    Emulator,
    Sort,
    Decrypt,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode {
    Folder,
    Zip,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct DataConfigState {
    #[serde(skip)]
    pub selected_job: Option<ImportSubTab>,
    pub import_path: String,
    pub import_mode: ImportMode,
    pub adb_target: AdbTarget,
    pub decrypt_path: String,
}

impl Default for DataConfigState {
    fn default() -> Self {
        Self {
            selected_job: None,
            import_path: String::new(),
            import_mode: ImportMode::Folder,
            adb_target: AdbTarget::Specific(Region::En),
            decrypt_path: String::new(),
        }
    }
}
