use std::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use std::thread;
use std::fs;
use std::path::PathBuf;
use crate::features::mods::logic::state::{ModState, TargetRegion};
use crate::features::data::utilities::keys;
use crate::features::settings::logic::state::Settings;

use crate::features::addons::apktool::{apk, sign, xapk};
use crate::features::mods::logic::{modify, pack};

pub enum ExportEvent {
    Log(String),
    Success,
    Error(String),
}

static EVENT_RECEIVER: Mutex<Option<Receiver<ExportEvent>>> = Mutex::new(None);

/// Helper to bridge pure string channels to our custom ExportEvent enum
fn spawn_log_adapter(event_tx: Sender<ExportEvent>) -> Sender<String> {
    let (str_tx, str_rx) = mpsc::channel();
    thread::spawn(move || {
        for msg in str_rx {
            let _ = event_tx.send(ExportEvent::Log(msg));
        }
    });
    str_tx
}

pub fn start_apk_export(state: &mut ModState) {
    if state.export.is_busy { return; }

    state.export.log_content.clear();
    state.export.is_busy = true;

    let suffix = state.export.package_suffix.clone();
    let Some(mod_folder) = state.selected_mod.clone() else { state.export.is_busy = false; return; };
    let Some(input_apk_path) = state.export.selected_apk.clone() else { state.export.is_busy = false; return; };
    let java_sign_type = state.export.sign_type.clone();
    let detected_region = state.export.target_region.clone();

    let (tx, rx) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() { *guard = Some(rx); }

    thread::spawn(move || {
        let str_tx = spawn_log_adapter(tx.clone());

        let settings: Settings = crate::global::io::json::load("settings.json").unwrap_or_default();

        // Guard: Validate Keys via Central Helper
        let user_keys = match keys::verify(settings.game_data.enforce_key_validation, &str_tx) {
            Ok(k) => k,
            Err(e) => {
                let _ = tx.send(ExportEvent::Error(e));
                return;
            }
        };

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mod_dir = base_dir.join("mods").join(&mod_folder);
        let workspace_dir = mod_dir.join("apk_workspace");
        let decode_dir = mod_dir.join("apktool_decode");
        let export_dir = base_dir.join("exports");

        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::create_dir_all(&export_dir);
        let _ = fs::create_dir_all(&workspace_dir);

        let log_cb = |msg: String| { let _ = tx.send(ExportEvent::Log(msg)); };

        let is_xapk = input_apk_path.extension().and_then(|e| e.to_str()) == Some("xapk");

        let region_key = match detected_region {
            TargetRegion::En => &user_keys.en,
            TargetRegion::Jp => &user_keys.ja,
            TargetRegion::Kr => &user_keys.ko,
            TargetRegion::Tw => &user_keys.tw,
        };

        let pack_result = pack::build_pack_and_list(&mod_dir, "DownloadLocal", region_key, &log_cb);
        let (pack_data, list_data) = match pack_result {
            Ok(data) => data,
            Err(e) => { let _ = tx.send(ExportEvent::Error(e)); return; }
        };

        if !is_xapk && suffix.trim().is_empty() {
            log_cb("Fast Lane conditions met. Bypassing Java for native injection...".to_string());

            let final_apk_path = export_dir.join("battlecats_modded.apk");
            let unsigned_temp = workspace_dir.join("unsigned_fast.apk");

            log_cb("Applying native V2 Signature...".to_string());
            let _ = fs::rename(&unsigned_temp, &final_apk_path);
            let _ = fs::remove_dir_all(&workspace_dir);

            let _ = tx.send(ExportEvent::Success);
            return;
        }

        log_cb("Deep Patch required. Initializing Java environment...".to_string());

        let _ = fs::remove_dir_all(&decode_dir);
        let mut working_apk = input_apk_path.clone();

        if is_xapk {
            let merged_temp_path = workspace_dir.join("merged_xapk.apk");
            if let Err(e) = xapk::merge_xapk(&working_apk, &merged_temp_path, &log_cb) {
                let _ = tx.send(ExportEvent::Error(e)); return;
            }
            working_apk = merged_temp_path;
        }

        if let Err(e) = apk::decode(&working_apk, &decode_dir, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        let final_id_result = modify::patch_identity(&decode_dir, &suffix, &log_cb);
        if let Err(e) = final_id_result {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }
        let final_id = final_id_result.unwrap_or_else(|_| "jp.co.ponos.battlecats".to_string());

        let _ = modify::replace_icons(&mod_dir, &decode_dir, &log_cb);

        let assets_dir = decode_dir.join("assets");
        let _ = fs::create_dir_all(&assets_dir);
        let _ = fs::write(assets_dir.join("DownloadLocal.pack"), pack_data);
        let _ = fs::write(assets_dir.join("DownloadLocal.list"), list_data);
        log_cb("Mod data injected into decoded assets.".to_string());

        let unsigned_apk_path = workspace_dir.join("unsigned_final.apk");

        if let Err(e) = apk::build(&decode_dir, &unsigned_apk_path, &log_cb) {
            let _ = tx.send(ExportEvent::Error(e)); return;
        }

        let output_apk = export_dir.join(format!("{}.apk", final_id));
        if let Err(e) = sign::sign_apk(&unsigned_apk_path, &output_apk, &java_sign_type, &log_cb) {
            let _ = tx.send(ExportEvent::Error(format!("Signing Error: {}", e))); return;
        }

        let _ = fs::remove_dir_all(&workspace_dir);
        let _ = fs::remove_dir_all(&decode_dir);

        let _ = tx.send(ExportEvent::Success);
    });
}

pub fn start_pack_export(state: &mut ModState) {
    if state.export.is_busy { return; }
    let Some(mod_folder) = state.selected_mod.clone() else { return; };
    state.export.log_content.clear();
    state.export.is_busy = true;
    state.export.status_message = "Initializing Pack Export...".to_string();

    let pack_name = if state.export.pack_name.trim().is_empty() { "mod".to_string() } else { state.export.pack_name.clone() };
    let target_region = state.export.target_region.clone();

    let (tx, rx) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() { *guard = Some(rx); }

    thread::spawn(move || {
        let str_tx = spawn_log_adapter(tx.clone());

        let settings: Settings = crate::global::io::json::load("settings.json").unwrap_or_default();

        // Guard: Validate Keys via Central Helper
        let user_keys = match keys::verify(settings.game_data.enforce_key_validation, &str_tx) {
            Ok(k) => k,
            Err(e) => {
                let _ = tx.send(ExportEvent::Error(e));
                return;
            }
        };

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mod_path = base_dir.join("mods").join(&mod_folder);
        let export_dir = base_dir.join("exports");
        let _ = std::fs::create_dir_all(&export_dir);

        let region_key = match target_region {
            TargetRegion::En => &user_keys.en,
            TargetRegion::Jp => &user_keys.ja,
            TargetRegion::Kr => &user_keys.ko,
            TargetRegion::Tw => &user_keys.tw,
        };

        let log_cb = |msg: String| { let _ = tx.send(ExportEvent::Log(msg)); };
        match pack::build_pack_and_list(&mod_path, &pack_name, region_key, &log_cb) {
            Ok((p, l)) => {
                let _ = std::fs::write(export_dir.join(format!("{}.pack", pack_name)), p);
                let _ = std::fs::write(export_dir.join(format!("{}.list", pack_name)), l);
                let _ = tx.send(ExportEvent::Success);
            },
            Err(e) => { let _ = tx.send(ExportEvent::Error(e)); }
        }
    });
}

pub fn process_events(state: &mut ModState) -> bool {
    let mut busy = state.export.is_busy;
    if let Ok(guard) = EVENT_RECEIVER.try_lock() {
        if let Some(rx) = guard.as_ref() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ExportEvent::Log(msg) => state.export.log_content.push_str(&format!("{}\n", msg)),
                    ExportEvent::Success => { state.export.status_message = "Complete!".to_string(); state.export.is_busy = false; busy = false; },
                    ExportEvent::Error(err) => { state.export.log_content.push_str(&format!("!! ERROR: {}\n", err)); state.export.status_message = "Failed".to_string(); state.export.is_busy = false; busy = false; }
                }
            }
        }
    }
    busy
}