use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;

use tracing::{error, info, trace};
use zip::ZipArchive;

use crate::modules::settings::UserKeys;

use super::decrypt;

fn format_log_name(name: &str, path: &Path) -> String {
    let cleaned = name.trim_start_matches("...\\").trim_start_matches(".../");
    if cleaned.len() > 35 {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let trunc_stem: String = stem.chars().take(20).collect();
        if !ext.is_empty() && ext.len() <= 5 {
            format!("{}....{}", trunc_stem, ext)
        } else {
            format!("{}...", trunc_stem)
        }
    } else {
        cleaned.to_string()
    }
}

pub fn run_archive(archive_path: &Path, workspace_dir: &Path, tx: Sender<String>, user_keys: &UserKeys) -> Result<(), String> {
    let _ = tx.send("Opening archive...".to_string());
    info!("Opening archive for extraction: {:?}", archive_path);

    let file = fs::File::open(archive_path).map_err(|e| {
        error!("Failed to open archive: {}", e);
        e.to_string()
    })?;

    let mut archive = ZipArchive::new(file).map_err(|e| {
        error!("Failed to parse zip archive: {}", e);
        e.to_string()
    })?;

    let mut has_pack_data = false;

    for icon in ["icon.png", "icon_foreground.png", "push_icon.png"] {
        for dpi in ["xxxhdpi", "xxhdpi", "xhdpi"] {
            let zip_path = format!("res/drawable-{}/{}", dpi, icon);

            let Ok(mut file_in_zip) = archive.by_name(&zip_path) else { continue };

            if let Ok(mut out_file) = fs::File::create(workspace_dir.join("icons").join(icon)) {
                let _ = std::io::copy(&mut file_in_zip, &mut out_file);
                trace!("Extracted APK icon: {}", icon);
            }
            break;
        }
    }

    let total_entries = archive.len();
    let log_interval = (total_entries / 10).max(1);
    let mut extracted_count = 0;

    for i in 0..total_entries {
        let Ok(mut file_in_zip) = archive.by_index(i) else { continue };
        if file_in_zip.is_dir() { continue; }

        let name = file_in_zip.name().to_string();
        let path = Path::new(&name);
        let Some(safe_name) = path.file_name() else { continue };
        let safe_name_str = safe_name.to_string_lossy();

        let save_path = if name.starts_with("patch/") || name.starts_with("loose/") || name.starts_with("icons/") {
            Some(workspace_dir.join(name.clone()))
        } else if name == "assets/DownloadLocal.list" || name == "assets/DownloadLocal.pack" {
            has_pack_data = true;
            Some(workspace_dir.join(safe_name))
        } else if safe_name_str == "metadata.json" {
            Some(workspace_dir.join("patch").join(safe_name))
        } else if path.parent() == Some(Path::new("assets")) {
            let is_download_tsv = safe_name_str.starts_with("download") && safe_name_str.ends_with(".tsv");
            let is_junk = safe_name_str.ends_with(".dat")
                || safe_name_str.ends_with(".lock")
                || safe_name_str.ends_with(".pack")
                || safe_name_str.ends_with(".list")
                || safe_name_str.ends_with(".dex");

            if !is_download_tsv && !is_junk {
                Some(workspace_dir.join("loose").join(safe_name))
            } else { None }
        } else { None };

        if let Some(out_path) = save_path {
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut out_file) = fs::File::create(&out_path) {
                let _ = std::io::copy(&mut file_in_zip, &mut out_file);
                extracted_count += 1;

                if extracted_count % log_interval == 0 || i == total_entries - 1 {
                    let display_name = format_log_name(&safe_name_str, path);
                    trace!("Extracted file: {}", display_name);
                    let _ = tx.send(format!("Extracted {} Files | Streaming: {}", extracted_count, display_name));
                }
            }
        }
    }

    if has_pack_data && workspace_dir.join("DownloadLocal.list").exists() && workspace_dir.join("DownloadLocal.pack").exists() {
        let _ = tx.send("Decrypting embedded pack data...".to_string());
        info!("Found embedded pack data, delegating to decrypt module.");

        let pack_file = workspace_dir.join("DownloadLocal.pack");
        decrypt::run(&pack_file, workspace_dir, tx.clone(), user_keys)?;

        let _ = fs::remove_file(workspace_dir.join("DownloadLocal.list"));
        let _ = fs::remove_file(workspace_dir.join("DownloadLocal.pack"));
    }

    Ok(())
}