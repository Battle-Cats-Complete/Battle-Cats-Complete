use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use tracing::{error, info, trace, warn};

use crate::modules::addons::avifenc::encoding as avifenc;
use crate::modules::addons::ffmpeg::encoding as ffmpeg;
use crate::modules::addons::paths::{self, Presence};

use super::{
    encoding, EncoderMessage, EncoderStatus, ExportConfig, ExportFormat,
};

pub fn start_encoding_thread(
    config: ExportConfig,
    receiver: mpsc::Receiver<EncoderMessage>,
    status_sender: mpsc::Sender<EncoderStatus>,
    abort_signal: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        info!("Encoding thread initialized for format: {:?}", config.format);

        if let Some(parent_directory) = config.output_path.parent() {
            if let Err(e) = fs::create_dir_all(parent_directory) {
                error!("Failed to create output directory: {}", e);
            }
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
        let temporary_filename = format!("{}.{}.tmp", file_stem, file_extension);
        let temporary_path = config.output_path.with_file_name(&temporary_filename);
        let final_path = config.output_path.clone();
        let final_sender = status_sender.clone();

        trace!("Generating temporary file path: {:?}", temporary_path);

        let is_success = match config.format {
            ExportFormat::Avif if paths::avifenc_status() == Presence::Installed => {
                info!("Delegating to external AVIF encoder");
                avifenc::encode(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            },

            ExportFormat::Gif | ExportFormat::WebP | ExportFormat::Png | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm | ExportFormat::Avif
            if paths::ffmpeg_status() == Presence::Installed => {
                info!("Delegating to external FFmpeg encoder");
                ffmpeg::encode(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            },

            _ => {
                info!("Delegating to native generic encoder");
                encoding::encode_native(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            }
        };

        let is_aborted = abort_signal.load(Ordering::Relaxed);
        let should_save_file = is_success && !is_aborted;

        if !should_save_file {
            warn!("Encoding aborted or failed. Cleaning up temporary files.");
            if temporary_path.exists() {
                if let Err(e) = fs::remove_file(&temporary_path) {
                    error!("Failed to remove temporary file: {}", e);
                }
            }
            if let Err(e) = final_sender.send(EncoderStatus::Finished) {
                error!("Failed to send finished status: {}", e);
            }
            return;
        }

        if !temporary_path.exists() {
            error!("Temporary file missing after supposedly successful encoding.");
            if let Err(e) = final_sender.send(EncoderStatus::Finished) {
                error!("Failed to send finished status: {}", e);
            }
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

        if let Err(e) = final_sender.send(EncoderStatus::Finished) {
            error!("Failed to send finished status: {}", e);
        }
    })
}