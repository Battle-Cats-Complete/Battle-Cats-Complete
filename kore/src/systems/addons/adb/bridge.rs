use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::common::region::Region;
use crate::domains::import::{AdbImportType, AdbTarget};
use crate::domains::mining::Build;

use super::driver;

const BASE_APK: &str = "base.apk";
const MODIFIED_APP: &str = "File modification suspected, do a clean install on device.";

#[derive(Default)]
pub(crate) struct Pull {
    pub(crate) directories: Vec<PathBuf>,
    pub(crate) builds: Vec<Build>,
}

#[derive(Clone, Copy)]
pub(crate) struct PullOptions {
    pub(crate) import_mode: AdbImportType,
    pub(crate) ignore_modified_app: bool,
}

pub(crate) fn execute_pull(
    base_output_directory: &Path,
    options: PullOptions,
    target_region: AdbTarget,
    emit_log: &(dyn Fn(String) + Sync),
    abort_flag: &AtomicBool
) -> Result<Pull, String> {

    emit_log("Starting ADB Server...".to_string());
    let _ = driver::run_command(&["kill-server"]);
    thread::sleep(Duration::from_millis(500));
    let _ = driver::run_command(&["start-server"]);

    if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

    let (mut current_serial, fallback_ip_address) = establish_connection(emit_log)?;

    emit_log("Device Verified.".to_string());
    if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

    if options.import_mode == AdbImportType::All {
        ensure_root_access(&mut current_serial, emit_log, abort_flag)?;
    }

    let regions_to_process = match target_region {
        AdbTarget::All => vec![
            AdbTarget::Specific(Region::En),
            AdbTarget::Specific(Region::Ja),
            AdbTarget::Specific(Region::Tw),
            AdbTarget::Specific(Region::Ko)
        ],
        _ => vec![target_region],
    };

    let mut pulled = Pull::default();

    for current_region in regions_to_process.iter() {
        if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }
        pull_region_data(
            current_region, &mut current_serial, &fallback_ip_address,
            base_output_directory, options, emit_log, &mut pulled
        );
    }

    let _ = driver::run_command(&["kill-server"]);

    if pulled.directories.is_empty() {
        return Err("No regions were successfully pulled.".to_string());
    }

    Ok(pulled)
}

fn read_build(serial_number: &str, package_name: &str) -> Option<Build> {
    let dump = driver::run_command(&["-s", serial_number, "shell", "dumpsys", "package", package_name]).ok()?;

    let name = field(&dump, "versionName=")?;

    if name.is_empty() {
        return None;
    }

    let code = field(&dump, "versionCode=").and_then(|value| value.parse().ok()).unwrap_or_default();

    Some(Build { code, name, label: package_name.to_string() })
}

fn field(dump: &str, key: &str) -> Option<String> {
    let tail = dump.split(key).nth(1)?;
    let value = tail.split_whitespace().next()?;

    Some(value.to_string())
}

fn establish_connection(emit_log: &(dyn Fn(String) + Sync)) -> Result<(String, Option<String>), String> {
    emit_log("Detecting device...".to_string());

    if let Some((serial, fallback)) = try_usb_connection(emit_log) {
        return Ok((serial, fallback));
    }

    if let Some(serial) = try_mdns_connection(emit_log) {
        return Ok((serial, None));
    }

    if let Some(serial) = try_emulator_connection(emit_log) {
        return Ok((serial, None));
    }

    if let Some(serial) = try_waydroid_connection(emit_log) {
        return Ok((serial, None));
    }

    Err("No device found. Ensure Wireless Debugging is ON or Emulator is running.".to_string())
}

fn try_usb_connection(emit_log: &(dyn Fn(String) + Sync)) -> Option<(String, Option<String>)> {
    let usb_serial = driver::find_usb_device()?;
    driver::verify_connection(&usb_serial).ok()?;

    emit_log(format!("USB Device Found: {}", usb_serial));
    let fallback = driver::enable_wireless_fallback(&usb_serial);
    Some((usb_serial, fallback))
}

fn try_mdns_connection(emit_log: &(dyn Fn(String) + Sync)) -> Option<String> {
    emit_log("Scanning network for Wireless Debugging...".to_string());
    let mdns_target = driver::find_mdns_device()?;

    emit_log(format!("Found via mDNS: {}", mdns_target));
    driver::connect_to_address(&mdns_target).ok()?;

    let stable_ip = driver::bootstrap_tcpip(&mdns_target)?;
    let _ = driver::run_command(&["disconnect", &mdns_target]);

    let stable_serial = driver::connect_to_address(&stable_ip).ok()?;
    driver::verify_connection(&stable_serial).ok()?;

    emit_log("Auto-Connection Successful!".to_string());
    Some(stable_serial)
}

fn try_emulator_connection(emit_log: &(dyn Fn(String) + Sync)) -> Option<String> {
    emit_log("Scanning for Emulators...".to_string());
    let emulator_serial = driver::find_emulator()?;
    driver::verify_connection(&emulator_serial).ok()?;
    Some(emulator_serial)
}

fn try_waydroid_connection(_emit_log: &(dyn Fn(String) + Sync)) -> Option<String> {
    let waydroid_ip = "192.168.240.112:5555";
    driver::connect_to_address(waydroid_ip).ok()?;
    driver::verify_connection(waydroid_ip).ok()?;
    Some(waydroid_ip.to_string())
}

fn ensure_root_access(current_serial: &mut String, emit_log: &(dyn Fn(String) + Sync), abort_flag: &AtomicBool) -> Result<(), String> {
    emit_log("Checking Root Permissions...".to_string());
    let root_check_cmd = "su -c 'echo root_test'";
    let root_test_output = driver::run_command(&["-s", current_serial, "shell", root_check_cmd]).unwrap_or_default();

    if root_test_output.contains("root_test") {
        emit_log("Root access confirmed via su.".to_string());
        return Ok(());
    }

    emit_log("Requesting Root Access...".to_string());
    let _ = driver::run_command(&["-s", current_serial, "root"]);
    thread::sleep(Duration::from_secs(3));

    if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

    if current_serial.contains(':') {
        let _ = driver::connect_wireless(current_serial);
    } else if !current_serial.starts_with("emulator")
        && let Some(new_serial) = driver::find_usb_device() {
        *current_serial = new_serial;
    }

    emit_log("Waiting for device to reconnect...".to_string());
    let _ = driver::run_command(&["-s", current_serial, "wait-for-device"]);
    Ok(())
}

fn pull_region_data(
    current_region: &AdbTarget,
    current_serial: &mut String,
    fallback_ip_address: &Option<String>,
    base_output_directory: &Path,
    options: PullOptions,
    emit_log: &(dyn Fn(String) + Sync),
    pulled: &mut Pull
) {
    let region_suffix = current_region.suffix();
    let package_name = format!("jp.co.ponos.battlecats{}", region_suffix);
    let check_installed_output = driver::run_command(&["-s", current_serial, "shell", "pm", "path", &package_name]).unwrap_or_default();

    if check_installed_output.trim().is_empty() || check_installed_output.contains("Error") {
        emit_log(format!("Skipping {}: Not installed.", package_name));
        return;
    }

    emit_log(format!("Pulling {}...", package_name));

    if let Some(build) = read_build(current_serial, &package_name) {
        emit_log(format!("Device reports {} v{}", package_name, build.name));
        pulled.builds.push(build);
    }

    let target_directory = base_output_directory.join(&package_name);

    let Err(process_error) = process_single_region_adb(emit_log, current_serial, &package_name, &target_directory, options) else {
        pulled.directories.push(target_directory);
        return;
    };

    let is_app_warning = process_error.contains("Root Copy Failed") || process_error.contains("APK Path not found") || process_error == MODIFIED_APP;

    if is_app_warning {
        emit_log(format!("Skipping {}: {}", package_name, process_error));
        return;
    }

    let Some(rescue_ip_address) = fallback_ip_address else {
        emit_log(format!("Skipping {} due to error: {}", package_name, process_error));
        return;
    };

    emit_log(format!("Error: {}. Engaging Wireless Rescue...", process_error));
    if driver::connect_wireless(rescue_ip_address).is_err() {
        return;
    }

    *current_serial = rescue_ip_address.clone();
    if process_single_region_adb(emit_log, current_serial, &package_name, &target_directory, options).is_ok() {
        emit_log("Rescue Successful!".to_string());
        pulled.directories.push(target_directory);
    }
}

fn process_single_region_adb(emit_log: &(dyn Fn(String) + Sync), serial_number: &str, package_name: &str, output_directory: &Path, options: PullOptions) -> Result<(), String> {
    if options.import_mode == AdbImportType::All {
        pull_game_data_files(serial_number, package_name, output_directory)?;
    }

    let package_manager_output = driver::run_command(&["-s", serial_number, "shell", "pm", "path", package_name]).unwrap_or_default();

    if pull_target_apk(serial_number, &package_manager_output, "split_InstallPack.apk", output_directory).is_ok() {
        return Ok(());
    }

    if !package_manager_output.contains(BASE_APK) {
        emit_log("Warning: Update APK missing.".to_string());
        return Ok(());
    }

    if !options.ignore_modified_app {
        return Err(MODIFIED_APP.to_string());
    }

    emit_log(MODIFIED_APP.to_string());
    emit_log("Importing the base app instead.".to_string());
    pull_target_apk(serial_number, &package_manager_output, BASE_APK, output_directory)
}

fn pull_game_data_files(serial_number: &str, package_name: &str, output_directory: &Path) -> Result<(), String> {
    let user_identity = driver::run_command(&["-s", serial_number, "shell", "whoami"]).unwrap_or_default();
    let remote_source_path = format!("/data/data/{}/files", package_name);

    let output_directory_string = output_directory.to_str().ok_or("Invalid path on host machine.")?;

    if !output_directory.exists() {
        let _ = std::fs::create_dir_all(output_directory);
    }

    if user_identity.contains("root") && driver::run_command(&["-s", serial_number, "pull", &remote_source_path, output_directory_string]).is_ok() {
        let total_pulled = std::fs::read_dir(output_directory.join("files")).map(|i| i.count()).unwrap_or(0);
        if total_pulled > 0 {
            return Ok(());
        }
    }

    let remote_staging_directory = "/data/local/tmp";
    let remote_staging_target = "/data/local/tmp/files";

    let _ = driver::run_command(&["-s", serial_number, "shell", &format!("rm -rf {}", remote_staging_target)]);
    let _ = driver::run_command(&["-s", serial_number, "shell", &format!("mkdir -p {}", remote_staging_directory)]);

    let cmd_su_cp = format!("su -c 'cp -r {} {}'", remote_source_path, remote_staging_directory);
    let cmd_su0_cp = format!("su 0 cp -r {} {}", remote_source_path, remote_staging_directory);

    let copy_successful = driver::run_command(&["-s", serial_number, "shell", &cmd_su_cp]).is_ok()
        || driver::run_command(&["-s", serial_number, "shell", &cmd_su0_cp]).is_ok();

    if !copy_successful {
        return Err("Root Copy Failed. Device might not be rooted.".to_string());
    }

    let chmod_cmd = format!("su -c 'chmod -R 777 {}'", remote_staging_target);
    let _ = driver::run_command(&["-s", serial_number, "shell", &chmod_cmd]);

    let find_cmd = format!("su -c 'find {} -name \"*:*\" -delete'", remote_staging_target);
    let _ = driver::run_command(&["-s", serial_number, "shell", &find_cmd]);

    let pull_result = driver::run_command(&["-s", serial_number, "pull", remote_staging_target, output_directory_string]);

    let rm_cmd = format!("su -c 'rm -rf {}'", remote_staging_target);
    let _ = driver::run_command(&["-s", serial_number, "shell", &rm_cmd]);

    if pull_result.is_err() {
        return Err("ADB Pull Failed.".to_string());
    }

    let total_pulled_files = std::fs::read_dir(output_directory.join("files")).map(|iterator| iterator.count()).unwrap_or(0);

    if total_pulled_files == 0 {
        return Err("Pull verification failed: empty directory.".to_string());
    }

    Ok(())
}

fn pull_target_apk(serial_number: &str, package_manager_output: &str, target_filename: &str, output_directory: &Path) -> Result<(), String> {
    let remote_apk_path = package_manager_output.lines()
        .find(|line| line.contains(target_filename))
        .map(|line| line.trim().strip_prefix("package:").unwrap_or("").to_string())
        .or_else(|| {
            package_manager_output.lines()
                .find(|line| line.contains("base.apk"))
                .map(|line| line.trim().strip_prefix("package:").unwrap_or("").replace("base.apk", target_filename))
        })
        .ok_or("APK Path not found on device.")?;

    let local_destination_path = output_directory.join(target_filename);

    if !output_directory.exists() {
        let _ = std::fs::create_dir_all(output_directory);
    }

    let local_destination_string = local_destination_path.to_str().ok_or("Invalid path on host machine.")?;

    let _ = driver::run_command(&["-s", serial_number, "pull", &remote_apk_path, local_destination_string])?;

    let downloaded_apk_size = local_destination_path.metadata().map(|metadata| metadata.len()).unwrap_or(0);

    if !local_destination_path.exists() || downloaded_apk_size == 0 {
        let _ = std::fs::remove_file(&local_destination_path);
        return Err("APK verification failed after pull.".to_string());
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::field;

    // dumpsys prints the version among other space-separated pairs on the same line.
    const DUMP: &str = "  Package [jp.co.ponos.battlecatsen] (a1b2):\n    versionCode=1505000 minSdk=24 targetSdk=33\n    versionName=15.5.0\n    dataDir=/data/user/0/jp.co.ponos.battlecatsen\n";

    #[test]
    fn a_dump_yields_the_version_pair_without_its_neighbours() {
        assert_eq!(field(DUMP, "versionName="), Some("15.5.0".to_string()));
        assert_eq!(field(DUMP, "versionCode="), Some("1505000".to_string()));
        assert_eq!(field(DUMP, "splitName="), None);
    }
}
