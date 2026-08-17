pub mod export;
pub mod import;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::domains::import::architecture;
use crate::{Conflict, Vault, VfsError};

use super::mods::export::ExportState;
use super::mods::import::ModImportState;

const MODS_ROOT: &str = "mods";

const RESERVED_NAMES: [&str; 2] = ["game", architecture::PACKAGES];

pub const METADATA: &str = "metadata.json";

pub fn taken(mods_root: &Path, candidate: &str) -> bool {
    if RESERVED_NAMES.iter().any(|reserved| candidate.eq_ignore_ascii_case(reserved)) {
        return true;
    }

    let Ok(entries) = fs::read_dir(mods_root) else {
        return false;
    };

    entries.flatten().any(|entry| {
        entry.file_name().to_str().is_some_and(|name| name.eq_ignore_ascii_case(candidate))
    })
}

pub fn locate(mod_dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut level = vec![mod_dir.to_path_buf()];
    let mut top = true;

    while !level.is_empty() {
        let mut hits = Vec::new();
        let mut next = Vec::new();

        for dir in &level {
            let Ok(entries) = fs::read_dir(dir) else { continue };

            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(OsStr::to_str) else { continue };

                if path.is_dir() {
                    if !top || !architecture::MOD_TRANSIENT.contains(&name) {
                        next.push(path);
                    }
                    continue;
                }

                if name == filename {
                    hits.push(path);
                }
            }
        }

        if let Some(found) = hits.into_iter().min() {
            return Some(found);
        }

        level = next;
        top = false;
    }

    None
}

pub fn enable(vault: &Vault, name: &str) -> Result<Vec<Conflict>, VfsError> {
    let path = Path::new(MODS_ROOT).join(name);
    let conflicts = vault.vfs.create(path.as_path())?;

    let keys = vault.vfs.keys(name);
    debug!(mod_name = name, files = keys.len(), "mod mounted, evicting shadowed game content");

    vault.purge(&keys);

    Ok(conflicts)
}

pub fn disable(vault: &Vault, name: &str) {
    let path = Path::new(MODS_ROOT).join(name);
    let keys = vault.vfs.keys(name);

    vault.vfs.destroy(path.as_path());

    debug!(mod_name = name, files = keys.len(), "mod unmounted, evicting stale mod content");

    vault.purge(&keys);
}

fn default_source() -> String {
    "Battle Cats Complete".to_string()
}

fn default_package() -> String {
    "".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)] pub title: String,
    #[serde(default)] pub author: String,
    #[serde(default)] pub version: String,
    #[serde(default)] pub description: String,
    #[serde(default = "default_package")] pub package: String,
    #[serde(default = "default_source")] pub source: String,
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            package: default_package(),
            source: default_source(),
        }
    }
}

impl ModMetadata {
    pub fn load<P: AsRef<Path>>(mod_folder_path: P) -> Self {
        let Some(meta_path) = locate(mod_folder_path.as_ref(), METADATA) else {
            return Self::default();
        };

        fs::read_to_string(meta_path).map_or_else(|_| Self::default(), |data| serde_json::from_str(&data).unwrap_or_default())
    }

    pub fn save<P: AsRef<Path>>(&self, mod_folder_path: P) -> Result<(), std::io::Error> {
        let root = mod_folder_path.as_ref();
        let meta_path = locate(root, METADATA).unwrap_or_else(|| root.join("patch").join(METADATA));

        if let Some(parent) = meta_path.parent()
            && !parent.exists()
        {
            let _ = fs::create_dir_all(parent);
        }

        let data = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, data)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModData {
    pub folder_name: String,
    pub enabled: bool,
    #[serde(skip)] pub metadata: ModMetadata,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModDataState {
    pub search_query: String,
    pub selected_mod: Option<String>,
    #[serde(skip)] pub loaded_mods: Vec<ModData>,
    #[serde(skip)] pub rename_buffer: String,
    pub import: ModImportState,
    pub export: ExportState,
}

impl ModDataState {
    pub fn refresh_mods(&mut self) {
        let mods_dir = Path::new(MODS_ROOT);
        if !mods_dir.exists() { return; }

        let mut current_folders = std::collections::HashSet::new();

        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.file_name() != architecture::PACKAGES {
                    let folder_name = entry.file_name().to_string_lossy().to_string();
                    current_folders.insert(folder_name.clone());

                    if !self.loaded_mods.iter().any(|m| m.folder_name == folder_name) {
                        let metadata = ModMetadata::load(mods_dir.join(&folder_name));

                        self.loaded_mods.push(ModData {
                            folder_name,
                            enabled: false,
                            metadata,
                        });
                    }
                }
            }
        }

        self.loaded_mods.retain(|m| current_folders.contains(&m.folder_name));
    }

    pub fn active_mod(&self) -> Option<String> {
        self.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone())
    }
}