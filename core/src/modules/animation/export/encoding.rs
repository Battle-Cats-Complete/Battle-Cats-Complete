use std::fs;
use std::io::{BufWriter, Cursor, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use gif::{
    DisposalMethod, Encoder as GifEncoder,
    Frame as GifFrame, Repeat as GifRepeat,
};
use image::RgbaImage;
use tracing::{error, info, warn};
use webp_animation::Encoder as WebpEncoder;

use super::{EncoderMessage, EncoderStatus, ExportConfig, ExportFormat};

pub(crate) fn encode_native(
    config: ExportConfig,
    receiver: mpsc::Receiver<EncoderMessage>,
    emit: &(dyn Fn(EncoderStatus) + Sync),
    temp_path: &PathBuf,
    abort_signal: &AtomicBool
) -> bool {
    let mut frames_processed = 0;
    let mut is_success = false;

    match config.format {
        ExportFormat::Gif => {
            let Ok(file) = fs::File::create(temp_path) else {
                error!("Failed to create temporary GIF file at {:?}", temp_path);
                return false;
            };
            let mut buffered_writer = BufWriter::new(file);

            let Ok(mut gif_encoder) = GifEncoder::new(&mut buffered_writer, config.width as u16, config.height as u16, &[]) else {
                error!("Failed to instantiate native GIF encoder.");
                return false;
            };

            if let Err(e) = gif_encoder.set_repeat(GifRepeat::Infinite) {
                warn!("Failed to set GIF repeat loop: {}", e);
            }

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) {
                    info!("GIF encoding aborted midway.");
                    return false;
                }

                match message {
                    EncoderMessage::Frame(raw_pixels, width, height, delay_milliseconds) => {
                        let image_data = prepare_image(raw_pixels, width, height, config.background);
                        let mut frame_ticks = (delay_milliseconds as f32 / 10.0).round() as u16;
                        if frame_ticks < 2 { frame_ticks = 2; }

                        let mut pixel_buffer = image_data.into_vec();
                        if !config.background {
                            for chunk in pixel_buffer.chunks_exact_mut(4) {
                                if chunk[3] < 127 {
                                    chunk[0] = 0; chunk[1] = 0; chunk[2] = 0; chunk[3] = 0;
                                } else {
                                    chunk[3] = 255;
                                }
                            }
                        }

                        let mut gif_frame = GifFrame::from_rgba(config.width as u16, config.height as u16, &mut pixel_buffer);
                        gif_frame.dispose = DisposalMethod::Background;
                        gif_frame.delay = frame_ticks;

                        if let Err(e) = gif_encoder.write_frame(&gif_frame) {
                            error!("Native GIF encoder broke during write_frame: {}", e);
                            break;
                        }

                        frames_processed += 1;
                        emit(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => {
                        is_success = true;
                        break;
                    }
                }
            }
        },
        ExportFormat::WebP => {
            let Ok(mut webp_encoder) = WebpEncoder::new((config.width, config.height)) else {
                error!("Failed to instantiate native WebP encoder.");
                return false;
            };
            let mut timestamp_milliseconds = 0;

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) {
                    info!("WebP encoding aborted midway.");
                    return false;
                }

                match message {
                    EncoderMessage::Frame(raw_pixels, width, height, delay_milliseconds) => {
                        let image_data = prepare_image(raw_pixels, width, height, config.background);

                        if let Err(e) = webp_encoder.add_frame(&image_data.into_vec(), timestamp_milliseconds) {
                            error!("Failed adding frame to WebP encoder: {}", e);
                            break;
                        }

                        timestamp_milliseconds += delay_milliseconds as i32;
                        frames_processed += 1;

                        emit(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => {
                        is_success = true;
                        break;
                    }
                }
            }

            if is_success && !abort_signal.load(Ordering::Relaxed) {
                match webp_encoder.finalize(timestamp_milliseconds) {
                    Ok(final_data) => {
                        is_success = fs::write(temp_path, final_data).is_ok();
                        if !is_success { error!("Failed writing finalized WebP data to disk."); }
                    },
                    Err(e) => {
                        error!("Failed to finalize WebP encoding: {:?}", e);
                        is_success = false;
                    }
                }
            } else {
                is_success = false;
            }
        },
        ExportFormat::Zip => {
            let mut frame_index = 0;
            let step_direction = if config.start_frame <= config.end_frame { 1 } else { -1 };

            let Ok(file) = fs::File::create(temp_path) else {
                error!("Failed to create temporary Zip file at {:?}", temp_path);
                return false;
            };

            let mut zip_writer = zip::ZipWriter::new(BufWriter::new(file));

            let compression_method = if config.compression_percent == 0 {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            };

            let zip_options = zip::write::SimpleFileOptions::default().compression_method(compression_method);

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) {
                    info!("Zip exporting aborted midway.");
                    return false;
                }

                match message {
                    EncoderMessage::Frame(raw_pixels, width, height, _) => {
                        let image_data = prepare_image(raw_pixels, width, height, config.background);
                        let current_frame = config.start_frame + (frame_index * step_direction);
                        let entry_name = format!("{}.{}f.png", config.base_name, current_frame);

                        if let Err(e) = zip_writer.start_file(entry_name, zip_options) {
                            error!("Failed to start Zip file entry: {}", e);
                            break;
                        }

                        let mut memory_buffer = Cursor::new(Vec::new());
                        if let Err(e) = image_data.write_to(&mut memory_buffer, image::ImageFormat::Png) {
                            error!("Failed compiling PNG frame to buffer: {}", e);
                            break;
                        }

                        if let Err(e) = zip_writer.write_all(memory_buffer.get_ref()) {
                            error!("Failed to write PNG buffer to Zip entry: {}", e);
                            break;
                        }

                        frame_index += 1;
                        frames_processed += 1;

                        emit(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => {
                        is_success = true;
                        break;
                    },
                }
            }
            if let Err(e) = zip_writer.finish() {
                error!("Failed finalizing the Zip archive structure: {}", e);
                is_success = false;
            }
        },
        _ => {
            warn!("Native export method unsupported for requested format.");
        }
    }
    is_success
}

pub(crate) fn prepare_image(mut pixel_buffer: Vec<u8>, width: u32, height: u32, is_opaque_background: bool) -> RgbaImage {
    for chunk in pixel_buffer.chunks_exact_mut(4) {
        if is_opaque_background {
            chunk[3] = 255;
        } else {
            let alpha_value = chunk[3].max(chunk[0]).max(chunk[1]).max(chunk[2]);
            chunk[3] = alpha_value;

            if alpha_value > 0 && alpha_value < 255 {
                let float_alpha = alpha_value as f32 / 255.0;
                chunk[0] = (chunk[0] as f32 / float_alpha).min(255.0) as u8;
                chunk[1] = (chunk[1] as f32 / float_alpha).min(255.0) as u8;
                chunk[2] = (chunk[2] as f32 / float_alpha).min(255.0) as u8;
            }
        }
    }

    let image_buffer = match RgbaImage::from_raw(width, height, pixel_buffer) {
        Some(buffer) => buffer,
        None => {
            error!("Failed converting raw frame array back to structured RgbaImage. Recovering with empty image.");
            return RgbaImage::new(width, height);
        }
    };

    image::imageops::flip_vertical(&image_buffer)
}