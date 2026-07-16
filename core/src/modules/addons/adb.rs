pub mod bridge;
pub mod driver;
pub mod mods;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use crate::modules::addons::{manager, DownloadConfig};
use crate::modules::addons::paths::{get_tools_dir, AddonStatus, ADB_BIN};

pub use bridge::execute_pull;
pub use mods::spawn_mod_import;

pub enum ModAdbEvent {
    Status(String),
    Success(String),
    Error(String),
}

pub struct AdbManager {
    pub status: AddonStatus,
    rx: Option<Receiver<AddonStatus>>,
}

impl Default for AdbManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
            rx: None,
        }
    }
}

impl AdbManager {
    pub fn update(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.status = msg;
            }
        }
        if let AddonStatus::Installed = self.status {
            self.rx = None;
        }
    }

    pub fn install(&mut self) {
        let asset_name = if cfg!(target_os = "windows") {
            "adb_win.zip"
        } else if cfg!(target_os = "macos") {
            "adb_mac.zip"
        } else {
            "adb_linux.zip"
        };

        let config = DownloadConfig {
            folder_name: "adb".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: ADB_BIN.to_string(),
        };

        self.rx = Some(manager::start_download(config));
        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());
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

pub fn get_adb_dir() -> PathBuf {
    get_tools_dir().join("adb")
}

pub fn get_adb_path() -> Option<PathBuf> {
    let bin = get_adb_dir().join(ADB_BIN);
    if bin.exists() { Some(bin) } else { None }
}

pub fn is_installed() -> bool {
    get_adb_path().is_some()
}