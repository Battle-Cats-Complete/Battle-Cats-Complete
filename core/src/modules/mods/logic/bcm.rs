use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use tracing::{debug, error, info, info_span, trace, warn};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::modules::mods::logic::state::ModDataState;
use crate::modules::mods::export::patch::{spawn_log_adapter, ExportEvent, EVENT_RECEIVER};

pub const BCM_COMPRESSION_MIN: i64 = 0;
pub const BCM_COMPRESSION_MAX: i64 = 9;
pub const BCM_COMPRESSION_DEFAULT: i64 = 6;

pub fn start_bcm_export(state: &mut ModDataState, compression: i64) {
    if state.export.is_busy {
        warn!("BCM export requested, but an export is already busy. Ignoring.");
        return;
    }

    let Some(mod_folder) = state.selected_mod.clone() else {
        error!("No mod selected for BCM export.");
        return;
    };

    info!("Initializing BCM Export for mod: {}", mod_folder);
    state.export.log_content.clear();
    state.export.is_busy = true;

    let app_title = state.export.app_title.clone();
    let export_name = if app_title.trim().is_empty() {
        mod_folder.clone()
    } else {
        app_title.trim().to_string()
    };

    let (transmitter, receiver) = mpsc::channel();
    if let Ok(mut guard) = EVENT_RECEIVER.lock() {
        *guard = Some(receiver);
    }

    thread::spawn(move || {
        let _thread_span = info_span!("bcm_export_worker", mod_id = %mod_folder).entered();

        let _ = spawn_log_adapter(transmitter.clone());
        let log_callback = |message: String| {
            let _ = transmitter.send(ExportEvent::Log(message));
        };

        log_callback("Preparing files...".to_string());

        let current_dir_res = std::env::current_dir();
        if let Err(error) = &current_dir_res {
            error!("Failed to determine current directory: {}", error);
            let _ = transmitter.send(ExportEvent::Error("Filesystem anchor lost".to_string()));
            return;
        }
        let base_dir = current_dir_res.unwrap_or_else(|_| PathBuf::from("../../../../.."));

        let mod_dir = base_dir.join("mods").join(&mod_folder);
        let export_dir = base_dir.join("exports");

        if !mod_dir.exists() {
            error!("Target mod directory does not exist: {}", mod_dir.display());
            let _ = transmitter.send(ExportEvent::Error("Mod directory missing".to_string()));
            return;
        }

        if let Err(error) = fs::create_dir_all(&export_dir) {
            error!("Failed to ensure exports directory exists: {}", error);
            let _ = transmitter.send(ExportEvent::Error(format!("Export dir error: {}", error)));
            return;
        }

        let mut bcm_filename = format!("{}.bcm", export_name);
        let mut bcm_path = export_dir.join(&bcm_filename);
        let mut counter = 1;

        while bcm_path.exists() {
            bcm_filename = format!("{}{}.bcm", export_name, counter);
            bcm_path = export_dir.join(&bcm_filename);
            counter += 1;
        }

        debug!("Target BCM package path: {}", bcm_path.display());
        log_callback("Packaging mod...".to_string());

        match build_bcm_archive(&mod_dir, &bcm_path, compression, &log_callback) {
            Ok(_) => {
                let success_message = format!("\nSuccessfully Created {}!", bcm_filename);
                info!("{}", success_message);
                let _ = transmitter.send(ExportEvent::Success(success_message));
            },
            Err(error) => {
                error!("BCM Packaging failed: {}", error);
                let _ = transmitter.send(ExportEvent::Error(format!("Packaging Error: {}", error)));
            }
        }
    });
}

fn count_target_files(source_dir: &Path, target_directories: &[&str]) -> usize {
    let mut count = 0;
    for dir_name in target_directories {
        let dir_path = source_dir.join(dir_name);
        if dir_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir_path) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

fn build_bcm_archive(
    source_dir: &Path,
    output_file: &Path,
    compression_level: i64,
    log_callback: &impl Fn(String)
) -> Result<usize, String> {

    let file = File::create(output_file).map_err(|e| format!("Failed to create BCM file: {}", e))?;
    let mut zip = ZipWriter::new(file);

    let clamped_level = compression_level.clamp(BCM_COMPRESSION_MIN, BCM_COMPRESSION_MAX);
    trace!("Using compression level: {}", clamped_level);

    let method = if clamped_level == 0 {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };

    let options = SimpleFileOptions::default()
        .compression_method(method)
        .compression_level(if method == CompressionMethod::Stored { None } else { Some(clamped_level) })
        .unix_permissions(0o755);

    let target_directories = ["", "patch", "loose", "icons"];
    let total_files = count_target_files(source_dir, &target_directories);

    if total_files == 0 {
        return Err("No files found to package.".to_string());
    }

    let log_interval = (total_files / 10).max(1);
    let mut processed_files = 0;

    for dir_name in target_directories {
        let dir_path = source_dir.join(dir_name);

        if !dir_path.exists() || !dir_path.is_dir() {
            debug!("Skipping missing target directory: '{}'", dir_name);
            continue;
        }

        trace!("Indexing target directory: '{}'", if dir_name.is_empty() { "Mod Root" } else { dir_name });
        if let Err(e) = write_flat_directory_to_zip(
            &mut zip, &dir_path, dir_name, options, total_files, log_interval, &mut processed_files, log_callback
        ) {
            warn!("Directory '{}' packaging encountered an error: {}", dir_name, e);
            return Err(e);
        }
    }

    debug!("Finalizing zip stream...");
    if let Err(e) = zip.finish() {
        error!("Zip closure failed: {}", e);
        return Err(format!("Archive finalization failed: {}", e));
    }

    Ok(processed_files)
}

#[allow(clippy::too_many_arguments)]
fn write_flat_directory_to_zip(
    zip: &mut ZipWriter<File>,
    current_dir: &Path,
    relative_prefix: &str,
    options: SimpleFileOptions,
    total_files: usize,
    log_interval: usize,
    processed_files: &mut usize,
    log_callback: &impl Fn(String)
) -> Result<(), String> {

    let entries = fs::read_dir(current_dir).map_err(|e| format!("Failed to read dir {}: {}", current_dir.display(), e))?;

    for entry_result in entries {
        let Ok(entry) = entry_result else {
            warn!("Skipped unreadable entry in {}", current_dir.display());
            continue;
        };

        let path = entry.path();

        if path.is_dir() {
            trace!("Ignoring subfolder to enforce flat structure: {}", path.display());
            continue;
        }

        if path.is_file() {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            let zip_path = if relative_prefix.is_empty() {
                file_name.to_string()
            } else {
                format!("{}/{}", relative_prefix, file_name)
            };

            trace!("Deflating file into archive: {}", zip_path);

            if let Err(e) = zip.start_file(&zip_path, options) {
                return Err(format!("Zip writer rejected file {}: {}", zip_path, e));
            }

            let mut file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => return Err(format!("Failed to open file for read {}: {}", path.display(), e)),
            };

            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer) {
                return Err(format!("Failed reading file data {}: {}", path.display(), e));
            }

            if let Err(e) = zip.write_all(&buffer) {
                return Err(format!("Failed writing zip data {}: {}", zip_path, e));
            }

            *processed_files += 1;

            if *processed_files % log_interval == 0 || *processed_files == total_files {
                log_callback(format!("Packed {} files | Streaming: {}", *processed_files, file_name));
            }
        }
    }

    Ok(())
}