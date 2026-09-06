use std::env;
use std::fs::File;
use std::path::Path;

use tracing::{debug, error, info, warn};
use zip::ZipArchive;

use crate::systems::addons::apkeditor::{get_apkeditor_path, get_java_path};
use crate::common::process;

fn execute_command(
    binary_path: &Path,
    arguments: &[String],
    env_vars: Option<(&str, String)>
) -> Result<(), String> {
    debug!("Preparing to execute command: {:?}", binary_path);

    let mut command = process::command(binary_path);
    command.args(arguments);

    if let Some((key, value)) = env_vars {
        debug!("Setting environment variable: {}={}", key, value);
        command.env(key, value);
    }

    let output = command
        .output()
        .map_err(|error| {
            error!("Failed to start process {:?}: {}", binary_path, error);
            format!("Failed to start process: {}", error)
        })?;

    if output.status.success() {
        debug!("Command executed successfully: {:?}", binary_path);
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    error!("Command failed with Exit Code {}:\nOut: {}\nErr: {}", exit_code, stdout.trim(), stderr.trim());
    Err(format!("Exit Code {}:\nOut: {}\nErr: {}", exit_code, stdout.trim(), stderr.trim()))
}

fn run_java_with_fallback(arguments: &[String], log_callback: &impl Fn(String)) -> Result<(), String> {
    let local_java_path = get_java_path();

    if let Some(java_binary) = local_java_path {
        info!("Attempting to run local JRE at {:?}", java_binary);

        let java_bin_dir = java_binary.parent().unwrap_or_else(|| Path::new(""));
        let current_path = env::var("PATH").unwrap_or_default();
        let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
        let new_path = format!("{}{}{}", java_bin_dir.display(), separator, current_path);

        if execute_command(&java_binary, arguments, Some(("PATH", new_path))).is_ok() {
            return Ok(());
        }

        warn!("Local JRE crashed or incompatible. Falling back to system JRE.");
        log_callback("JRE crashed or incompatible\nFalling back to system JRE...".to_string());
    } else {
        info!("Local JRE not found. Falling back to system JRE.");
    }

    let system_java = Path::new("java");
    if let Err(system_error) = execute_command(system_java, arguments, None) {
        error!("System Java execution also failed: {}", system_error);
        return Err(format!("System Java execution also failed.\nError: {}", system_error));
    }

    Ok(())
}

fn validate_split_apk_archive(input_xapk: &Path) -> Result<(), String> {
    let file = File::open(input_xapk)
        .map_err(|error| format!("Failed to open {}: {}", input_xapk.display(), error))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|_| format!("{} is not a valid xapk/apkm/apks archive.", input_xapk.display()))?;

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|error| error.to_string())?;
        if entry.name().to_ascii_lowercase().ends_with(".apk") {
            return Ok(());
        }
    }

    Err(format!(
        "{} does not contain any split APKs.\nThe download may be incomplete or corrupted.",
        input_xapk.display()
    ))
}

pub(crate) fn merge_xapk(input_xapk: &Path, output_apk: &Path, log_callback: &impl Fn(String)) -> Result<(), String> {
    info!("Merging XAPK: {:?} -> {:?}", input_xapk, output_apk);

    validate_split_apk_archive(input_xapk)?;

    let editor_jar = get_apkeditor_path().ok_or("APKEditor.jar is not installed.")?;

    let arguments = vec![
        "-jar".to_string(),
        editor_jar.to_string_lossy().to_string(),
        "m".to_string(),
        "-i".to_string(),
        input_xapk.to_string_lossy().to_string(),
        "-o".to_string(),
        output_apk.to_string_lossy().to_string(),
    ];

    run_java_with_fallback(&arguments, log_callback)?;

    if !output_apk.exists() {
        error!("APKEditor finished but output APK is missing at {:?}", output_apk);
        return Err("APKEditor executed successfully, but the output APK is missing.".to_string());
    }

    info!("Successfully merged XAPK to {:?}", output_apk);
    Ok(())
}