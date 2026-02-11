pub mod state;

use eframe::egui;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::export::{self, ExportConfig, ExportFormat, QualityLevel, EncoderMessage, EncoderStatus};
use crate::core::anim::{animator, smooth, transform}; 
use crate::core::anim::canvas::GlowRenderer;
use std::sync::{Arc, Mutex, mpsc};
use std::path::PathBuf;
use crate::ui::views::settings::toggle_ui;

use self::state::ExporterState;

static STATUS_RX: Mutex<Option<mpsc::Receiver<EncoderStatus>>> = Mutex::new(None);

pub fn show_popup(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    model: Option<&Model>,
    anim: Option<&Animation>,
    sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    if !*is_open { return; }

    let ctx = ui.ctx().clone();
    let mut open_local = *is_open;
    let allow_drag = state.drag_guard.update(&ctx);

    egui::Window::new("Export Animation")
        .open(&mut open_local)
        .order(egui::Order::Foreground) 
        .constrain(true)             
        .movable(allow_drag)         
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .fixed_size(egui::vec2(400.0, 520.0))
        .show(&ctx, |ui| {
            render_content(ui, state, model, anim, sheet, is_open, start_region_selection);
        });
    
    if !open_local {
        *is_open = false;
    }
}

fn render_content(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    _model: Option<&Model>,
    anim: Option<&Animation>,
    _sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    if state.anim_name.is_empty() {
        if let Some(a) = anim {
            let full_length = a.calculate_true_loop().unwrap_or(a.max_frame);
            state.max_frame = full_length;
            state.frame_end = full_length;
        }
        state.anim_name = "Animation".to_string(); 
    }
    
    // Check for encoder status
    if state.is_processing {
        if let Ok(rx_opt) = STATUS_RX.lock() {
            if let Some(rx) = rx_opt.as_ref() {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        EncoderStatus::Encoding => { },
                        EncoderStatus::Finished => {
                            state.is_processing = false;
                            // FIXED: Capture completion time for the 5s timer
                            state.completion_time = Some(ui.input(|i| i.time));
                        }
                    }
                }
            }
        }
    }

    let bottom_height = 90.0; 
    let available_height = ui.available_height() - bottom_height;

    ui.add_enabled_ui(!state.is_processing, |ui| {
        egui::ScrollArea::vertical()
            .max_height(available_height)
            .auto_shrink([false, false]) 
            .show(ui, |ui| {
            
            ui.add_space(5.0);
            ui.heading("Input"); 
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Frames");
                ui.add(egui::DragValue::new(&mut state.frame_start).range(0..=state.max_frame));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut state.frame_end).range(0..=state.max_frame));
            });

            ui.add_space(20.0);
            ui.heading("Camera"); 
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Set Region").on_hover_text("Right-click and drag on the viewport to select area").clicked() {
                    *start_region_selection = true;
                    *is_open = false; 
                }
                
                if ui.button("Reset").clicked() {
                    state.region_x = -150.0;
                    state.region_y = -150.0;
                    state.region_w = 300.0;
                    state.region_h = 300.0;
                    state.zoom = 1.0;
                }
            });
            
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label("X"); 
                ui.add(egui::DragValue::new(&mut state.region_x).speed(1.0));
                
                ui.add_space(10.0);
                
                ui.label("Y"); 
                ui.add(egui::DragValue::new(&mut state.region_y).speed(1.0));
            });
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label("W"); 
                ui.add(egui::DragValue::new(&mut state.region_w).range(1.0..=2000.0).speed(1.0));
                
                ui.add_space(8.0);
                
                ui.label("H"); 
                ui.add(egui::DragValue::new(&mut state.region_h).range(1.0..=2000.0).speed(1.0));
            });

            ui.add_space(20.0);
            ui.heading("Output"); 
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Name");
                ui.add(egui::TextEdit::singleline(&mut state.file_name).desired_width(120.0));
            });

            egui::Grid::new("out_grid")
                .num_columns(2)
                .spacing([10.0, 8.0]) 
                .show(ui, |ui| {
                    ui.label("Format");
                    egui::ComboBox::from_id_salt("fmt_combo")
                        .selected_text(match state.format {
                            ExportFormat::Gif => "GIF",
                            ExportFormat::WebP => "WebP (Animated)",
                            ExportFormat::Avif => "AVIF (Animated)",
                            ExportFormat::PngSequence => "PNG Sequence",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.format, ExportFormat::Gif, "GIF");
                            ui.selectable_value(&mut state.format, ExportFormat::WebP, "WebP (Animated)");
                            ui.selectable_value(&mut state.format, ExportFormat::Avif, "AVIF (Animated)");
                            ui.selectable_value(&mut state.format, ExportFormat::PngSequence, "PNG Sequence");
                        });
                    ui.end_row();

                    ui.label("Quality");
                    egui::ComboBox::from_id_salt("qual_combo")
                        .selected_text(format!("{:?}", state.quality))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.quality, QualityLevel::Low, "Low");
                            ui.selectable_value(&mut state.quality, QualityLevel::Medium, "Medium");
                            ui.selectable_value(&mut state.quality, QualityLevel::High, "High");
                        });
                    ui.end_row();
                });
            
            ui.horizontal(|ui| {
                toggle_ui(ui, &mut state.interpolation);
                ui.label("Interpolation");
            });

            ui.add_space(20.0);
            ui.heading("OPET");
            ui.add_space(5.0);
            ui.label("Optional Performance Enhancing Tools");
            ui.add_space(5.0);
        });
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(5.0); 

        ui.add_enabled_ui(!state.is_processing, |ui| {
            if ui.add_sized(egui::vec2(ui.available_width(), 30.0), egui::Button::new("Begin Export")).clicked() {
                start_export(state);
            }
        });

        ui.add_space(5.0);

        // Progress Bar Logic with "Done" Timer and "Ready at 100%"
        let count = (state.frame_end - state.frame_start).abs() + 1;
        let ratio = if count == 0 { 0.0 } else { state.current_progress as f32 / count as f32 };

        let (progress_val, label_text) = if state.is_processing {
            // Encoding Phase
            (ratio, format!("Rendering... {:.0}% ({}/{})", ratio * 100.0, state.current_progress, count))
        } else {
            match state.completion_time {
                Some(done_time) => {
                    let elapsed = ui.input(|i| i.time) - done_time;
                    if elapsed < 5.0 {
                        ui.ctx().request_repaint(); // Refresh to ensure timer works
                        (1.0, "Done".to_string())
                    } else {
                        state.completion_time = None; // Reset after 5s
                        (1.0, "Ready".to_string())
                    }
                },
                None => {
                    // Default state (or Paused)
                    if ratio > 0.0 && ratio < 1.0 {
                         (ratio, "Paused".to_string())
                    } else {
                         (1.0, "Ready".to_string()) // Default start at 100%
                    }
                }
            }
        };
        
        ui.add(egui::ProgressBar::new(progress_val).text(label_text).animate(state.is_processing));

        ui.add_space(5.0);
        ui.separator(); 
    });
}

fn start_export(state: &mut ExporterState) {
    if state.is_processing { return; }
    
    state.is_processing = true;
    state.current_progress = 0;
    state.completion_time = None; // Reset timer
    
    let mut output_path = std::env::current_dir().unwrap_or(PathBuf::from("."));
    output_path.push("exports");
    output_path.push(&state.file_name);
    
    if let Some(ext) = match state.format {
        ExportFormat::Gif => Some("gif"),
        ExportFormat::WebP => Some("webp"),
        ExportFormat::Avif => Some("avif"),
        ExportFormat::PngSequence => Some("png"),
    } {
            if state.format != ExportFormat::PngSequence {
                output_path.set_extension(ext);
            }
    }

    let config = ExportConfig {
        width: state.region_w as u32,
        height: state.region_h as u32,
        camera_x: state.region_x,
        camera_y: state.region_y,
        camera_zoom: state.zoom,
        format: state.format.clone(),
        quality: state.quality.clone(),
        fps: state.fps as u32,
        start_frame: state.frame_start,
        end_frame: state.frame_end,
        interpolation: state.interpolation,
        output_path, 
    };

    let (tx, rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel();
    
    if let Ok(mut lock) = STATUS_RX.lock() {
        *lock = Some(status_rx);
    }

    state.tx = Some(tx);
    
    export::start_encoding_thread(config, rx, status_tx);
}

pub fn process_frame(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut ExporterState,
    model: &Model,
    anim: &Animation,
    sheet: &SpriteSheet,
    renderer_ref: Arc<Mutex<Option<GlowRenderer>>>,
) {
    if state.tx.is_none() { return; }

    let count = (state.frame_end - state.frame_start).abs() + 1;
    
    if state.current_progress >= count {
        if let Some(tx) = state.tx.take() {
            let _ = tx.send(EncoderMessage::Finish);
        }
        return;
    }

    let start = state.frame_start;
    let end = state.frame_end;
    let step = if start < end { 1 } else { -1 };
    
    let f = start + (state.current_progress * step);
    let frame_delay = 1000.0 / state.fps as f32;

    let parts = if state.interpolation {
        smooth::animate(model, anim, f as f32)
    } else {
        animator::animate(model, anim, f as f32)
    };
    
    let world_parts = transform::solve_hierarchy(&parts, model);

    let pan_x = -state.region_x - (state.region_w as f32 / (2.0 * state.zoom));
    let pan_y = -state.region_y - (state.region_h as f32 / (2.0 * state.zoom));
    let pan = egui::vec2(pan_x, pan_y);

    let bg_color = if state.format == ExportFormat::Gif {
        [50, 50, 50, 255]
    } else {
        [0, 0, 0, 0]
    };

    let renderer_arc = renderer_ref.clone();
    let sheet_arc = Arc::new(sheet.clone()); 
    let tx = if let Some(t) = state.tx.as_ref() { t.clone() } else { return };
    
    let w = state.region_w;
    let h = state.region_h;
    let z = state.zoom;
    
    ui.painter().add(egui::PaintCallback {
        rect, 
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, painter| {
            let mut lock = renderer_arc.lock().unwrap();
            if let Some(renderer) = lock.as_mut() {
                let img = export::render_frame(
                    renderer,
                    painter.gl(), 
                    w as u32, h as u32, 
                    &world_parts, 
                    &sheet_arc, 
                    pan,
                    z,
                    bg_color
                );
                
                let _ = tx.send(EncoderMessage::Frame(img, frame_delay as u32));
            }
        })),
    });

    state.current_progress += 1;
}