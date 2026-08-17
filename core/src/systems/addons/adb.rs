pub mod bridge;
pub mod driver;
pub mod mods;

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::systems::addons::DownloadConfig;
use crate::systems::addons::paths::{get_tools_dir, AddonStatus, ADB_BIN};


pub struct AdbManager {
    pub status: AddonStatus,
}

impl Default for AdbManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
        }
    }
}

impl AdbManager {
    pub fn install(&mut self) -> DownloadConfig {
        let asset_name = if cfg!(target_os = "windows") {
            "adb_win.zip"
        } else if cfg!(target_os = "macos") {
            "adb_mac.zip"
        } else {
            "adb_linux.zip"
        };

        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());

        DownloadConfig {
            folder_name: "adb".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: ADB_BIN.to_string(),
        }
    }

    pub fn uninstall(&mut self) {
        let _ = driver::run_command(&["kill-server"]);
        thread::sleep(Duration::from_millis(200));
        let dir = get_adb_dir();
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
        self.status = AddonStatus::NotInstalled;
    }
}

fn get_adb_dir() -> PathBuf {
    get_tools_dir().join("adb")
}

pub(crate) fn get_adb_path() -> Option<PathBuf> {
    let bin = get_adb_dir().join(ADB_BIN);
    if bin.exists() { Some(bin) } else { None }
}

pub fn is_installed() -> bool {
    get_adb_path().is_some()
}