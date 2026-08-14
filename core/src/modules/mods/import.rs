pub mod decrypt;
pub(crate) mod extract;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{error, info, trace};

use crate::common::job::JobEvent;
use crate::modules::import::engine::keys;

use super::{taken, ModMetadata};

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum ModImportTab {
    #[default]
    Adb,
    Bcm,
    Pack,
}

#[derive(PartialEq, Clone, Copy, Default, Serialize, Deserialize, Debug)]
pub enum ModPackType {
    #[default]
    Apk,
    Pack,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModImportState {
    pub tab: ModImportTab,
    pub package_suffix: String,
    pub pack_type: ModPackType,
}

pub fn run_bcm(path: PathBuf, enforce_validation: bool, emit: impl Fn(JobEvent) + Sync) -> Result<(), String> {
    let log = |line: String| emit(JobEvent::Log(line));

    log("Creating workspace...".to_string());

    let (workspace_dir, _name) = create_workspace(None)
        .map_err(|e| format!("Failed to construct workspace: {}", e))?;

    let user_keys = keys::verify(enforce_validation, &log)?;

    extract::run_archive(&path, &workspace_dir, &log, &user_keys)?;

    let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
    info!("BCM import finished completely. Saved as {}", final_name);
    log(format!("\nImport Complete! Saved as '{}'", final_name));
    Ok(())
}

pub fn run_pack(path: PathBuf, pack_type: ModPackType, enforce_validation: bool, emit: impl Fn(JobEvent) + Sync) -> Result<(), String> {
    let log = |line: String| emit(JobEvent::Log(line));

    let user_keys = keys::verify(enforce_validation, &log)?;

    let (workspace_dir, _name) = create_workspace(None)
        .map_err(|e| format!("Failed to construct workspace: {}", e))?;

    let res = match pack_type {
        ModPackType::Apk => extract::run_archive(&path, &workspace_dir, &log, &user_keys),
        ModPackType::Pack => decrypt::run(&path, &workspace_dir, &log, &user_keys),
    };

    if let Err(e) = res {
        error!("Pack import failed: {}", e);
        return Err(e);
    }

    let final_name = apply_metadata_rename(Path::new("mods"), &workspace_dir);
    info!("Pack import finished completely. Saved as {}", final_name);
    log(format!("\nImport Complete! Saved as '{}'", final_name));
    Ok(())
}

pub fn create_workspace(base_name: Option<&str>) -> std::io::Result<(PathBuf, String)> {
    let mods_root = Path::new("mods");
    fs::create_dir_all(mods_root)?;

    let default_name = base_name.unwrap_or("NewMod");
    let mut final_name = default_name.to_string();
    let mut counter = 1;

    if base_name.is_none() {
        while taken(mods_root, &format!("{}{}", default_name, counter)) {
            counter += 1;
        }
        final_name = format!("{}{}", default_name, counter);
    } else {
        while taken(mods_root, &final_name) {
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
        let meta = ModMetadata::load(target_dir);
        let safe_title = meta.title.replace(&['<', '>', ':', '"', '/', '\\', '|', '?', '*'][..], "").trim().to_string();

        if !safe_title.is_empty() && safe_title != final_name {
            let mut attempt = safe_title.clone();
            let mut counter = 1;
            let mut new_path = mods_root.join(&attempt);

            if new_path != target_dir {
                while !attempt.eq_ignore_ascii_case(&final_name) && taken(mods_root, &attempt) {
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
