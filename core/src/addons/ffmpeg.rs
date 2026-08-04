pub mod encoding;

use std::fs;
use std::path::PathBuf;

use tracing::{error, info};

use crate::addons::DownloadConfig;
use crate::addons::paths::{get_tools_dir, AddonStatus, FFMPEG_BIN};

pub struct FfmpegManager {
    pub status: AddonStatus,
}

impl Default for FfmpegManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
        }
    }
}

impl FfmpegManager {
    pub fn install(&mut self) -> DownloadConfig {
        info!("Starting FFmpeg installation...");
        let asset_name = if cfg!(target_os = "windows") {
            "ffmpeg_win.zip"
        } else if cfg!(target_os = "macos") {
            "ffmpeg_mac.zip"
        } else {
            "ffmpeg_linux.zip"
        };

        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());

        DownloadConfig {
            folder_name: "ffmpeg".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: FFMPEG_BIN.to_string(),
        }
    }

    pub fn uninstall(&mut self) {
        info!("Uninstalling FFmpeg...");
        let dir = get_ffmpeg_dir();

        if dir.exists()
            && let Err(err) = fs::remove_dir_all(&dir) {
            error!("Failed to remove FFmpeg directory: {}", err);
        }

        self.status = AddonStatus::NotInstalled;
    }
}

fn get_ffmpeg_dir() -> PathBuf {
    get_tools_dir().join("ffmpeg")
}

pub(crate) fn get_ffmpeg_path() -> Option<PathBuf> {
    let bin = get_ffmpeg_dir().join(FFMPEG_BIN);
    if bin.exists() { Some(bin) } else { None }
}

pub fn is_installed() -> bool {
    get_ffmpeg_path().is_some()
}