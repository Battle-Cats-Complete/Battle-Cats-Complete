pub mod apk;
pub mod bcm;
pub mod modify;
pub mod pack;
pub mod sign;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::common::region::Region;

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum ExportType {
    #[default]
    Apk,
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
    pub tab: ExportType,
    pub patch_mode: PatchMode,
    pub target_region: Region,
    pub app_title: String,
    pub package_suffix: String,
    pub pack_name: String,
    #[serde(skip)] pub selected_apk: Option<PathBuf>,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            tab: ExportType::Apk,
            patch_mode: PatchMode::Update,
            target_region: Region::En,
            app_title: String::new(),
            package_suffix: String::new(),
            pack_name: String::new(),
            selected_apk: None,
        }
    }
}
