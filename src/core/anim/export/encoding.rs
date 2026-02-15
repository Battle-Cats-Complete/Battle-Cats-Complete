use std::env;
use std::fs;
use std::io::{self, Cursor, Write, BufWriter};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use eframe::glow::{self, HasContext};
use image::RgbaImage;
use zip::ZipArchive;
use zip::write::FileOptions;
use webp_animation::Encoder as WebpEncoder;
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat as GifRepeat, DisposalMethod};

use crate::core::anim::canvas::GlowRenderer;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::transform::WorldTransform;

// ==================================================================================
// CONFIGURATION & TYPES
// ==================================================================================

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_zoom: f32,
    pub format: ExportFormat,
    
    // Percentages (0-100)
    pub quality_percent: u32, 
    pub compression_percent: u32,

    pub fps: u32,
    pub start_frame: i32,
    pub end_frame: i32,
    pub interpolation: bool,
    pub output_path: PathBuf,
    pub base_name: String, 
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat {
    Gif,
    WebP,
    Avif,
    PngSequence,
}

pub enum EncoderMessage {
    // raw_pixels, width, height, delay_ms
    Frame(Vec<u8>, u32, u32, u32),
    Finish,
}

#[derive(Debug, Clone)]
pub enum EncoderStatus {
    Encoding, 
    Progress(u32),
    Finished,
}

// ==================================================================================
// RENDERER BRIDGE
// ==================================================================================

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
        gl.tex_image_2d(
            glow::TEXTURE_2D, 0, glow::RGBA as i32,
            width as i32, height as i32, 0,
            glow::RGBA, glow::UNSIGNED_BYTE, None
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        gl.bind_texture(glow::TEXTURE_2D, None);

        gl.viewport(0, 0, width as i32, height as i32);

        let (r, g, b, a) = (
            bg_color[0] as f32 / 255.0, 
            bg_color[1] as f32 / 255.0, 
            bg_color[2] as f32 / 255.0, 
            bg_color[3] as f32 / 255.0
        );
        gl.clear_color(r, g, b, a);
        gl.clear(glow::COLOR_BUFFER_BIT);

        renderer.paint(
            gl, 
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32)),
            parts, 
            sheet, 
            pan, 
            zoom, 
            true
        );

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

fn prepare_image(mut pixels: Vec<u8>, width: u32, height: u32) -> RgbaImage {
    // We KEEP this loop to fix premultiplied alpha from OpenGL.
    // Without this, colors look dark/wrong.
    for chunk in pixels.chunks_exact_mut(4) {
        let alpha = chunk[3];
        if alpha > 0 && alpha < 255 {
            let a = alpha as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }

    if let Some(img) = RgbaImage::from_raw(width, height, pixels) {
        image::imageops::flip_vertical(&img)
    } else {
        RgbaImage::new(width, height)
    }
}

// ==================================================================================
// ENCODER LOGIC
// ==================================================================================

pub fn start_encoding_thread(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>,
    status_tx: mpsc::Sender<EncoderStatus>
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Some(parent) = config.output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let temp_path = config.output_path.with_extension("tmp");
        let mut frames_processed = 0;

        match config.format {
            ExportFormat::Gif => {
                let mut buffer = Vec::new();
                {
                    let mut writer = Cursor::new(&mut buffer);
                    let mut encoder = match GifEncoder::new(&mut writer, config.width as u16, config.height as u16, &[]) {
                        Ok(e) => e, Err(_) => { return; }
                    };
                    let _ = encoder.set_repeat(GifRepeat::Infinite);

                    while let Ok(msg) = rx.recv() {
                        match msg {
                            EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                                let img = prepare_image(raw_pixels, w, h);
                                let mut ticks = (delay_ms as f32 / 10.0).round() as u16;
                                if ticks < 2 { ticks = 2; } 
                                
                                let mut pixels = img.into_vec();

                                // FIX: Hard Alpha Threshold for GIF Transparency
                                // This removes semi-transparent fringes that cause "thick black lines"
                                for chunk in pixels.chunks_exact_mut(4) {
                                    // 127 is the 50% cutoff
                                    if chunk[3] < 127 {
                                        // Force full transparent
                                        chunk[0] = 0; chunk[1] = 0; chunk[2] = 0; chunk[3] = 0;
                                    } else {
                                        // Force full opaque
                                        chunk[3] = 255;
                                    }
                                }

                                let mut frame = GifFrame::from_rgba(config.width as u16, config.height as u16, &mut pixels);
                                frame.dispose = DisposalMethod::Background; // Clears prev frame for transparency
                                frame.delay = ticks;
                                
                                // We ignore errors to prevent partial exports from crashing the loop
                                let _ = encoder.write_frame(&frame);
                                
                                frames_processed += 1;
                                let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                            },
                            EncoderMessage::Finish => {
                                let _ = status_tx.send(EncoderStatus::Encoding);
                                break;
                            }
                        }
                    }
                } 
                let _ = fs::write(&temp_path, &buffer);
            },
            ExportFormat::WebP => {
                let mut encoder = WebpEncoder::new((config.width, config.height)).expect("Failed WebP");
                let mut timestamp_ms = 0;
                while let Ok(msg) = rx.recv() {
                    match msg {
                        EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                            let img = prepare_image(raw_pixels, w, h);
                            let raw = img.into_vec();
                            let _ = encoder.add_frame(&raw, timestamp_ms);
                            timestamp_ms += delay_ms as i32;

                            frames_processed += 1;
                            let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                        },
                        EncoderMessage::Finish => break
                    }
                }
                if let Ok(data) = encoder.finalize(timestamp_ms) {
                    let _ = std::fs::write(&temp_path, data);
                }
            },
            ExportFormat::Avif => {
                let avif_path = match get_avifenc_path() { Ok(p) => p, Err(_) => { return; } };
                
                let speed_val = 10 - (config.compression_percent as f32 / 10.0).round() as u32;
                let speed_arg = speed_val.clamp(0, 10).to_string();

                let q_val = 63 - ((config.quality_percent as f32 / 100.0) * 63.0).round() as u32;
                let q_arg = q_val.clamp(0, 63).to_string();

                let mut child = Command::new(avif_path)
                    .args(&[
                        "--stdin", "--stdin-format", "raw", 
                        "--width", &config.width.to_string(), 
                        "--height", &config.height.to_string(), 
                        "--depth", "8", 
                        "--fps", &config.fps.to_string(), 
                        "--speed", &speed_arg,
                        "--min", &q_arg, 
                        "--max", &q_arg,
                        "-o", &temp_path.to_string_lossy()
                    ])
                    .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit())
                    .spawn().expect("Failed avifenc");

                if let Some(mut stdin) = child.stdin.take() {
                    while let Ok(msg) = rx.recv() {
                        match msg {
                            EncoderMessage::Frame(raw_pixels, w, h, _) => { 
                                let img = prepare_image(raw_pixels, w, h);
                                let _ = stdin.write_all(&img.into_vec()); 
                                frames_processed += 1;
                                let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                            },
                            EncoderMessage::Finish => break
                        }
                    }
                }
                let _ = child.wait();
            },
            ExportFormat::PngSequence => {
                let mut frame_idx = 0;
                let step = if config.start_frame <= config.end_frame { 1 } else { -1 };
                let is_zip = config.output_path.extension().map_or(false, |e| e.eq_ignore_ascii_case("zip"));

                if is_zip {
                     match fs::File::create(&temp_path) {
                         Ok(file) => {
                             let buf_writer = BufWriter::new(file);
                             let mut zip = zip::ZipWriter::new(buf_writer);
                             
                             let method = if config.compression_percent == 0 {
                                 zip::CompressionMethod::Stored
                             } else {
                                 zip::CompressionMethod::Deflated
                             };
                             
                             let options = FileOptions::default().compression_method(method);
        
                             while let Ok(msg) = rx.recv() {
                                match msg {
                                    EncoderMessage::Frame(raw_pixels, w, h, _) => {
                                        let img = prepare_image(raw_pixels, w, h);
                                        let current_frame = config.start_frame + (frame_idx as i32 * step);
                                        let entry_name = format!("{}.{}f.png", config.base_name, current_frame);
                                        
                                        let _ = zip.start_file(entry_name, options);
                                        
                                        let mut buffer = Cursor::new(Vec::new());
                                        if let Ok(_) = img.write_to(&mut buffer, image::ImageFormat::Png) {
                                            let _ = zip.write_all(buffer.get_ref());
                                        }

                                        frame_idx += 1;
                                        frames_processed += 1;
                                        let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                                    },
                                    EncoderMessage::Finish => break,
                                }
                             }
                             let _ = zip.finish();
                         },
                         Err(e) => {
                             eprintln!("Failed to create temp zip file: {}", e);
                         }
                     }
                } else {
                    while let Ok(msg) = rx.recv() {
                        match msg {
                            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                                 let img = prepare_image(raw_pixels, w, h);
                                 let mut file = std::fs::File::create(&temp_path).expect("Failed to create single png file");
                                 img.write_to(&mut file, image::ImageFormat::Png).expect("Failed to write single png data");
                                 frames_processed += 1;
                                 let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                            },
                            EncoderMessage::Finish => break,
                        }
                    }
                }
            }
        }
        
        if temp_path.exists() {
            let _ = fs::rename(&temp_path, &config.output_path);
        }
        
        let _ = status_tx.send(EncoderStatus::Finished);
    })
}

// ==================================================================================
// DRIVER LOGIC
// ==================================================================================

#[cfg(target_os = "windows")]
const TOOL_URL: &str = "https://github.com/AOMediaCodec/libavif/releases/download/v1.1.1/avifenc-v1.1.1-windows.zip"; 
#[cfg(target_os = "windows")]
const TOOL_BINARY_NAME: &str = "avifenc.exe";

#[cfg(target_os = "linux")]
const TOOL_URL: &str = "https://github.com/AOMediaCodec/libavif/releases/download/v1.1.1/avifenc-v1.1.1-linux.zip";
#[cfg(target_os = "linux")]
const TOOL_BINARY_NAME: &str = "avifenc";

#[cfg(target_os = "macos")]
const TOOL_URL: &str = "https://github.com/AOMediaCodec/libavif/releases/download/v1.1.1/avifenc-v1.1.1-macos.zip";
#[cfg(target_os = "macos")]
const TOOL_BINARY_NAME: &str = "avifenc";

pub fn get_avifenc_path() -> Result<PathBuf, String> {
    let tool_dir = env::temp_dir().join("battle_cats_manager_tools");
    if !tool_dir.exists() {
        fs::create_dir_all(&tool_dir).map_err(|e| format!("Failed to create tool dir: {}", e))?;
    }
    let binary_path = tool_dir.join(TOOL_BINARY_NAME);
    if binary_path.exists() { return Ok(binary_path); }

    download_and_extract_tool(TOOL_URL, &tool_dir, TOOL_BINARY_NAME)?;

    if binary_path.exists() { Ok(binary_path) } else { Err("Download finished but binary was not found.".to_string()) }
}

fn download_and_extract_tool(url: &str, dest_dir: &PathBuf, binary_name: &str) -> Result<(), String> {
    let response = reqwest::blocking::get(url).map_err(|e| format!("Failed to download tool: {}", e))?;
    let bytes = response.bytes().map_err(|e| format!("Failed to read bytes: {}", e))?;
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Failed to open zip: {}", e))?;

    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if let Some(name) = file.enclosed_name() {
             if name.file_name().map(|n| n.to_str()).flatten() == Some(binary_name) {
                let out_path = dest_dir.join(binary_name);
                let mut out_file = fs::File::create(&out_path).map_err(|e| format!("Failed to create file: {}", e))?;
                io::copy(&mut file, &mut out_file).map_err(|e| format!("Failed to extract: {}", e))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(&out_path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&out_path, perms);
                    }
                }
                found = true;
                break;
             }
        }
    }
    if found { Ok(()) } else { Err(format!("Could not find '{}' in the zip.", binary_name)) }
}