use std::process::Command;
use std::path::Path;
use std::fs;
use crate::features::addons::apktool::download::{get_jar_path, get_apktool_dir, get_java_path};

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

pub fn decode(apk_path: &Path, out_dir: &Path, log_callback: &impl Fn(String)) -> Result<(), String> {
    let apktool_jar = get_jar_path().ok_or("apktool.jar is not installed.")?;

    let safe_temp_dir = get_apktool_dir().join("tmp");
    let _ = fs::create_dir_all(&safe_temp_dir);

    let arguments = vec![
        format!("-Djava.io.tmpdir={}", safe_temp_dir.display()),
        "-jar".to_string(),
        apktool_jar.to_string_lossy().to_string(),
        "d".to_string(),
        apk_path.to_string_lossy().to_string(),
        "-o".to_string(),
        out_dir.to_string_lossy().to_string(),
        "-f".to_string(),
    ];

    run_java_with_fallback(&arguments, log_callback)?;
    Ok(())
}

pub fn build(decode_dir: &Path, out_apk: &Path, log_callback: &impl Fn(String)) -> Result<(), String> {
    let apktool_jar = get_jar_path().ok_or("apktool.jar is not installed.")?;

    let safe_temp_dir = get_apktool_dir().join("tmp");
    let _ = fs::create_dir_all(&safe_temp_dir);

    let aapt2_name = if cfg!(target_os = "windows") { "aapt2.exe" } else { "aapt2" };
    let local_aapt2 = get_apktool_dir().join("bin").join(aapt2_name);

    let mut arguments = vec![
        format!("-Djava.io.tmpdir={}", safe_temp_dir.display()),
        "-jar".to_string(),
        apktool_jar.to_string_lossy().to_string(),
        "b".to_string(),
        decode_dir.to_string_lossy().to_string(),
        "-o".to_string(),
        out_apk.to_string_lossy().to_string(),
    ];

    if local_aapt2.exists() {
        arguments.push("--aapt".to_string());
        arguments.push(local_aapt2.to_string_lossy().to_string());
    }

    run_java_with_fallback(&arguments, log_callback)?;
    Ok(())
}