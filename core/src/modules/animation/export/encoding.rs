use std::fs;
use std::io::{BufWriter, Cursor, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use gif::{
    DisposalMethod, Encoder as GifEncoder,
    Frame as GifFrame, Repeat as GifRepeat
};
use glow::HasContext;
use image::RgbaImage;
use nyanko::graphics::engine::resolve_frame;
use nyanko::graphics::rig::{Animation, Unit};
use webp_animation::Encoder as WebpEncoder;

use crate::modules::animation::logic::canvas::GlowRenderer;

#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_zoom: f32,
    pub format: ExportFormat,
    pub quality_percent: u32,
    pub compression_percent: u32,
    pub fps: u32,
    pub start_frame: i32,
    pub end_frame: i32,
    pub interpolation: bool,
    pub output_path: PathBuf,
    pub base_name: String,
    pub background: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat {
    Gif,
    WebP,
    Avif,
    Png,
    Mp4,
    Mkv,
    Webm,
    Zip
}

pub enum EncoderMessage {
    Frame(Vec<u8>, u32, u32, u32),
    Finish,
}

#[derive(Debug, Clone)]
pub enum EncoderStatus {
    #[allow(dead_code)] Encoding,
    Progress(u32),
    Finished,
}

pub fn encode_native(
    config: ExportConfig,
    receiver: mpsc::Receiver<EncoderMessage>,
    status_sender: mpsc::Sender<EncoderStatus>,
    temp_path: &PathBuf,
    abort_signal: Arc<AtomicBool>
) -> bool {
    let mut frames_processed = 0;
    let mut is_success = false;

    match config.format {
        ExportFormat::Gif => {
            let Ok(file) = fs::File::create(temp_path) else { return false; };
            let mut buffered_writer = BufWriter::new(file);

            let Ok(mut gif_encoder) = GifEncoder::new(&mut buffered_writer, config.width as u16, config.height as u16, &[]) else { return false; };
            let _ = gif_encoder.set_repeat(GifRepeat::Infinite);

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }

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

                        if gif_encoder.write_frame(&gif_frame).is_err() { break; }
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; }
                }
            }
        },
        ExportFormat::WebP => {
            let Ok(mut webp_encoder) = WebpEncoder::new((config.width, config.height)) else { return false; };
            let mut timestamp_milliseconds = 0;

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }

                match message {
                    EncoderMessage::Frame(raw_pixels, width, height, delay_milliseconds) => {
                        let image_data = prepare_image(raw_pixels, width, height, config.background);
                        let _ = webp_encoder.add_frame(&image_data.into_vec(), timestamp_milliseconds);
                        timestamp_milliseconds += delay_milliseconds as i32;
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; }
                }
            }

            if is_success && !abort_signal.load(Ordering::Relaxed) {
                let Ok(final_data) = webp_encoder.finalize(timestamp_milliseconds) else { return false; };
                is_success = fs::write(temp_path, final_data).is_ok();
            } else {
                is_success = false;
            }
        },
        ExportFormat::Zip => {
            let mut frame_index = 0;
            let step_direction = if config.start_frame <= config.end_frame { 1 } else { -1 };
            let Ok(file) = fs::File::create(temp_path) else { return false; };
            let mut zip_writer = zip::ZipWriter::new(BufWriter::new(file));

            let compression_method = if config.compression_percent == 0 {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            };

            let zip_options = zip::write::SimpleFileOptions::default().compression_method(compression_method);

            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }

                match message {
                    EncoderMessage::Frame(raw_pixels, width, height, _) => {
                        let image_data = prepare_image(raw_pixels, width, height, config.background);
                        let current_frame = config.start_frame + (frame_index * step_direction);
                        let entry_name = format!("{}.{}f.png", config.base_name, current_frame);

                        let _ = zip_writer.start_file(entry_name, zip_options);
                        let mut memory_buffer = Cursor::new(Vec::new());

                        if image_data.write_to(&mut memory_buffer, image::ImageFormat::Png).is_ok() {
                            let _ = zip_writer.write_all(memory_buffer.get_ref());
                        }

                        frame_index += 1;
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; },
                }
            }
            let _ = zip_writer.finish();
        },
        _ => {}
    }
    is_success
}

pub fn render_frame(
    renderer: &mut GlowRenderer,
    gl_context: &glow::Context,
    width: u32,
    height: u32,
    unit: &Unit,
    animation: Option<&Animation>,
    frame_time: f32,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    background_color: [u8; 4],
) -> Result<Vec<u8>, String> {
    unsafe {
        gl_context.disable(glow::SCISSOR_TEST);

        let framebuffer = gl_context.create_framebuffer()
            .map_err(|error| format!("Failed to create OpenGL framebuffer: {}", error))?;

        gl_context.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        let texture = gl_context.create_texture()
            .map_err(|error| format!("Failed to create OpenGL texture: {}", error))?;

        gl_context.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl_context.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width as i32, height as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
        gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl_context.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(texture), 0);
        gl_context.bind_texture(glow::TEXTURE_2D, None);

        gl_context.viewport(0, 0, width as i32, height as i32);

        let (red, green, blue, alpha) = (
            background_color[0] as f32 / 255.0,
            background_color[1] as f32 / 255.0,
            background_color[2] as f32 / 255.0,
            background_color[3] as f32 / 255.0
        );

        gl_context.clear_color(red, green, blue, alpha);
        gl_context.clear(glow::COLOR_BUFFER_BIT);

        let mut geometry = resolve_frame(unit, animation, frame_time);
        for part in &mut geometry {
            if part.glow == 1 || part.glow == 3 {
                part.glow = 1;
            } else if part.glow == 2 {
                part.glow = 0;
            }
        }

        let _ = renderer.draw_frame(
            gl_context,
            &geometry,
            &unit.sheet,
            width as f32,
            height as f32,
            pan_x,
            pan_y,
            zoom
        );

        gl_context.pixel_store_i32(glow::PACK_ALIGNMENT, 1);

        let mut pixel_buffer = vec![0u8; (width * height * 4) as usize];
        gl_context.read_pixels(0, 0, width as i32, height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut pixel_buffer));

        gl_context.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl_context.delete_framebuffer(framebuffer);
        gl_context.delete_texture(texture);

        gl_context.enable(glow::SCISSOR_TEST);
        gl_context.pixel_store_i32(glow::PACK_ALIGNMENT, 4);

        Ok(pixel_buffer)
    }
}

pub fn prepare_image(mut pixel_buffer: Vec<u8>, width: u32, height: u32, is_opaque_background: bool) -> RgbaImage {
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

    let Some(image_buffer) = RgbaImage::from_raw(width, height, pixel_buffer) else {
        return RgbaImage::new(width, height);
    };

    image::imageops::flip_vertical(&image_buffer)
}