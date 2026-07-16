pub mod adb;
pub mod apkeditor;
pub mod avifenc;
pub mod ffmpeg;
pub mod manager;
pub mod oem;
pub mod toolpaths;

#[derive(Clone, PartialEq, Debug)]
pub enum AddonStatus {
    NotInstalled,
    Installed,
    Downloading(f32, String),
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Presence {
    Installed,
    Missing,
}

pub struct DownloadConfig {
    pub folder_name: String,
    pub asset_name: String,
    pub binary_name: String,
}