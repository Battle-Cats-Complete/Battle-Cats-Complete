use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use tracing::{debug, error, info, warn};

use crate::modules::animation::export::{
    EncoderMessage, EncoderStatus,
    ExportConfig, ExportFormat,
};
use crate::modules::animation::export::encoding::prepare_image;

use super::get_ffmpeg_path;

pub fn encode(
    config: ExportConfig,
    receiver: mpsc::Receiver<EncoderMessage>,
    emit: &(dyn Fn(EncoderStatus) + Sync),
    temp_path: &PathBuf,
    abort_signal: &AtomicBool
) -> bool {
    let Some(ffmpeg_path) = get_ffmpeg_path() else {
        error!("FFmpeg binary not found. Aborting encode.");
        return false;
    };

    let mut arguments = vec![
        "-nostdin".to_string(),
        "-f".to_string(), "rawvideo".to_string(),
        "-pixel_format".to_string(), "rgba".to_string(),
        "-video_size".to_string(), format!("{}x{}", config.width, config.height),
        "-framerate".to_string(), config.fps.to_string(),
        "-i".to_string(), "-".to_string(),
    ];

    match config.format {
        ExportFormat::Gif => {
            info!("Configuring FFmpeg for GIF export");
            let dither = if config.quality_percent >= 80 {
                "sierra2_4a"
            } else if config.quality_percent >= 40 {
                "floyd_steinberg"
            } else {
                "bayer:bayer_scale=5"
            };

            let stats_mode = if config.compression_percent < 50 { "full" } else { "diff" };
            let filter = format!("split[s0][s1];[s0]palettegen=stats_mode={}[p];[s1][p]paletteuse=dither={}", stats_mode, dither);

            arguments.extend_from_slice(&[
                "-vf".to_string(), filter,
                "-f".to_string(), "gif".to_string(),
            ]);
        },
        ExportFormat::WebP => {
            info!("Configuring FFmpeg for WebP export");
            let compression_level = (config.compression_percent as f32 / 100.0 * 6.0).round() as u8;
            arguments.extend_from_slice(&[
                "-c:v".to_string(), "libwebp_anim".to_string(),
                "-loop".to_string(), "0".to_string(),
                "-q:v".to_string(), config.quality_percent.to_string(),
                "-compression_level".to_string(), compression_level.to_string(),
                "-preset".to_string(), "drawing".to_string(),
                "-threads".to_string(), "0".to_string(),
                "-f".to_string(), "webp".to_string(),
            ]);
        },
        ExportFormat::Png => {
            info!("Configuring FFmpeg for APNG export");
            arguments.extend_from_slice(&[
                "-plays".to_string(), "0".to_string(),
                "-c:v".to_string(), "apng".to_string(),
                "-f".to_string(), "apng".to_string(),
            ]);
        },
        ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm => {
            info!("Configuring FFmpeg for Video export");
            let use_av1 = config.quality_percent > 90 && config.compression_percent > 90;
            let needs_even_dims = use_av1 || config.format != ExportFormat::Webm;

            if needs_even_dims {
                arguments.extend_from_slice(&["-vf".to_string(), "crop=trunc(iw/2)*2:trunc(ih/2)*2".to_string()]);
            }

            if use_av1 {
                debug!("Using AV1 codec");
                let crf_value = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0);
                let cpu_used_value = 4.0 + (config.compression_percent as f32 / 100.0 * 4.0);

                arguments.extend_from_slice(&[
                    "-c:v".to_string(), "libaom-av1".to_string(),
                    "-pix_fmt".to_string(), "yuv420p".to_string(),
                    "-crf".to_string(), format!("{:.0}", crf_value),
                    "-cpu-used".to_string(), format!("{:.0}", cpu_used_value),
                    "-b:v".to_string(), "0".to_string(),
                    "-strict".to_string(), "experimental".to_string(),
                ]);
            } else {
                match config.format {
                    ExportFormat::Webm => {
                        debug!("Using VP9 codec for Webm");
                        let crf_value = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0);
                        arguments.extend_from_slice(&[
                            "-c:v".to_string(), "libvpx-vp9".to_string(),
                            "-pix_fmt".to_string(), "yuva420p".to_string(),
                            "-crf".to_string(), format!("{:.0}", crf_value),
                            "-b:v".to_string(), "0".to_string(),
                        ]);
                    },
                    _ => {
                        debug!("Using x264 codec");
                        let crf_value = 51.0 - (config.quality_percent as f32 / 100.0 * 33.0);
                        let presets = ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"];
                        let preset_index = (config.compression_percent as f32 / 100.0 * 8.0).round() as usize;

                        arguments.extend_from_slice(&[
                            "-c:v".to_string(), "libx264".to_string(),
                            "-pix_fmt".to_string(), "yuv420p".to_string(),
                            "-profile:v".to_string(), "main".to_string(),
                            "-crf".to_string(), format!("{:.0}", crf_value),
                            "-preset".to_string(), presets[preset_index].to_string(),
                        ]);
                    }
                }
            }

            let container_format = match config.format {
                ExportFormat::Mp4 => "mp4",
                ExportFormat::Mkv => "matroska",
                ExportFormat::Webm => "webm",
                _ => "mp4",
            };
            arguments.extend_from_slice(&["-f".to_string(), container_format.to_string()]);
        },
        _ => {
            error!("Unsupported format selected for FFmpeg.");
            return false;
        },
    }

    arguments.push("-y".to_string());
    arguments.push(temp_path.to_string_lossy().to_string());

    let mut command_builder = Command::new(&ffmpeg_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command_builder.creation_flags(0x08000000);
    }

    let Ok(mut child_process) = command_builder.args(&arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn() else {
        error!("Failed to spawn FFmpeg process.");
        return false;
    };

    let Some(mut ffmpeg_stdin) = child_process.stdin.take() else {
        error!("Failed to open FFmpeg stdin pipe.");
        let _ = child_process.kill();
        return false;
    };

    debug!("Starting FFmpeg pixel stream...");
    let mut frames_processed = 0;
    let mut finished_cleanly = false;

    while let Ok(message) = receiver.recv() {
        if abort_signal.load(Ordering::Relaxed) {
            warn!("Abort signal received. Halting pixel stream.");
            break;
        }

        match message {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                emit(EncoderStatus::Progress(frames_processed));

                let image_data = prepare_image(raw_pixels, w, h, config.background);

                if ffmpeg_stdin.write_all(&image_data.into_vec()).is_err() {
                    error!("Failed to write frame {} to FFmpeg stdin.", frames_processed);
                    break;
                }
                frames_processed += 1;
            },
            EncoderMessage::Finish => {
                debug!("Received Finish signal. Total frames passed to FFmpeg: {}", frames_processed);
                finished_cleanly = true;
                break;
            }
        }
    }
    drop(ffmpeg_stdin);

    let did_input_succeed = finished_cleanly;

    if abort_signal.load(Ordering::Relaxed) || !did_input_succeed {
        error!("FFmpeg encoding aborted or stream failed.");
        let _ = child_process.kill();
        let _ = child_process.wait();
        return false;
    }

    let is_success = child_process.wait().map(|status| status.success()).unwrap_or(false);
    if is_success {
        info!("FFmpeg encode completed successfully.");
    } else {
        error!("FFmpeg process finished with an error code.");
    }

    is_success
}