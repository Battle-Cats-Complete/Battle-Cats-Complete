use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use tracing::{debug, error, info};
use zip::ZipArchive;

use super::paths::get_tools_dir;
use super::AddonStatus;

use crate::modules::addons::DownloadConfig;

const RELEASE_TAG: &str = "tools";
const REPO_OWNER: &str = "omochikaeri15";
const REPO_NAME: &str = "battle-cats-complete";

pub fn start_download(config: DownloadConfig) -> Receiver<AddonStatus> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        info!("Spawning download thread for {}", config.asset_name);
        if let Err(err) = download_thread(tx.clone(), config) {
            error!("Download failed: {}", err);
            let _ = tx.send(AddonStatus::Error(err));
        }
    });

    rx
}

fn download_thread(tx: Sender<AddonStatus>, config: DownloadConfig) -> Result<(), String> {
    let url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        REPO_OWNER, REPO_NAME, RELEASE_TAG, config.asset_name
    );

    debug!("Target URL: {}", url);
    let _ = tx.send(AddonStatus::Downloading(0.1, "Connecting...".to_string()));

    let client = reqwest::blocking::Client::builder()
        .user_agent("BattleCatsComplete/0.8.0")
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let response = client.get(&url)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: Status {}", response.status()));
    }

    let _ = tx.send(AddonStatus::Downloading(0.3, "Downloading...".to_string()));
    let bytes = response.bytes().map_err(|e| format!("Read error: {}", e))?;

    let _ = tx.send(AddonStatus::Downloading(0.7, "Extracting...".to_string()));
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Zip error: {}", e))?;

    let dest_dir = get_tools_dir().join(&config.folder_name);

    if !dest_dir.exists() {
        debug!("Creating destination directory at {:?}", dest_dir);
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    }

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;

        let Some(name) = file.enclosed_name() else { continue; };
        let out_path = dest_dir.join(name);

        if file.is_dir() {
            let _ = fs::create_dir_all(&out_path);
            continue;
        }

        if let Some(parent_dir) = out_path.parent() {
            let _ = fs::create_dir_all(parent_dir);
        }

        let mut outfile = fs::File::create(&out_path).map_err(|e| format!("File creation error: {}", e))?;
        std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Write error: {}", e))?;

        set_executable_permissions(&out_path, &config.binary_name);
    }

    info!("Successfully installed addon: {}", config.folder_name);
    let _ = tx.send(AddonStatus::Installed);
    Ok(())
}

#[cfg(unix)]
fn set_executable_permissions(out_path: &Path, binary_name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let Some(file_name) = out_path.file_name() else { return; };
    let is_target = file_name == binary_name;
    let in_bin_folder = out_path.parent().is_some_and(|p| p.ends_with("bin"));

    if is_target || in_bin_folder {
        debug!("Setting executable permissions for {:?}", out_path);
        let _ = fs::set_permissions(out_path, fs::Permissions::from_mode(0o755));
    }
}

#[cfg(not(unix))]
fn set_executable_permissions(_out_path: &Path, _binary_name: &str) {}