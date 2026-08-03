// TODO: One week after `iced` release delete this entire
// file and resolve the like one error it will create
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

use core::common::dirs;

const LEGACY_APP_DIR: &str = "battle_cats_complete";
const LEGACY_PROJECT_NAME: &str = "Battle_Cats_Complete";

const LEGACY_LEFTOVER_FILES: &[&str] = &["logs.txt", "logs.prev.txt", "app.ron"];

enum Destination {
    Config,
    State,
}

const MIGRATIONS: &[(&str, Destination)] = &[
    ("settings.json", Destination::Config),
    ("exceptions.json", Destination::Config),
    ("keys.json", Destination::Config),
    ("identity.pem", Destination::Config),
    ("state.json", Destination::State),
    ("meta.json", Destination::State),
];

pub(crate) enum Note {
    Info(String),
    Warn(String),
}

fn legacy_config_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        PathBuf::from(env::var("APPDATA").ok()?)
    } else {
        PathBuf::from(env::var("HOME").ok()?).join(".config")
    };

    Some(base.join(LEGACY_APP_DIR).join("data"))
}

fn legacy_tools_dir() -> Option<PathBuf> {
    let project_dirs = ProjectDirs::from("", "", LEGACY_PROJECT_NAME)?;
    Some(project_dirs.data_dir().join("tools"))
}

fn legacy_data_local_dir() -> Option<PathBuf> {
    Some(BaseDirs::new()?.data_local_dir().join(LEGACY_APP_DIR))
}

fn cleanup_legacy_directories() {
    if let Some(legacy_config) = legacy_config_dir() {
        let _ = fs::remove_dir(&legacy_config);
        if let Some(legacy_root) = legacy_config.parent() {
            let _ = fs::remove_dir(legacy_root);
        }
    }

    if let Some(legacy_data_local) = legacy_data_local_dir() {
        let _ = fs::remove_dir_all(legacy_data_local.join("cache"));

        for leftover in LEGACY_LEFTOVER_FILES {
            let _ = fs::remove_file(legacy_data_local.join(leftover));
        }

        let _ = fs::remove_dir(&legacy_data_local);
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|err| err.to_string())?;

    for entry in fs::read_dir(from).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let source = entry.path();
        let target = to.join(entry.file_name());

        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

fn move_path(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }

    if from.is_dir() {
        copy_tree(from, to)?;
        fs::remove_dir_all(from).map_err(|err| err.to_string())
    } else {
        fs::copy(from, to).map_err(|err| err.to_string())?;
        fs::remove_file(from).map_err(|err| err.to_string())
    }
}

fn migrate_entry(from: &Path, to: &Path, label: &str, notes: &mut Vec<Note>) {
    if !from.exists() || to.exists() {
        return;
    }

    match move_path(from, to) {
        Ok(()) => notes.push(Note::Info(format!("Migrated {} to {:?}", label, to))),
        Err(err) => notes.push(Note::Warn(format!("Failed to migrate {}: {}", label, err))),
    }
}

pub(crate) fn run() -> Vec<Note> {
    let mut notes = Vec::new();

    if let Some(legacy_config) = legacy_config_dir() {
        for (filename, destination) in MIGRATIONS {
            let new_directory = match destination {
                Destination::Config => dirs::config(),
                Destination::State => dirs::state(),
            };

            let Some(new_directory) = new_directory else {
                notes.push(Note::Warn(format!("No destination directory available for {}", filename)));
                continue;
            };

            migrate_entry(&legacy_config.join(filename), &new_directory.join(filename), filename, &mut notes);
        }
    }

    if let (Some(legacy_tools), Some(data_directory)) = (legacy_tools_dir(), dirs::data()) {
        migrate_entry(&legacy_tools, &data_directory.join("tools"), "tools directory", &mut notes);
    }

    cleanup_legacy_directories();

    notes
}
