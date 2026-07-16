use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use tracing::{error, info, trace, warn};

use crate::modules::addons::adb::{ModAdbEvent, spawn_mod_import};
use crate::modules::mods::import::{decrypt, extract};
use super::state::{ModDataState, ModPackType};

fn format_log_name(name: &str, path: &Path) -> String {
    let cleaned = name.trim_start_matches("...\\").trim_start_matches(".../");
    if cleaned.len() > 35 {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let trunc_stem: String = stem.chars().take(20).collect();
        if !ext.is_empty() && ext.len() <= 5 {
            format!("{}....{}", trunc_stem, ext)
        } else {
            format!("{}...", trunc_stem)
        }
    } else {
        cleaned.to_string()
    }
}

pub fn process_events(state: &mut ModDataState) -> bool {
    process_adb_events(state);
    process_pack_events(state);
    state.import.is_busy
}

fn process_adb_events(state: &mut ModDataState) {
    let Some(rx) = &state.import.adb_rx else { return; };

    while let Ok(event) = rx.try_recv() {
        match event {
            ModAdbEvent::Status(msg) => {
                state.import.status_message = msg.clone();
                state.import.log_content.push_str(&format!("{}\n", msg));
            }
            ModAdbEvent::Success(msg) => {
                state.import.status_message = msg.clone();
                state.import.log_content.push_str(&format!("SUCCESS: {}\n", msg));
                state.import.is_busy = false;
            }
            ModAdbEvent::Error(msg) => {
                state.import.status_message = format!("Error: {}", msg);
                state.import.log_content.push_str(&format!("ERROR: {}\n", msg));
                state.import.is_busy = false;
            }
        }
    }
}

fn process_pack_events(state: &mut ModDataState) {
    let Some(rx) = &state.import.pack_rx else { return; };

    while let Ok(msg) = rx.try_recv() {
        state.import.status_message = msg.clone();
        state.import.log_content.push_str(&format!("{}\n", msg));

        let lower_msg = msg.to_lowercase();
        if lower_msg.contains("complete!") || lower_msg.contains("error") || lower_msg.contains("failed") {
            state.import.is_busy = false;
        }
    }
}

pub fn start_adb_import(state: &mut ModDataState) {
    state.import.log_content.clear();
    state.import.status_message = "Initializing Mod ADB Pull...".to_string();
    state.import.is_busy = true;

    let (tx, rx) = mpsc::channel();
    state.import.adb_rx = Some(rx);
    spawn_mod_import(tx, state.import.package_suffix.clone());
}

pub fn start_bcm_import(state: &mut ModDataState, path: PathBuf) {
    state.import.log_content.clear();
    state.import.status_message = format!("Extracting {:?}...", path.file_name().unwrap_or_default());
    state.import.is_busy = true;

    let (tx, rx) = mpsc::channel();
    state.import.pack_rx = Some(rx);

    thread::spawn(move || {
        let _ = tx.send("Creating workspace...".to_string());

        let (workspace_dir, _name) = match create_workspace(None) {
            Ok(res) => res,
            Err(e) => {
                let _ = tx.send(format!("Error: Failed to construct workspace: {}", e));
                return;
            }
        };

        let settings: crate::modules::settings::logic::state::Settings = crate::common::io::json::load("settings.json").unwrap_or_default();
        let user_keys = match crate::modules::data::utilities::keys::verify(settings.game_data.enforce_key_validation, &tx) {
            Ok(keys) => keys,
            Err(_) => return,
        };

        if let Err(e) = extract::run_archive(&path, &workspace_dir, tx.clone(), &user_keys) {
            let _ = tx.send(format!("Error: {}", e));
            return;
        }

        let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
        info!("BCM import finished completely. Saved as {}", final_name);
        let _ = tx.send(format!("\nImport Complete! Saved as '{}'", final_name));
    });
}

pub fn start_pack_import(state: &mut ModDataState, path: PathBuf) {
    let pack_type = state.import.pack_type;

    state.import.log_content.clear();
    state.import.status_message = format!("Processing {:?}...", path.file_name().unwrap_or_default());
    state.import.is_busy = true;

    let (tx, rx) = mpsc::channel();
    state.import.pack_rx = Some(rx);

    thread::spawn(move || {
        let settings: crate::modules::settings::logic::state::Settings = crate::common::io::json::load("settings.json").unwrap_or_default();

        let user_keys = match crate::modules::data::utilities::keys::verify(settings.game_data.enforce_key_validation, &tx) {
            Ok(keys) => keys,
            Err(_) => return,
        };

        let (workspace_dir, _name) = match create_workspace(None) {
            Ok(res) => res,
            Err(e) => {
                let _ = tx.send(format!("Error: Failed to construct workspace: {}", e));
                return;
            }
        };

        let res = match pack_type {
            ModPackType::Apk => extract::run_archive(&path, &workspace_dir, tx.clone(), &user_keys),
            ModPackType::Pack => decrypt::run(&path, &workspace_dir, tx.clone(), &user_keys),
        };

        match res {
            Ok(_) => {
                let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
                info!("Pack import finished completely. Saved as {}", final_name);
                let _ = tx.send(format!("\nImport Complete! Saved as '{}'", final_name));
            }
            Err(e) => {
                error!("Pack import failed: {}", e);
                let _ = tx.send(format!("Error: {}", e));
            }
        }
    });
}

pub fn start_raw_import(state: &mut ModDataState, is_folder: bool, path_opt: Option<PathBuf>, files: Vec<PathBuf>) {
    state.import.log_content.clear();
    state.import.is_busy = true;
    state.import.status_message = "Copying raw files...".to_string();

    let (tx, rx) = mpsc::channel();
    state.import.pack_rx = Some(rx);

    thread::spawn(move || {
        let _ = tx.send("Creating workspace...".to_string());

        let (workspace_dir, _name) = match create_workspace(None) {
            Ok(res) => res,
            Err(e) => {
                let _ = tx.send(format!("Error: Failed to construct workspace: {}", e));
                return;
            }
        };

        if is_folder {
            let Some(p) = path_opt else { return; };
            let total_files = count_files_recursive(&p);
            let log_interval = (total_files / 10).max(1);
            let mut processed = 0;

            if let Err(e) = copy_dir_all_with_log(&p, &workspace_dir, &tx, total_files, log_interval, &mut processed) {
                let _ = tx.send(format!("Error copying folder: {}", e));
            } else {
                let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
                info!("Raw folder import finished. Saved as {}", final_name);
                let _ = tx.send(format!("\nImport Complete! Saved as '{}'", final_name));
            }
            return;
        }

        let total_files = files.len();
        let log_interval = (total_files / 10).max(1);
        let mut processed = 0;

        for file in files {
            let Some(name) = file.file_name() else { continue; };
            if let Err(e) = fs::copy(&file, workspace_dir.join("loose").join(name)) {
                warn!("Error copying individual file {:?}: {}", name, e);
                let _ = tx.send(format!("Error copying file {:?}: {}", name, e));
            } else {
                processed += 1;
                if processed % log_interval == 0 || processed == total_files {
                    let display_name = format_log_name(&name.to_string_lossy(), &file);
                    trace!("Copied raw file: {}", display_name);
                    let _ = tx.send(format!("Extracted {} Files | Streaming: {}", processed, display_name));
                }
            }
        }

        let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
        info!("Raw file import finished. Saved as {}", final_name);
        let _ = tx.send(format!("\nImport Complete! Saved as '{}'", final_name));
    });
}

fn count_files_recursive(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                count += count_files_recursive(&entry.path());
            } else {
                count += 1;
            }
        }
    }
    count
}

fn copy_dir_all_with_log(
    src: &Path,
    dst: &Path,
    tx: &mpsc::Sender<String>,
    total: usize,
    interval: usize,
    processed: &mut usize
) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all_with_log(&entry.path(), &dst.join(entry.file_name()), tx, total, interval, processed)?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
            *processed += 1;

            if (*processed).is_multiple_of(interval) || *processed == total {
                let name = entry.file_name().to_string_lossy().into_owned();
                let display_name = format_log_name(&name, &entry.path());
                trace!("Copied folder content: {}", display_name);
                let _ = tx.send(format!("Extracted {} Files | Streaming: {}", *processed, display_name));
            }
        }
    }
    Ok(())
}

pub fn create_workspace(base_name: Option<&str>) -> std::io::Result<(PathBuf, String)> {
    let mods_root = Path::new("mods");
    fs::create_dir_all(mods_root)?;

    let default_name = base_name.unwrap_or("NewMod");
    let mut final_name = default_name.to_string();
    let mut counter = 1;

    if base_name.is_none() {
        while mods_root.join(format!("{}{}", default_name, counter)).exists() {
            counter += 1;
        }
        final_name = format!("{}{}", default_name, counter);
    } else {
        while mods_root.join(&final_name).exists() {
            final_name = format!("{}{}", default_name, counter);
            counter += 1;
        }
    }

    let workspace_dir = mods_root.join(&final_name);
    fs::create_dir_all(&workspace_dir)?;
    fs::create_dir_all(workspace_dir.join("patch"))?;
    fs::create_dir_all(workspace_dir.join("loose"))?;
    fs::create_dir_all(workspace_dir.join("icons"))?;

    trace!("Workspace generated: {}", final_name);
    Ok((workspace_dir, final_name))
}

pub fn apply_metadata_rename(mods_root: &Path, target_dir: &Path) -> String {
    let mut final_name = target_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
    let meta_path = target_dir.join("patch").join("metadata.json");

    if meta_path.exists() {
        let meta = crate::modules::mods::logic::metadata::ModMetadata::load(target_dir);
        let safe_title = meta.title.replace(&['<', '>', ':', '"', '/', '\\', '|', '?', '*'][..], "").trim().to_string();

        if !safe_title.is_empty() && safe_title != final_name {
            let mut attempt = safe_title.clone();
            let mut counter = 1;
            let mut new_path = mods_root.join(&attempt);

            if new_path != target_dir {
                while new_path.exists() {
                    attempt = format!("{}{}", safe_title, counter);
                    new_path = mods_root.join(&attempt);
                    counter += 1;
                }

                if fs::rename(target_dir, &new_path).is_ok() {
                    trace!("Renamed workspace to metadata target: {}", attempt);
                    final_name = attempt;
                }
            }
        }
    }
    final_name
}

pub fn delete_mod_folder(path: PathBuf) {
    thread::spawn(move || {
        trace!("Deleting mod folder: {:?}", path);
        let _ = fs::remove_dir_all(path);
    });
}