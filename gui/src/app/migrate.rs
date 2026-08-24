// TODO: One week after `iced` release delete this entire
// file and resolve the like one error it will create
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

use kore::common::architecture;
use kore::common::dirs;

const LEGACY_APP_DIR: &str = "battle_cats_complete";
const LEGACY_PROJECT_NAME: &str = "Battle_Cats_Complete";

const LEGACY_LEFTOVER_FILES: &[&str] = &["logs.txt", "logs.prev.txt", "app.ron"];

const LEGACY_TRANSIENT: &str = "app";
const EXPORT_SCRATCH: &[&str] = &["binaries", "xapk", "unsigned_final.apk", "normalized_final.apk"];

const LEGACY_PACKAGES: &str = "packages";
const PULLED_PACKAGE: &str = "jp.co.ponos.battlecats";

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

    notes.push(move_path(from, to).map_or_else(
        |err| Note::Warn(format!("Failed to migrate {}: {}", label, err)),
        |()| Note::Info(format!("Migrated {} to {:?}", label, to)),
    ));
}

fn is_export_scratch(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut entries| {
        entries.any(|entry| {
            entry.is_ok_and(|entry| entry.file_name().to_str().is_some_and(|name| EXPORT_SCRATCH.contains(&name)))
        })
    })
}

fn relocate_pulled_packages(notes: &mut Vec<Note>) {
    let legacy = Path::new(architecture::GAME).join(LEGACY_TRANSIENT);

    if !legacy.is_dir() {
        return;
    }

    let _ = fs::create_dir_all(architecture::WORK);

    let destination = Path::new(architecture::WORK).join("import");

    migrate_entry(&legacy, &destination, "pulled android packages", notes);
}

fn drop_export_scratch(notes: &mut Vec<Note>) {
    let Ok(entries) = fs::read_dir(architecture::MODS) else {
        return;
    };

    for entry in entries.flatten() {
        let scratch = entry.path().join(LEGACY_TRANSIENT);

        if !scratch.is_dir() || !is_export_scratch(&scratch) {
            continue;
        }

        notes.push(fs::remove_dir_all(&scratch).map_or_else(
            |err| Note::Warn(format!("Failed to remove leftover export scratch {:?}: {}", scratch, err)),
            |()| Note::Info(format!("Removed leftover export scratch {:?}", scratch)),
        ));
    }
}

fn drop_extract_scratch(notes: &mut Vec<Note>) {
    let leftover = Path::new(architecture::MODS).join(LEGACY_PACKAGES);

    if !leftover.is_dir() {
        return;
    }

    let scratch = fs::read_dir(&leftover).is_ok_and(|entries| {
        entries.flatten().all(|entry| {
            entry.file_name().to_str().is_some_and(|name| name.starts_with(PULLED_PACKAGE))
        })
    });

    if !scratch {
        notes.push(Note::Warn(format!("{:?} holds unrecognised content and is now listed as a mod", leftover)));
        return;
    }

    notes.push(fs::remove_dir_all(&leftover).map_or_else(
        |err| Note::Warn(format!("Failed to remove leftover extract scratch {:?}: {}", leftover, err)),
        |()| Note::Info(format!("Removed leftover extract scratch {:?}", leftover)),
    ));
}

pub(crate) fn transient() -> Vec<Note> {
    let mut notes = Vec::new();

    relocate_pulled_packages(&mut notes);
    drop_export_scratch(&mut notes);
    drop_extract_scratch(&mut notes);

    notes
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
