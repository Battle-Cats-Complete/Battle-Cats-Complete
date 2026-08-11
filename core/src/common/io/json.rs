use std::fs;
use std::io::{self, Error};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::common::dirs;

pub fn save<T: Serialize>(filename: &str, data: &T) -> io::Result<()> {
    let directory = dirs::config().ok_or_else(|| Error::other("Config directory unavailable"))?;
    save_in(&directory, filename, data)
}

pub fn load<T: DeserializeOwned>(filename: &str) -> Option<T> {
    load_in(&dirs::config()?, filename)
}

pub fn save_state<T: Serialize>(filename: &str, data: &T) -> io::Result<()> {
    let directory = dirs::state().ok_or_else(|| Error::other("State directory unavailable"))?;
    save_in(&directory, filename, data)
}

pub fn load_state<T: DeserializeOwned>(filename: &str) -> Option<T> {
    load_in(&dirs::state()?, filename)
}

fn save_in<T: Serialize>(directory: &Path, filename: &str, data: &T) -> io::Result<()> {
    let path = directory.join(filename);

    let json = serde_json::to_string_pretty(data).map_err(Error::other)?;
    let tmp_path = super::hidden_temp(&path);
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, &path)
}

fn load_in<T: DeserializeOwned>(directory: &Path, filename: &str) -> Option<T> {
    let path = directory.join(filename);

    if path.exists()
        && let Ok(data) = fs::read_to_string(&path)
        && let Ok(parsed) = serde_json::from_str::<T>(&data) {
        return Some(parsed);
    }
    None
}
