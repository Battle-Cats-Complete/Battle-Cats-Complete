use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use crate::features::mods::logic::state::ModState;
use crate::global::region::Region;
use crate::features::data::utilities::keys;
use crate::features::settings::logic::state::Settings;
use crate::features::addons::apktool::{apk, xapk};
use crate::features::mods::export::{modify, sign, pack};
use crate::features::mods::export::patch::{EVENT_RECEIVER, ExportEvent, spawn_log_adapter};

pub fn start_apk_export(state: &mut ModState) {
    if state.export.is_busy { return; }

    state.export.log_content.clear();
    state.export.is_busy = true;

    let app_title = state.export.app_title.clone();
    let suffix = state.export.package_suffix.clone();
    let Some(mod_folder) = state.selected_mod.clone() else { state.export.is_busy = false; return; };
    let Some(input_apk_path) = state.export.selected_apk.clone() else { state.export.is_busy = false; return; };
    let detected_region = state.export.target_region.clone();

    let (tx, rx) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() { *guard = Some(rx); }

    thread::spawn(move || {
        let str_tx = spawn_log_adapter(tx.clone());
        let log_cb = |msg: String| { let _ = tx.send(ExportEvent::Log(msg)); };

        let settings: Settings = crate::global::io::json::load("settings.json").unwrap_or_default();
        let user_keys = match keys::verify(settings.game_data.enforce_key_validation, &str_tx) {
            Ok(k) => k,
            Err(e) => {
                let _ = tx.send(ExportEvent::Error(e));
                return;
            }
        };

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("../../../.."));
        let mod_dir = base_dir.join("mods").join(&mod_folder);
        let export_dir = base_dir.join("exports");

        let app_dir = mod_dir.join("app");
        let xapk_dir = app_dir.join("xapk");
        let apk_dir = app_dir.join("apk");

        let _ = fs::remove_dir_all(&app_dir);
        let _ = fs::create_dir_all(&export_dir);
        let _ = fs::create_dir_all(&app_dir);

        let is_xapk = input_apk_path.extension().and_then(|e| e.to_str()) == Some("xapk");
        let mut working_apk = input_apk_path.clone();

        if is_xapk {
            log_cb("Merging XAPK into APK...".to_string());
            let _ = fs::create_dir_all(&xapk_dir);
            let merged_temp_path = xapk_dir.join("merged_xapk.apk");

            if let Err(e) = xapk::merge_xapk(&working_apk, &merged_temp_path, &log_cb) {
                let _ = tx.send(ExportEvent::Error(e)); return;
            }
            working_apk = merged_temp_path;
        }

        log_cb("Decoding APK...".to_string());
        if let Err(e) = apk::decode(&working_apk, &apk_dir, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        log_cb("Applying package...".to_string());
        let final_id_result = modify::patch_identity(&apk_dir, &suffix, &app_title, &log_cb);
        if let Err(e) = final_id_result {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }
        let final_id = final_id_result.unwrap_or_else(|_| "jp.co.ponos.battlecats".to_string());

        log_cb("Replacing icons...".to_string());
        let _ = modify::replace_icons(&mod_dir, &apk_dir, &log_cb);

        let region_key = match detected_region {
            Region::En => &user_keys.en,
            Region::Ja => &user_keys.ja,
            Region::Ko => &user_keys.ko,
            Region::Tw => &user_keys.tw,
        };

        let patch_dir = mod_dir.join("patch");
        let assets_dir = apk_dir.join("assets");
        let _ = fs::create_dir_all(&assets_dir);

        if let Err(e) = pack::stream_pack_and_list(&patch_dir, &assets_dir, "DownloadLocal", region_key, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        let loose_count = modify::inject_loose_assets(&mod_dir, &apk_dir).unwrap_or(0);
        if loose_count > 0 {
            log_cb(format!("Injected {} loose files.", loose_count));
        }

        log_cb("Successfully patched data.".to_string());
        log_cb("Rebuilding APK...".to_string());

        let unsigned_apk_path = app_dir.join("unsigned_final.apk");
        if let Err(e) = apk::build(&apk_dir, &unsigned_apk_path, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        log_cb("Normalizing binaries...".to_string());
        let normalized_apk_path = app_dir.join("normalized_final.apk");
        if let Err(e) = modify::normalize_apk(&unsigned_apk_path, &normalized_apk_path) {
            let _ = tx.send(ExportEvent::Error(format!("Normalization Error: {}", e))); return;
        }

        log_cb("Signing APK...".to_string());
        if let Err(e) = sign::sign(&normalized_apk_path, None) {
            let _ = tx.send(ExportEvent::Error(format!("Native Signing Error: {}", e))); return;
        }

        let output_apk = export_dir.join(format!("{}.apk", final_id));
        if let Err(e) = fs::copy(&normalized_apk_path, &output_apk) {
            let _ = tx.send(ExportEvent::Error(format!("Filesystem Error: {}", e))); return;
        }

        let _ = fs::remove_dir_all(&app_dir);
        let _ = tx.send(ExportEvent::Success(format!("Successfully Built {}.apk!", final_id)));
    });
}