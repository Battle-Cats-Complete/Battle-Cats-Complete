use std::env;
use std::path::Path;
use std::process::Command;

use super::download::{get_apkeditor_path, get_java_path};

fn execute_command(
    binary_path: &Path,
    arguments: &[String],
    env_vars: Option<(&str, String)>
) -> Result<(), String> {
    let mut command = Command::new(binary_path);
    command.args(arguments);
    
    if let Some((key, value)) = env_vars {
        command.env(key, value);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
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
        let java_bin_dir = java_binary.parent().unwrap_or(Path::new(""));
        let current_path = env::var("PATH").unwrap_or_default();
        let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
        let new_path = format!("{}{}{}", java_bin_dir.display(), separator, current_path);
        
        if execute_command(&java_binary, arguments, Some(("PATH", new_path))).is_ok() {
            return Ok(());
        }
        log_callback("JRE crashed or incompatible\nFalling back to system JRE...".to_string());
    }

    let system_java = Path::new("java");
    if let Err(system_error) = execute_command(system_java, arguments, None) {
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