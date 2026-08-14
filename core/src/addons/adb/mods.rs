use std::fs;
use std::path::PathBuf;

use crate::addons::adb::driver;
use crate::common::job::JobEvent;
use crate::modules::import::engine::keys;
use crate::modules::mods::import::extract;

pub fn run(suffix: String, enforce_validation: bool, emit: impl Fn(JobEvent) + Sync) -> Result<(), String> {
    let log = |line: String| emit(JobEvent::Log(line));

    log("Starting ADB Server...".to_string());
    let _ = driver::run_command(&["start-server"]);

    let pkg = format!("jp.co.ponos.battlecats{}", suffix);
    log(format!("Targeting Package: {}", pkg));

    let Some(serial) = driver::find_usb_device().or_else(driver::find_emulator) else {
        return Err("No device found.".to_string());
    };

    let target_dir = PathBuf::from(format!("mods/packages/{}", pkg));
    if !target_dir.exists() {
        let _ = fs::create_dir_all(&target_dir);
    }

    log(format!("Pulling base.apk for {}...", pkg));

    let pm_path = driver::run_command(&["-s", &serial, "shell", "pm", "path", &pkg]).unwrap_or_default();
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

    log("Extracting DownloadLocal data...".to_string());

    let user_keys = keys::verify(enforce_validation, &log)?;

    if let Err(e) = extract::run_archive(&local_apk_path, &target_dir, &log, &user_keys) {
        return Err(format!("Extraction/Decryption failed: {}", e));
    }

    log("Cleaning up temporary base.apk and pack files...".to_string());
    let _ = fs::remove_dir_all(&target_dir);

    log("ADB Mod Import Complete!".to_string());
    Ok(())
}
