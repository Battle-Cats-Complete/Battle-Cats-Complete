pub mod android;
pub mod engine;
pub mod pack;
pub mod raw;

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::common::region::Region;
use crate::domains::{cat, enemy, stage};
use crate::ContentStore;

pub use engine::manifest::Fault as PackFault;

pub fn pack_index() -> Result<HashMap<String, String>, PackFault> {
    engine::manifest::index()
}

pub fn pack_stamp() -> Option<SystemTime> {
    engine::manifest::stamp()
}

pub fn purge_derived_caches() {
    cat::scanner::purge();
    enemy::scanner::purge();
    stage::scanner::purge();
    ContentStore::purge();
}

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
    pub import_path: String,
    pub import_mode: ImportMode,
    pub adb_target: AdbTarget,
    pub decrypt_path: String,
}

impl Default for DataConfigState {
    fn default() -> Self {
        Self {
            import_path: String::new(),
            import_mode: ImportMode::Folder,
            adb_target: AdbTarget::Specific(Region::En),
            decrypt_path: String::new(),
        }
    }
}
