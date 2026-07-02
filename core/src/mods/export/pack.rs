use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use nyanko::pack::cryptology;
use tracing::{debug, error, info, trace, warn};

use crate::data::utilities::keys;
use crate::global::region::Region;
use crate::mods::export::patch::{spawn_log_adapter, ExportEvent, EVENT_RECEIVER};
use crate::mods::logic::state::ModDataState;
use crate::settings::logic::keys::RegionKey;
use crate::settings::logic::state::Settings;

pub fn start_pack_export(state: &mut ModDataState) {
    if state.export.is_busy {
        warn!("Pack export requested, but an export is already busy. Ignoring.");
        return;
    }

    let Some(mod_folder) = state.selected_mod.clone() else {
        error!("No mod selected for pack export.");
        return;
    };

    info!("Initializing Pack Export for mod: {}", mod_folder);
    state.export.log_content.clear();
    state.export.is_busy = true;
    state.export.status_message = "Initializing Pack Export...".to_string();

    let pack_name = if state.export.pack_name.trim().is_empty() {
        "DownloadLocal".to_string()
    } else {
        state.export.pack_name.clone()
    };
    let target_region = state.export.target_region;

    let (transmitter, receiver) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() { *guard = Some(receiver); }

    thread::spawn(move || {
        let string_transmitter = spawn_log_adapter(transmitter.clone());
        let log_callback = |message: String| { let _ = transmitter.send(ExportEvent::Log(message)); };

        trace!("Loading settings for pack export...");
        let settings: Settings = crate::global::io::json::load("settings.json").unwrap_or_default();

        debug!("Verifying keys...");
        let user_keys = match keys::verify(settings.game_data.enforce_key_validation, &string_transmitter) {
            Ok(keys) => {
                trace!("Keys successfully verified.");
                keys
            },
            Err(error) => {
                error!("Key verification failed: {}", error);
                let _ = transmitter.send(ExportEvent::Error(error));
                return;
            }
        };

        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("../../../../../../../../.."));
        let mod_path = base_dir.join("mods").join(&mod_folder);
        let export_dir = base_dir.join("exports");

        let patch_dir = mod_path.join("patch");
        let _ = fs::create_dir_all(&export_dir);

        let region_key = match target_region {
            Region::En => &user_keys.en,
            Region::Ja => &user_keys.ja,
            Region::Ko => &user_keys.ko,
            Region::Tw => &user_keys.tw,
        };

        debug!("Starting stream pack and list build targeting region: {:?}", target_region);
        if let Err(error) = stream_pack_and_list(&patch_dir, &export_dir, &pack_name, region_key, &log_callback) {
            error!("Pack and list streaming failed: {}", error);
            let _ = transmitter.send(ExportEvent::Error(error)); return;
        }

        info!("Pack Export successfully finished: {}.pack", pack_name);
        let _ = transmitter.send(ExportEvent::Success(format!("\nSuccessfully Created {}.pack!", pack_name)));
    });
}

pub fn stream_pack_and_list(
    source_dir: &Path,
    dest_dir: &Path,
    pack_name: &str,
    region_key: &RegionKey,
    log_callback: &impl Fn(String)
) -> Result<(), String> {

    debug!("Scanning source directory for pack: {:?}", source_dir);
    let mut files_with_size = Vec::new();

    if let Ok(entries) = fs::read_dir(source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Ok(metadata) = fs::metadata(&path) {
                files_with_size.push((path, metadata.len() as usize));
            }
        }
    }

    let total_files = files_with_size.len();
    if total_files == 0 {
        warn!("No files found in the patch directory.");
        return Err("No files found in the patch directory.".to_string());
    }

    let pack_name_lower = pack_name.to_lowercase();
    let pack_type = if pack_name_lower.contains("imagedatalocal") {
        cryptology::PackType::ImageData
    } else if pack_name_lower.contains("server") {
        cryptology::PackType::Server
    } else {
        cryptology::PackType::Standard
    };

    trace!("Determined pack type: {:?}", pack_type);
    info!("Found {} files to patch.", total_files);
    log_callback(format!("Found {} files to patch.", total_files));

    let standard_keys = if pack_type == cryptology::PackType::Standard {
        trace!("Decoding hex standard keys...");
        let key_bytes = hex::decode(&region_key.key).map_err(|_| "Invalid Region Key Hex".to_string())?;
        let iv_bytes = hex::decode(&region_key.iv).map_err(|_| "Invalid Region IV Hex".to_string())?;

        if key_bytes.len() != 16 || iv_bytes.len() != 16 {
            error!("Region Key/IV length is incorrect: key {}, iv {}", key_bytes.len(), iv_bytes.len());
            return Err("Region Key/IV length is incorrect. Check settings.".to_string());
        }

        let key_array: [u8; 16] = key_bytes.try_into().map_err(|_| "Failed to map key array")?;
        let iv_array: [u8; 16] = iv_bytes.try_into().map_err(|_| "Failed to map IV array")?;

        Some((key_array, iv_array))
    } else {
        None
    };

    let log_interval = (total_files / 10).max(1);

    let pack_path = dest_dir.join(format!("{}.pack", pack_name));
    let list_path = dest_dir.join(format!("{}.list", pack_name));

    let pack_file = File::create(&pack_path).map_err(|error| format!("Failed to create pack stream file: {}", error))?;
    let mut pack_writer = BufWriter::new(pack_file);

    let mut list_string = format!("{}\n", total_files);
    let mut current_address = 0;

    debug!("Beginning stream write sequence...");
    for (index, (file_path, _file_size)) in files_with_size.iter().enumerate() {
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if index > 0 && index % log_interval == 0 {
            trace!("Streamed {} files so far.", index);
            log_callback(format!("Packed {} files | Streaming: {}", index, filename));
        }

        let mut file_data = fs::read(file_path).map_err(|error| format!("Failed to read {}: {}", filename, error))?;

        let (cipher_key, cipher_iv) = match &standard_keys {
            Some((key_array, iv_array)) => (Some(key_array), Some(iv_array)),
            None => (None, None),
        };

        file_data = cryptology::encrypt_chunk(&file_data, pack_type, cipher_key, cipher_iv)
            .map_err(|error| format!("Encryption failed for {}: {}", filename, error))?;

        pack_writer.write_all(&file_data).map_err(|error| format!("Failed to write to pack buffer: {}", error))?;

        let new_size = file_data.len();
        list_string.push_str(&format!("{},{},{}\n", filename, current_address, new_size));
        current_address += new_size;
    }

    debug!("Flushing pack writer to disk.");
    pack_writer.flush().map_err(|error| format!("Failed to flush pack stream to disk: {}", error))?;

    trace!("Encrypting and saving list metadata.");
    let list_bytes = cryptology::encrypt_list(&list_string)
        .map_err(|error| format!("Failed to encrypt list file: {}", error))?;

    fs::write(list_path, list_bytes).map_err(|error| format!("Failed to write list file: {}", error))?;

    info!("Pack stream and list generation complete.");
    Ok(())
}