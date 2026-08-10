#![cfg(target_os = "linux")]

use std::env;
use std::fs;
use std::path::PathBuf;

use image::ImageFormat;
use tracing::{debug, info, warn};

use crate::common::assets;
use crate::common::dirs::APP_DIR;

const LEGACY_DESKTOP_FILE: &str = "battle_cats_complete.desktop";
const LEGACY_ICON_FILE: &str = "battle_cats_complete.png";

fn desktop_file_name() -> String {
    format!("{}.desktop", APP_DIR)
}

fn icon_file_name() -> String {
    format!("{}.png", APP_DIR)
}

fn get_base_directory(sub_path: &str) -> Option<PathBuf> {
    env::var("HOME").ok().map(|home_directory| {
        let mut base_path = PathBuf::from(home_directory);
        base_path.push(sub_path);
        base_path
    })
}

pub fn is_desktop_data_present() -> bool {
    get_base_directory(".local/share/applications").is_some_and(|dir| dir.join(desktop_file_name()).exists())
}

fn remove_legacy_desktop_data() -> bool {
    let mut had_legacy_entry = false;

    if let Some(applications_directory) = get_base_directory(".local/share/applications") {
        let legacy_desktop_path = applications_directory.join(LEGACY_DESKTOP_FILE);
        if legacy_desktop_path.exists() {
            had_legacy_entry = true;
            if let Err(error) = fs::remove_file(&legacy_desktop_path) {
                warn!("Failed to remove legacy .desktop entry: {}", error);
            } else {
                debug!("Removed legacy .desktop entry file.");
            }
        }
    }

    if let Some(icons_directory) = get_base_directory(".local/share/icons") {
        let legacy_icon_path = icons_directory.join(LEGACY_ICON_FILE);
        if legacy_icon_path.exists() {
            if let Err(error) = fs::remove_file(&legacy_icon_path) {
                warn!("Failed to remove legacy application icon: {}", error);
            } else {
                debug!("Removed legacy desktop application icon.");
            }
        }
    }

    had_legacy_entry
}

pub fn create_desktop_data() -> Result<(), String> {
    let applications_directory = get_base_directory(".local/share/applications")
        .ok_or("Could not find HOME directory")?;

    let icons_directory = get_base_directory(".local/share/icons")
        .ok_or("Could not find HOME directory")?;

    fs::create_dir_all(&applications_directory).map_err(|error| error.to_string())?;
    fs::create_dir_all(&icons_directory).map_err(|error| error.to_string())?;

    let icon_path = icons_directory.join(icon_file_name());
    let image_data = image::load_from_memory(assets::ICON)
        .map_err(|error| format!("Failed to load embedded icon: {}", error))?;

    image_data.save_with_format(&icon_path, ImageFormat::Png)
        .map_err(|error| format!("Failed to save PNG icon: {}", error))?;

    let current_executable = env::current_exe()
        .map_err(|error| format!("Could not get executable path: {}", error))?;

    let executable_string = current_executable.to_str()
        .ok_or("Executable path contains invalid UTF-8")?;

    let working_directory = current_executable.parent()
        .ok_or("Could not determine parent directory of executable")?;

    let working_directory_string = working_directory.to_str()
        .ok_or("Working directory path contains invalid UTF-8")?;

    let cargo_version = env!("CARGO_PKG_VERSION");

    let desktop_file_content = format!(
        "[Desktop Entry]\n\
        Version=1.0\n\
        Type=Application\n\
        Name=Battle Cats Complete\n\
        Comment=Toolkit for The Battle Cats\n\
        Exec=\"{}\"\n\
        Path={}\n\
        Icon={}\n\
        Terminal=false\n\
        Categories=Development;Game;\n\
        StartupWMClass={}\n\
        X-AppVersion={}\n",
        executable_string,
        working_directory_string,
        APP_DIR,
        APP_DIR,
        cargo_version
    );

    let desktop_file_path = applications_directory.join(desktop_file_name());
    fs::write(desktop_file_path, desktop_file_content)
        .map_err(|error| format!("Failed to write .desktop file: {}", error))?;

    info!("Successfully created desktop entry and icon metadata.");
    Ok(())
}

pub fn delete_desktop_data() -> Result<(), String> {
    remove_legacy_desktop_data();

    if let Some(applications_directory) = get_base_directory(".local/share/applications") {
        let desktop_file_path = applications_directory.join(desktop_file_name());
        if desktop_file_path.exists() {
            fs::remove_file(desktop_file_path).map_err(|error| error.to_string())?;
            debug!("Removed .desktop entry file.");
        }
    }

    if let Some(icons_directory) = get_base_directory(".local/share/icons") {
        let icon_path = icons_directory.join(icon_file_name());
        if icon_path.exists() {
            fs::remove_file(icon_path).map_err(|error| error.to_string())?;
            debug!("Removed desktop application icon.");
        }
    }

    Ok(())
}

pub fn sync_desktop_data() -> Result<(), String> {
    if remove_legacy_desktop_data() {
        info!("Legacy desktop entry found. Regenerating under the new application name.");
        return create_desktop_data();
    }

    if !is_desktop_data_present() {
        return Ok(());
    }

    let applications_directory = get_base_directory(".local/share/applications")
        .ok_or("Could not find HOME directory")?;
    let desktop_file_path = applications_directory.join(desktop_file_name());

    let file_content = fs::read_to_string(&desktop_file_path)
        .map_err(|error| format!("Failed to read .desktop file: {}", error))?;

    let current_executable = env::current_exe()
        .map_err(|error| format!("Could not get executable path: {}", error))?;

    let executable_string = current_executable.to_str()
        .ok_or("Executable path contains invalid UTF-8")?;

    let working_directory = current_executable.parent()
        .ok_or("Could not determine parent directory of executable")?;

    let working_directory_string = working_directory.to_str()
        .ok_or("Working directory path contains invalid UTF-8")?;

    let cargo_version = env!("CARGO_PKG_VERSION");

    let expected_exec_line = format!("Exec=\"{}\"", executable_string);
    let expected_path_line = format!("Path={}", working_directory_string);
    let expected_wm_class_line = format!("StartupWMClass={}", APP_DIR);
    let expected_version_line = format!("X-AppVersion={}", cargo_version);

    if !file_content.contains(&expected_exec_line)
        || !file_content.contains(&expected_path_line)
        || !file_content.contains(&expected_wm_class_line)
        || !file_content.contains(&expected_version_line)
    {
        debug!("Desktop entry is out of date. Initiating sync regeneration...");
        create_desktop_data()?;
    }

    Ok(())
}