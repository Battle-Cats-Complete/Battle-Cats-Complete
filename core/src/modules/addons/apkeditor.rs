pub mod xapk;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use tracing::{error, info};

use crate::modules::addons::manager::{self, DownloadConfig};
use crate::modules::addons::toolpaths::{get_tools_dir, AddonStatus, APKEDITOR_JAR, JAVA_BIN};

pub struct ApkeditorManager {
    pub status: AddonStatus,
    rx: Option<Receiver<AddonStatus>>,
}

impl Default for ApkeditorManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
            rx: None,
        }
    }
}

impl ApkeditorManager {
    pub fn poll(&mut self) {
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
        info!("Starting APKEditor JRE installation...");

        let asset_name = if cfg!(target_os = "windows") {
            "jre_win.zip"
        } else if cfg!(target_os = "macos") {
            "jre_mac.zip"
        } else {
            "jre_linux.zip"
        };

        let bin_name = if cfg!(target_os = "windows") { "java.exe" } else { "java" };

        let config = DownloadConfig {
            folder_name: "apkeditor".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: bin_name.to_string(),
        };

        self.rx = Some(manager::start_download(config));
        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());
    }

    pub fn uninstall(&mut self) {
        info!("Uninstalling APKEditor JRE...");
        let dir = get_apkeditor_dir();

        if dir.exists() {
            if let Err(err) = fs::remove_dir_all(&dir) {
                error!("Failed to remove APKEditor directory: {}", err);
            }
        }

        self.status = AddonStatus::NotInstalled;
    }
}

pub fn get_apkeditor_dir() -> PathBuf {
    get_tools_dir().join("apkeditor")
}

pub fn get_java_path() -> Option<PathBuf> {
    let bin = get_apkeditor_dir().join(JAVA_BIN);
    if bin.exists() { Some(bin) } else { None }
}

pub fn get_apkeditor_path() -> Option<PathBuf> {
    let jar = get_apkeditor_dir().join(APKEDITOR_JAR);
    if jar.exists() { Some(jar) } else { None }
}

pub fn is_installed() -> bool {
    get_apkeditor_path().is_some()
}