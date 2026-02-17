use std::process::Command;
use super::download;

pub fn get_adb_command() -> Result<std::path::PathBuf, String> {
    if let Some(path) = download::get_adb_path() {
        Ok(path)
    } else {
        Err("ADB not found. Please download it in Settings > Add-Ons.".to_string())
    }
}

pub fn run_command(args: &[&str]) -> Result<String, String> {
    let adb_path = get_adb_command()?;
    let mut cmd = Command::new(adb_path);
    cmd.args(args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn connect_to_device() -> Result<String, String> {
    let ports = [7555, 5555, 62001, 21503, 16384]; 
    
    // 1. Priority: Check if ANY device is already connected and ready.
    // This covers USB phones and emulators that are already running.
    if let Ok(devices_out) = run_command(&["devices"]) {
        for line in devices_out.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            if let Some((_, status)) = line.split_once('\t') {
                // If we see 'device', we are good. No need to mess with IPs.
                if status == "device" {
                    return Ok("Device already connected".to_string());
                }
            }
        }
    }

    // 2. Fallback: No active device found. Try connecting to known emulator ports.
    // This handles BlueStacks/Nox/etc if they haven't auto-registered.
    for port in ports {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(out) = run_command(&["connect", &addr]) {
            if out.contains("connected") {
                return Ok(format!("Connected to {}", addr));
            }
        }
    }

    Err("No device found. Please connect a phone via USB or launch an emulator.".to_string())
}