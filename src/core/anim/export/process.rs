use eframe::egui;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::export::encoding::{self, ExportConfig, ExportFormat, EncoderMessage, EncoderStatus};
use crate::core::anim::export::state::ExporterState;
use crate::core::anim::{animator, smooth, transform}; 
use crate::core::anim::canvas::GlowRenderer;
use std::sync::{Arc, Mutex, mpsc};
use std::path::{PathBuf, Path};

// Global status receiver
pub static STATUS_RX: Mutex<Option<mpsc::Receiver<EncoderStatus>>> = Mutex::new(None);

pub fn start_export(state: &mut ExporterState) {
    if state.is_processing { return; }
    
    state.is_processing = true;
    state.current_progress = 0;
    state.encoded_frames = 0; // RESET
    state.completion_time = None; 
    
    // Calculate accurate Total Frame Count
    if state.showcase_mode {
        state.frame_start = 0;
        let total = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
        state.frame_end = if total > 0 { total - 1 } else { 0 }; 
    }

    // Name Generation Logic
    let (base_name, file_name) = if state.file_name.trim().is_empty() {
        let (disp_start, disp_end) = if state.showcase_mode {
             let total = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
             let end_disp = if total > 0 { total - 1 } else { 0 };
             (0, end_disp)
        } else {
             (state.frame_start, state.frame_end)
        };

        let range_part = if disp_start == disp_end { format!("{}f", disp_start) } else { format!("{}f~{}f", disp_start, disp_end) };
        let clean_prefix = state.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");
        
        let prefix_display = if state.showcase_mode {
             let p: Vec<&str> = clean_prefix.split('.').collect();
             if !p.is_empty() { format!("{}.showcase", p[0]) } else { "unit.showcase".to_string() }
        } else { clean_prefix.clone() };

        let base = if prefix_display.is_empty() { "animation".to_string() } else { prefix_display };
        let full = format!("{}.{}", base, range_part);
        
        (base, full)
    } else {
        // User custom name
        let path = Path::new(&state.file_name);
        let base = path.file_stem().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
        (base, state.file_name.clone())
    };

    let mut output_path = std::env::current_dir().unwrap_or(PathBuf::from("."));
    output_path.push("exports");
    
    // Non-destructive extension check
    let mut final_name = file_name;
    let frame_count = (state.frame_end - state.frame_start).abs() + 1;

    // Determine extension
    if let Some(ext) = match state.format {
        ExportFormat::Gif => Some("gif"), 
        ExportFormat::WebP => Some("webp"), 
        ExportFormat::Avif => Some("avif"), 
        ExportFormat::PngSequence => {
            if frame_count > 1 { Some("zip") } else { Some("png") }
        },
    } {
        if !final_name.to_lowercase().ends_with(&format!(".{}", ext)) { final_name = format!("{}.{}", final_name, ext); }
    }
    output_path.push(final_name);

    let config = ExportConfig {
        width: state.region_w as u32, height: state.region_h as u32,
        camera_x: state.region_x, camera_y: state.region_y, camera_zoom: state.zoom,
        format: state.format.clone(), quality: state.quality.clone(), fps: state.fps as u32,
        start_frame: state.frame_start, end_frame: state.frame_end, interpolation: state.interpolation,
        output_path,
        base_name, 
    };

    let (tx, rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel();
    
    if let Ok(mut lock) = STATUS_RX.lock() { *lock = Some(status_rx); }
    
    state.tx = Some(tx);
    encoding::start_encoding_thread(config, rx, status_tx);
}

pub fn process_frame(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut ExporterState,
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet,
    renderer_ref: Arc<Mutex<Option<GlowRenderer>>>,
    current_time: f32, 
) {
    if state.tx.is_none() { return; }

    let count = (state.frame_end - state.frame_start).abs() + 1;
    
    // Stop condition
    if state.current_progress >= count {
        if let Some(tx) = state.tx.take() { let _ = tx.send(EncoderMessage::Finish); }
        return;
    }

    let frame_delay = 1000.0 / state.fps as f32;
    
    let parts = if let Some(a) = anim {
        if state.interpolation { smooth::animate(model, a, current_time) } else { animator::animate(model, a, current_time) }
    } else {
        model.parts.clone()
    };
    
    let world_parts = transform::solve_hierarchy(&parts, model);
    let pan = egui::vec2(-state.region_x - (state.region_w as f32 / (2.0 * state.zoom)), -state.region_y - (state.region_h as f32 / (2.0 * state.zoom)));
    let bg_color = if state.format == ExportFormat::Gif { [50, 50, 50, 255] } else { [0, 0, 0, 0] };

    let renderer_arc = renderer_ref.clone();
    let sheet_arc = Arc::new(sheet.clone()); 
    let tx = if let Some(t) = state.tx.as_ref() { t.clone() } else { return };
    let (w, h, z) = (state.region_w, state.region_h, state.zoom);
    
    // Render Command
    ui.painter().add(egui::PaintCallback {
        rect, 
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, painter| {
            let mut lock = renderer_arc.lock().unwrap();
            if let Some(renderer) = lock.as_mut() {
                let img = encoding::render_frame(renderer, painter.gl(), w as u32, h as u32, &world_parts, &sheet_arc, pan, z, bg_color);
                let _ = tx.send(EncoderMessage::Frame(img, frame_delay as u32));
            }
        })),
    });

    state.current_progress += 1;
}