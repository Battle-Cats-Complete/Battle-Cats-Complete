pub mod encoding;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use tracing::{error, info};

use crate::modules::addons::manager::{self, DownloadConfig};
use crate::modules::addons::toolpaths::{get_tools_dir, AddonStatus, AVIF_BIN};

pub struct AvifManager {
    pub status: AddonStatus,
    rx: Option<Receiver<AddonStatus>>,
}

impl Default for AvifManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
            rx: None,
        }
    }
}

impl AvifManager {
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
        info!("Starting avifenc installation...");
        let asset_name = if cfg!(target_os = "windows") {
            "avifenc_win.zip"
        } else if cfg!(target_os = "macos") {
            "avifenc_mac.zip"
        } else {
            "avifenc_linux.zip"
        };

        let config = DownloadConfig {
            folder_name: "avifenc".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: AVIF_BIN.to_string(),
        };

        self.rx = Some(manager::start_download(config));
        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());
    }

    pub fn uninstall(&mut self) {
        info!("Uninstalling avifenc...");
        let dir = get_avif_dir();
        if dir.exists() {
            if let Err(err) = fs::remove_dir_all(dir) {
                error!("Failed to remove avifenc directory: {}", err);
            }
        }
        self.status = AddonStatus::NotInstalled;
    }
}

pub fn get_avif_dir() -> PathBuf {
    get_tools_dir().join("avifenc")
}

pub fn get_avif_path() -> Option<PathBuf> {
    let bin = get_avif_dir().join(AVIF_BIN);
    if bin.exists() { Some(bin) } else { None }
}

pub fn is_installed() -> bool {
    get_avif_path().is_some()
}