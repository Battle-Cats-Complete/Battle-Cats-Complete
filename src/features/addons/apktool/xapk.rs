use std::process::Command;
use std::path::Path;
use crate::features::addons::apktool::download::{get_apkeditor_path, get_java_path};

fn execute_command(binary_path: &Path, arguments: &[String]) -> Result<(), String> {
    let output = Command::new(binary_path)
        .args(arguments)
        .output()
        .map_err(|error| format!("Failed to start process: {}", error))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    Err(format!("Exit Code {}:\nOut: {}\nErr: {}", exit_code, stdout.trim(), stderr.trim()))
}

fn run_java_with_fallback(arguments: &[String], log_callback: &impl Fn(String)) -> Result<(), String> {
    let local_java_path = get_java_path();

    if let Some(java_binary) = local_java_path {
        if execute_command(&java_binary, arguments).is_ok() {
            return Ok(());
        }
        log_callback("JRE crashed or incompatible\nFalling back to system JRE...".to_string());
    }

    let system_java = Path::new("java");
    if let Err(system_error) = execute_command(system_java, arguments) {
        return Err(format!("System Java execution also failed.\nError: {}", system_error));
    }

    Ok(())
}

pub fn merge_xapk(input_xapk: &Path, output_apk: &Path, log_callback: &impl Fn(String)) -> Result<(), String> {
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
        return Err("APKEditor executed successfully, but the output APK is missing.".to_string());
    }

    Ok(())
}