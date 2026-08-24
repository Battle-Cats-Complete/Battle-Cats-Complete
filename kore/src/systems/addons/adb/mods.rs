use std::fs;
use std::path::{Path, PathBuf};

use tracing::{error, info};

use crate::common::architecture;
use crate::common::job::JobEvent;
use crate::domains::import::engine::keys;
use crate::domains::mods::import::{apply_metadata_rename, create_workspace, extract};

use super::driver;

pub fn run(suffix: String, enforce_validation: bool, emit: impl Fn(JobEvent) + Sync) -> Result<(), String> {
    let log = |line: String| emit(JobEvent::Log(line));

    let user_keys = keys::verify(enforce_validation, &log)?;

    let _work = architecture::Scratch::claim();
    let pkg = format!("jp.co.ponos.battlecats{}", suffix);

    let imported = pull_base_apk(&pkg, &log).and_then(|apk| {
        let (workspace_dir, _name) = create_workspace(None, &log)
            .map_err(|error| format!("Failed to construct workspace: {}", error))?;

        log("Extracting DownloadLocal data...".to_string());

        extract::run_archive(&apk, &workspace_dir, &log, &user_keys)
            .map_err(|error| format!("Extraction/Decryption failed: {}", error))?;

        Ok(apply_metadata_rename(Path::new(architecture::MODS), &workspace_dir))
    });

    let final_name = imported.inspect_err(|error| error!("ADB mod import failed: {}", error))?;

    info!("ADB mod import finished completely. Saved as {}", final_name);
    log(format!("\nImport Complete! Saved as '{}'", final_name));
    Ok(())
}

fn pull_base_apk(pkg: &str, log: &impl Fn(String)) -> Result<PathBuf, String> {
    log("Starting ADB Server...".to_string());
    let _ = driver::run_command(&["start-server"]);

    log(format!("Targeting Package: {}", pkg));

    let Some(serial) = driver::find_usb_device().or_else(driver::find_emulator) else {
        return Err("No device found.".to_string());
    };

    let target_dir = Path::new(architecture::WORK).join("extract").join(pkg);

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|error| format!("Failed to create the pull directory: {}", error))?;
    }

    log(format!("Pulling base.apk for {}...", pkg));

    let pm_path = driver::run_command(&["-s", &serial, "shell", "pm", "path", pkg]).unwrap_or_default();
    let remote_path = pm_path
        .lines()
        .find(|line| line.contains("base.apk"))
        .unwrap_or("")
        .trim()
        .strip_prefix("package:")
        .unwrap_or("");

    if remote_path.is_empty() {
        return Err(format!("Could not find base.apk for {}", pkg));
    }

    let local_apk_path = target_dir.join("base.apk");

    let Some(local_apk_str) = local_apk_path.to_str() else {
        return Err("Invalid local APK path.".to_string());
    };

    if driver::run_command(&["-s", &serial, "pull", remote_path, local_apk_str]).is_err() {
        return Err("Failed to pull base.apk from device.".to_string());
    }

    Ok(local_apk_path)
}
