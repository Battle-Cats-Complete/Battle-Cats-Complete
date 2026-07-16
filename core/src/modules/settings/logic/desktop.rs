#![cfg(target_os = "linux")]
use std::env;
use std::fs;
use std::path::PathBuf;

fn get_base_directory(sub_path: &str) -> Option<PathBuf> {
    env::var("HOME").ok().map(|home_directory| {
        let mut path = PathBuf::from(home_directory);
        path.push(sub_path);
        path
    })
}

pub fn is_desktop_data_present() -> bool {
    if let Some(applications_directory) = get_base_directory(".local/share/applications") {
        let desktop_file_path = applications_directory.join("battle_cats_complete.desktop");
        desktop_file_path.exists()
    } else {
        false
    }
}

pub fn create_desktop_data() -> Result<(), String> {
    let applications_directory = get_base_directory(".local/share/applications")
        .ok_or("Could not find HOME directory")?;

    let icons_directory = get_base_directory(".local/share/icons")
        .ok_or("Could not find HOME directory")?;

    fs::create_dir_all(&applications_directory).map_err(|error| error.to_string())?;
    fs::create_dir_all(&icons_directory).map_err(|error| error.to_string())?;

    let icon_path = icons_directory.join("battle_cats_complete.png");
    let image_data = image::load_from_memory(crate::common::assets::ICON)
        .map_err(|error| format!("Failed to load embedded icon: {}", error))?;

    image_data.save_with_format(&icon_path, image::ImageFormat::Png)
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
        Icon=battle_cats_complete\n\
        Terminal=false\n\
        Categories=Development;Game;\n\
        X-AppVersion={}\n",
        executable_string,
        working_directory_string,
        cargo_version
    );

    let desktop_file_path = applications_directory.join("battle_cats_complete.desktop");
    fs::write(desktop_file_path, desktop_file_content)
        .map_err(|error| format!("Failed to write .desktop file: {}", error))?;

    Ok(())
}

pub fn delete_desktop_data() -> Result<(), String> {
    if let Some(applications_directory) = get_base_directory(".local/share/applications") {
        let desktop_file_path = applications_directory.join("battle_cats_complete.desktop");
        if desktop_file_path.exists() {
            fs::remove_file(desktop_file_path).map_err(|error| error.to_string())?;
        }
    }

    if let Some(icons_directory) = get_base_directory(".local/share/icons") {
        let icon_path = icons_directory.join("battle_cats_complete.png");
        if icon_path.exists() {
            fs::remove_file(icon_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

pub fn sync_desktop_data() -> Result<(), String> {
    if !is_desktop_data_present() {
        return Ok(());
    }

    let applications_directory = get_base_directory(".local/share/applications")
        .ok_or("Could not find HOME directory")?;
    let desktop_file_path = applications_directory.join("battle_cats_complete.desktop");

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
    let expected_path_line = format!("Path=\"{}\"", working_directory_string);
    let expected_version_line = format!("X-AppVersion={}", cargo_version);

    if !file_content.contains(&expected_exec_line)
        || !file_content.contains(&expected_path_line)
        || !file_content.contains(&expected_version_line)
    {
        create_desktop_data()?;
    }

    Ok(())
}