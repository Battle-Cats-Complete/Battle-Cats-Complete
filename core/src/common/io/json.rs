use std::fs;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn get_app_data_dir() -> PathBuf {
    let mut path = if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
    };
    path.push("battle_cats_complete");
    path.push("data");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn save<T: Serialize>(filename: &str, data: &T) {
    let mut path = get_app_data_dir();
    path.push(filename);

    if let Ok(json) = serde_json::to_string_pretty(data) {
        let tmp_path = path.with_extension("tmp");
        if fs::write(&tmp_path, json).is_ok() {
            let _ = fs::rename(&tmp_path, &path);
        }
    }
}

pub fn load<T: DeserializeOwned>(filename: &str) -> Option<T> {
    let mut path = get_app_data_dir();
    path.push(filename);

    if path.exists()
        && let Ok(data) = fs::read_to_string(&path)
        && let Ok(parsed) = serde_json::from_str::<T>(&data) {
        return Some(parsed);
    }
    None
}