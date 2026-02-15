use std::fs;
use std::io::{Cursor, Write, BufWriter};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use eframe::glow::{self, HasContext};
use image::RgbaImage;
use zip::write::FileOptions;
use webp_animation::Encoder as WebpEncoder;
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat as GifRepeat, DisposalMethod};

use crate::core::anim::canvas::GlowRenderer;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::transform::WorldTransform;
use crate::core::addons::toolpaths::{self, Presence};

use crate::core::addons::avifenc::encoding as avif_addon;
use crate::core::addons::ffmpeg::encoding as ffmpeg_addon;

#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    #[allow(dead_code)] pub camera_x: f32,
    #[allow(dead_code)] pub camera_y: f32,
    #[allow(dead_code)] pub camera_zoom: f32,
    pub format: ExportFormat,
    pub quality_percent: u32, 
    pub compression_percent: u32,
    pub fps: u32,
    pub start_frame: i32,
    pub end_frame: i32,
    #[allow(dead_code)] pub interpolation: bool,
    pub output_path: PathBuf,
    pub base_name: String, 
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat { Gif, WebP, Avif, PngSequence }

pub enum EncoderMessage {
    Frame(Vec<u8>, u32, u32, u32),
    Finish,
}

#[derive(Debug, Clone)]
pub enum EncoderStatus {
    Encoding, 
    Progress(u32),
    Finished,
}

pub fn start_encoding_thread(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>,
    status_tx: mpsc::Sender<EncoderStatus>
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Some(parent) = config.output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let ext = match config.format {
            ExportFormat::Gif => "gif",
            ExportFormat::WebP => "webp",
            ExportFormat::Avif => "avif",
            ExportFormat::PngSequence => "png", 
        };
        
        let file_stem = config.output_path.file_stem().unwrap_or_default().to_string_lossy();
        let temp_filename = format!("{}.{}.tmp", file_stem, ext);
        let temp_path = config.output_path.with_file_name(temp_filename);
        
        let final_path = config.output_path.clone();
        let final_tx = status_tx.clone();

        // --- 1. FFMPEG (GIF) ---
        if toolpaths::ffmpeg_status() == Presence::Installed && config.format == ExportFormat::Gif {
            if ffmpeg_addon::encode(config, rx, status_tx, &temp_path) {
                finalize_export(&temp_path, &final_path, &final_tx);
            }
            return;
        }

        // --- 2. AVIF PIPE ---
        if toolpaths::avifenc_status() == Presence::Installed && toolpaths::ffmpeg_status() == Presence::Installed && config.format == ExportFormat::Avif {
            if avif_addon::encode(config, rx, status_tx, &temp_path) {
                finalize_export(&temp_path, &final_path, &final_tx);
            }
            return;
        }

        // --- 3. NATIVE FALLBACK ---
        let mut frames_processed = 0;
        let mut success = false;

        match config.format {
            ExportFormat::Gif => {
                if let Ok(file) = fs::File::create(&temp_path) {
                    let mut writer = BufWriter::new(file);
                    if let Ok(mut encoder) = GifEncoder::new(&mut writer, config.width as u16, config.height as u16, &[]) {
                        let _ = encoder.set_repeat(GifRepeat::Infinite);

                        while let Ok(msg) = rx.recv() {
                            match msg {
                                EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                                    let img = prepare_image(raw_pixels, w, h);
                                    let mut ticks = (delay_ms as f32 / 10.0).round() as u16;
                                    if ticks < 2 { ticks = 2; } 
                                    
                                    let mut pixels = img.into_vec();
                                    for chunk in pixels.chunks_exact_mut(4) {
                                        if chunk[3] < 127 { chunk[0]=0; chunk[1]=0; chunk[2]=0; chunk[3]=0; } 
                                        else { chunk[3]=255; }
                                    }

                                    let mut frame = GifFrame::from_rgba(config.width as u16, config.height as u16, &mut pixels);
                                    frame.dispose = DisposalMethod::Background;
                                    frame.delay = ticks;
                                    
                                    if encoder.write_frame(&frame).is_err() { break; }
                                    
                                    frames_processed += 1;
                                    if status_tx.send(EncoderStatus::Progress(frames_processed)).is_err() { break; }
                                },
                                EncoderMessage::Finish => {
                                    let _ = status_tx.send(EncoderStatus::Encoding);
                                    success = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            },
            ExportFormat::WebP => {
                let mut encoder = WebpEncoder::new((config.width, config.height)).expect("Failed WebP");
                let mut timestamp_ms = 0;
                while let Ok(msg) = rx.recv() {
                    match msg {
                        EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                            let img = prepare_image(raw_pixels, w, h);
                            let _ = encoder.add_frame(&img.into_vec(), timestamp_ms);
                            timestamp_ms += delay_ms as i32;

                            frames_processed += 1;
                            if status_tx.send(EncoderStatus::Progress(frames_processed)).is_err() { break; }
                        },
                        EncoderMessage::Finish => {
                            success = true;
                            break;
                        }
                    }
                }
                if success {
                    if let Ok(data) = encoder.finalize(timestamp_ms) {
                        let _ = std::fs::write(&temp_path, data);
                    } else { success = false; }
                }
            },
            ExportFormat::PngSequence => {
                let _ = status_tx.send(EncoderStatus::Finished);
                return;
            },
            ExportFormat::Avif => {
                let _ = status_tx.send(EncoderStatus::Finished);
                return;
            }
        }
        
        if success {
            finalize_export(&temp_path, &final_path, &final_tx);
        } else {
            if temp_path.exists() { let _ = fs::remove_file(temp_path); }
        }
    })
}

fn finalize_export(temp: &PathBuf, output: &PathBuf, status_tx: &mpsc::Sender<EncoderStatus>) {
    if temp.exists() {
        if output.exists() { let _ = fs::remove_file(output); }
        let _ = fs::rename(temp, output);
    }
    let _ = status_tx.send(EncoderStatus::Finished);
}

// ... (render_frame and prepare_image unchanged)
pub fn render_frame(
    renderer: &mut GlowRenderer,
    gl: &glow::Context,
    width: u32,
    height: u32,
    parts: &[WorldTransform],
    sheet: &SpriteSheet,
    pan: egui::Vec2,
    zoom: f32,
    bg_color: [u8; 4],
) -> Vec<u8> {
    unsafe {
        gl.disable(glow::SCISSOR_TEST);
        let fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width as i32, height as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.viewport(0, 0, width as i32, height as i32);
        let (r, g, b, a) = (bg_color[0] as f32 / 255.0, bg_color[1] as f32 / 255.0, bg_color[2] as f32 / 255.0, bg_color[3] as f32 / 255.0);
        gl.clear_color(r, g, b, a);
        gl.clear(glow::COLOR_BUFFER_BIT);
        renderer.paint(gl, egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32)), parts, sheet, pan, zoom, true);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        gl.read_pixels(0, 0, width as i32, height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut pixels));
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.delete_framebuffer(fbo);
        gl.delete_texture(tex);
        gl.enable(glow::SCISSOR_TEST);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
        pixels
    }
}

pub fn prepare_image(mut pixels: Vec<u8>, width: u32, height: u32) -> RgbaImage {
    for chunk in pixels.chunks_exact_mut(4) {
        let alpha = chunk[3];
        if alpha > 0 && alpha < 255 {
            let a = alpha as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }
    if let Some(img) = RgbaImage::from_raw(width, height, pixels) { image::imageops::flip_vertical(&img) } 
    else { RgbaImage::new(width, height) }
}