use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{error, info, trace, warn};

use crate::addons::avifenc::encoding as avifenc;
use crate::addons::ffmpeg::encoding as ffmpeg;
use crate::addons::paths::{self, Presence};

use super::{
    encoding, EncoderMessage, EncoderStatus, ExportConfig, ExportFormat,
};

pub fn run(
    config: ExportConfig,
    receiver: mpsc::Receiver<EncoderMessage>,
    emit: impl Fn(EncoderStatus) + Sync,
    abort_signal: &AtomicBool,
) {
    info!("Encoding worker initialized for format: {:?}", config.format);

    if let Some(parent_directory) = config.output_path.parent()
        && let Err(e) = fs::create_dir_all(parent_directory) {
        error!("Failed to create output directory: {}", e);
    }

    let file_extension = match config.format {
        ExportFormat::Gif => "gif",
        ExportFormat::WebP => "webp",
        ExportFormat::Avif => "avif",
        ExportFormat::Png => "png",
        ExportFormat::Mp4 => "mp4",
        ExportFormat::Mkv => "mkv",
        ExportFormat::Webm => "webm",
        ExportFormat::Zip => "zip",
    };

    let file_stem = config.output_path.file_stem().unwrap_or_default().to_string_lossy();
    let job_stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let temporary_filename = format!(".{}.{}.{}.tmp", file_stem, job_stamp, file_extension);
    let temporary_path = config.output_path.with_file_name(&temporary_filename);
    let final_path = config.output_path.clone();

    trace!("Generating temporary file path: {:?}", temporary_path);

    let is_success = match config.format {
        ExportFormat::Avif if paths::avifenc_status() == Presence::Installed => {
            info!("Delegating to external AVIF encoder");
            avifenc::encode(config.clone(), receiver, &emit, &temporary_path, abort_signal)
        },

        ExportFormat::Gif | ExportFormat::WebP | ExportFormat::Png | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm | ExportFormat::Avif
        if paths::ffmpeg_status() == Presence::Installed => {
            info!("Delegating to external FFmpeg encoder");
            ffmpeg::encode(config.clone(), receiver, &emit, &temporary_path, abort_signal)
        },

        _ => {
            info!("Delegating to native generic encoder");
            encoding::encode_native(config.clone(), receiver, &emit, &temporary_path, abort_signal)
        }
    };

    let is_aborted = abort_signal.load(Ordering::Relaxed);
    let should_save_file = is_success && !is_aborted;

    if !should_save_file {
        warn!("Encoding aborted or failed. Cleaning up temporary files.");
        if temporary_path.exists()
            && let Err(e) = fs::remove_file(&temporary_path) {
            error!("Failed to remove temporary file: {}", e);
        }
        emit(EncoderStatus::Finished);
        return;
    }

    if !temporary_path.exists() {
        error!("Temporary file missing after supposedly successful encoding.");
        emit(EncoderStatus::Finished);
        return;
    }

    if final_path.exists() {
        trace!("Overwriting existing export at: {:?}", final_path);
        if let Err(e) = fs::remove_file(&final_path) {
            error!("Failed to remove existing final file: {}", e);
        }
    }

    if let Err(e) = fs::rename(&temporary_path, &final_path) {
        error!("Failed to rename temporary file to final path: {}", e);
    } else {
        info!("Successfully saved encoded animation to {:?}", final_path);
    }

    emit(EncoderStatus::Finished);
}
