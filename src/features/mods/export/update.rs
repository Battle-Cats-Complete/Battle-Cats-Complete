use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use std::io::{Read, Write};
use zip::{ZipArchive, ZipWriter};

use crate::features::mods::logic::state::ModState;
use crate::global::region::Region;
use crate::features::data::utilities::keys;
use crate::features::settings::logic::state::Settings;
use crate::features::addons::apktool::xapk;
use crate::features::mods::export::pack;
use crate::features::mods::export::sign;
use crate::features::mods::export::patch::{EVENT_RECEIVER, ExportEvent, spawn_log_adapter};

pub fn start_fast_track_export(state: &mut ModState) {
    if state.export.is_busy { return; }

    state.export.log_content.clear();
    state.export.is_busy = true;

    let Some(mod_folder) = state.selected_mod.clone() else { state.export.is_busy = false; return; };
    let Some(input_apk_path) = state.export.selected_apk.clone() else { state.export.is_busy = false; return; };
    let target_region = state.export.target_region.clone();

    let suffix = state.export.package_suffix.clone();
    let final_name = if suffix.is_empty() { "battlecats".to_string() } else { format!("battlecats{}", suffix) };

    let (tx, rx) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() { *guard = Some(rx); }

    thread::spawn(move || {
        let str_tx = spawn_log_adapter(tx.clone());
        let log_cb = |msg: String| { let _ = tx.send(ExportEvent::Log(msg)); };

        let settings: Settings = crate::global::io::json::load("settings.json").unwrap_or_default();
        let user_keys = match keys::verify(settings.game_data.enforce_key_validation, &str_tx) {
            Ok(k) => k,
            Err(e) => { let _ = tx.send(ExportEvent::Error(e)); return; }
        };

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("../../../.."));
        let mod_dir = base_dir.join("mods").join(&mod_folder);
        let export_dir = base_dir.join("exports");

        let app_dir = mod_dir.join("app");
        let patch_dir = mod_dir.join("patch");
        let loose_dir = mod_dir.join("loose");
        let icons_dir = mod_dir.join("icons");

        let _ = fs::remove_dir_all(&app_dir);
        let _ = fs::create_dir_all(&export_dir);
        let _ = fs::create_dir_all(&app_dir);

        let temp_assets_dir = app_dir.join("assets");
        let _ = fs::create_dir_all(&temp_assets_dir);

        let is_xapk = input_apk_path.extension().and_then(|e| e.to_str()) == Some("xapk");
        let mut working_apk = input_apk_path.clone();

        if is_xapk {
            log_cb("Merging XAPK into APK...".to_string());
            let xapk_dir = app_dir.join("xapk");
            let _ = fs::create_dir_all(&xapk_dir);

            let merged_temp_path = xapk_dir.join("merged_xapk.apk");
            if let Err(e) = xapk::merge_xapk(&working_apk, &merged_temp_path, &log_cb) {
                let _ = tx.send(ExportEvent::Error(e)); return;
            }
            working_apk = merged_temp_path;
        }

        let region_key = match target_region {
            Region::En => &user_keys.en,
            Region::Ja => &user_keys.ja,
            Region::Ko => &user_keys.ko,
            Region::Tw => &user_keys.tw,
        };

        if let Err(e) = pack::stream_pack_and_list(&patch_dir, &temp_assets_dir, "DownloadLocal", region_key, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        let mut inject_map: HashMap<String, PathBuf> = HashMap::new();
        inject_map.insert("assets/DownloadLocal.pack".to_string(), temp_assets_dir.join("DownloadLocal.pack"));
        inject_map.insert("assets/DownloadLocal.list".to_string(), temp_assets_dir.join("DownloadLocal.list"));

        let mut loose_count = 0;
        if loose_dir.exists() {
            if let Ok(entries) = fs::read_dir(&loose_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        inject_map.insert(format!("assets/{}", filename), entry.path());
                        loose_count += 1;
                    }
                }
            }
        }

        if loose_count > 0 {
            log_cb(format!("Injected {} loose files.", loose_count));
        }

        let icon_png = icons_dir.join("icon.png");
        let icon_fg = icons_dir.join("icon_foreground.png");
        let push_icon = icons_dir.join("push_icon.png");

        let source_file = match File::open(&working_apk) {
            Ok(f) => f,
            Err(e) => { let _ = tx.send(ExportEvent::Error(format!("Failed to open source APK: {}", e))); return; }
        };

        let mut archive = match ZipArchive::new(source_file) {
            Ok(a) => a,
            Err(e) => { let _ = tx.send(ExportEvent::Error(format!("Invalid Source APK format: {}", e))); return; }
        };

        let mut compression_catalog: HashMap<String, zip::CompressionMethod> = HashMap::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                compression_catalog.insert(file.name().to_string(), file.compression());
            }
        }

        let unsigned_apk_path = app_dir.join("unsigned_fast.apk");
        let dest_file = match File::create(&unsigned_apk_path) {
            Ok(f) => f,
            Err(e) => { let _ = tx.send(ExportEvent::Error(format!("Failed to create temp APK: {}", e))); return; }
        };
        let mut zip_writer = ZipWriter::new(dest_file);

        log_cb("Injecting and aligning files...".to_string());
        let mut injected_count = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();

            if name.starts_with("META-INF/") {
                continue;
            }

            let mut replacement_path = inject_map.get(&name).cloned();

            if replacement_path.is_none() && name.starts_with("res/") {
                let filename = Path::new(&name).file_name().unwrap_or_default().to_string_lossy();
                if filename == "icon.png" && icon_png.exists() {
                    replacement_path = Some(icon_png.clone());
                } else if filename == "icon_foreground.png" && icon_fg.exists() {
                    replacement_path = Some(icon_fg.clone());
                } else if filename == "push_icon.png" && push_icon.exists() {
                    replacement_path = Some(push_icon.clone());
                }
            }

            if let Some(local_path) = replacement_path {
                let data = fs::read(&local_path).unwrap_or_default();
                let comp_method = compression_catalog.get(&name).copied().unwrap_or(zip::CompressionMethod::Deflated);

                let mut options = zip::write::SimpleFileOptions::default().compression_method(comp_method);
                if comp_method == zip::CompressionMethod::Stored {
                    let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("");
                    let alignment = if ext == "so" { 4096 } else { 4 };
                    options = options.with_alignment(alignment);
                }

                if zip_writer.start_file(&name, options).is_ok() {
                    let _ = zip_writer.write_all(&data);
                    injected_count += 1;
                }
                inject_map.remove(&name);
            } else {
                if file.compression() == zip::CompressionMethod::Stored {
                    let mut data = Vec::new();
                    if file.read_to_end(&mut data).is_ok() {
                        let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("");
                        let alignment = if ext == "so" { 4096 } else { 4 };
                        let options = zip::write::SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Stored)
                            .with_alignment(alignment);

                        if zip_writer.start_file(&name, options).is_ok() {
                            let _ = zip_writer.write_all(&data);
                        }
                    }
                } else {
                    if let Err(e) = zip_writer.raw_copy_file(file) {
                        let _ = tx.send(ExportEvent::Error(format!("Failed to copy internal zip chunk {}: {}", name, e))); return;
                    }
                }
            }
        }
        
        for (zip_path, local_path) in inject_map {
            let data = fs::read(&local_path).unwrap_or_default();
            let ext = Path::new(&zip_path).extension().and_then(|e| e.to_str()).unwrap_or("");

            let is_stored = matches!(ext, "pack" | "list" | "dex" | "arsc" | "so" | "ogg");
            let compression = if is_stored { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };

            let mut options = zip::write::SimpleFileOptions::default().compression_method(compression);
            if is_stored {
                let alignment = if ext == "so" { 4096 } else { 4 };
                options = options.with_alignment(alignment);
            }

            if zip_writer.start_file(&zip_path, options).is_ok() {
                let _ = zip_writer.write_all(&data);
                injected_count += 1;
            }
        }

        if let Err(e) = zip_writer.finish() {
            let _ = tx.send(ExportEvent::Error(format!("Failed to finalize APK Zip: {}", e))); return;
        }

        log_cb(format!("Successfully patched {} files.", injected_count));
        log_cb("Signing APK...".to_string());

        if let Err(e) = sign::sign(&unsigned_apk_path, None) {
            let _ = tx.send(ExportEvent::Error(format!("Native Signing Error: {}", e))); return;
        }

        let final_apk_path = export_dir.join(format!("{}.apk", final_name));

        if let Err(e) = fs::copy(&unsigned_apk_path, &final_apk_path) {
            let _ = tx.send(ExportEvent::Error(format!("Filesystem Error: {}", e))); return;
        }

        let _ = fs::remove_dir_all(&app_dir);
        let _ = tx.send(ExportEvent::Success(format!("Successfully Updated {}.apk!", final_name)));
    });
}