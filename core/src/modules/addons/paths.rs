use std::fs;
use std::path::PathBuf;

use tracing::{debug, error};

pub use super::{AddonStatus, Presence};

#[cfg(target_os = "windows")]
pub const ADB_BIN: &str = "adb.exe";
#[cfg(not(target_os = "windows"))]
pub const ADB_BIN: &str = "adb";

#[cfg(target_os = "windows")]
pub const AVIF_BIN: &str = "avifenc.exe";
#[cfg(not(target_os = "windows"))]
pub const AVIF_BIN: &str = "avifenc";

#[cfg(target_os = "windows")]
pub const FFMPEG_BIN: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
pub const FFMPEG_BIN: &str = "ffmpeg";

#[cfg(target_os = "windows")]
pub const JAVA_BIN: &str = "bin/java.exe";
#[cfg(not(target_os = "windows"))]
pub const JAVA_BIN: &str = "bin/java";

pub const APKEDITOR_JAR: &str = "APKEditor.jar";

pub fn get_tools_dir() -> PathBuf {
    let base_dir = if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "Battle_Cats_Complete") {
        proj_dirs.data_dir().join("tools")
    } else {
        PathBuf::from("tools")
    };

    if !base_dir.exists() {
        debug!("Tools directory missing, creating at {:?}", base_dir);
        if let Err(err) = fs::create_dir_all(&base_dir) {
            error!("Failed to create base tools directory: {}", err);
        }
    }

    base_dir
}

pub fn adb_status() -> Presence {
    if get_tools_dir().join("adb").join(ADB_BIN).exists() { Presence::Installed } else { Presence::Missing }
}

pub fn avifenc_status() -> Presence {
    if get_tools_dir().join("avifenc").join(AVIF_BIN).exists() { Presence::Installed } else { Presence::Missing }
}

pub fn ffmpeg_status() -> Presence {
    if get_tools_dir().join("ffmpeg").join(FFMPEG_BIN).exists() { Presence::Installed } else { Presence::Missing }
}

pub fn apkeditor_status() -> Presence {
    if get_tools_dir().join("apkeditor").join(APKEDITOR_JAR).exists() { Presence::Installed } else { Presence::Missing }
}