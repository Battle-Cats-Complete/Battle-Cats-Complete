use eframe::egui;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::export::encoding::{ExportFormat, QualityLevel, EncoderStatus};
use crate::core::anim::export::state::{ExporterState, ExportMode, LoopStatus};
use crate::core::anim::export::process::{start_export, STATUS_RX};
use crate::core::anim::export::findloop;
use crate::ui::views::settings::toggle_ui; 
use crate::core::anim::bounds;

pub fn show_popup(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    model: Option<&Model>,
    anim: Option<&Animation>,
    sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    // --- SETUP ---
    let attention_latch_id = egui::Id::new("export_needs_critical_attention");

    // --- EXPORT STATUS POLLING ---
    if state.is_processing {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        if let Ok(rx_opt) = STATUS_RX.lock() {
            if let Some(rx) = rx_opt.as_ref() {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        EncoderStatus::Encoding => { },
                        EncoderStatus::Progress(p) => { state.encoded_frames = p as i32; },
                        EncoderStatus::Finished => { 
                            state.is_processing = false; 
                            state.completion_time = Some(ui.input(|i| i.time));
                            ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, true));
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("export_done_seen"), false));
                        }
                    }
                }
            }
        }
    }

    // --- LOOP SEARCH STATUS POLLING ---
    let mut loop_finished = false;
    
    if state.is_loop_searching {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        
        if let Some(rx) = &state.loop_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    LoopStatus::Searching(n) => { state.loop_frames_searched = n; },
                    LoopStatus::Found(start, end) => {
                        state.frame_start = start;
                        state.frame_end = end;
                        state.frame_start_str = start.to_string();
                        state.frame_end_str = end.to_string();
                        loop_finished = true;
                    },
                    LoopStatus::NotFound => {
                        loop_finished = true;
                    },
                    LoopStatus::Error(_) => {
                        loop_finished = true;
                    }
                }
            }
        }
    }
    
    // Cleanup outside the borrow scope
    if loop_finished {
        state.is_loop_searching = false;
        state.loop_rx = None;
        state.loop_abort = None;
    }

    // --- LATCH EXECUTION ---
    let needs_attention = ui.ctx().data(|d| d.get_temp(attention_latch_id).unwrap_or(false));
    if needs_attention {
        if ui.input(|i| i.focused) {
            ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, false));
        } else {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
    }

    // --- UI RENDERING ---
    if !*is_open { return; }

    let ctx = ui.ctx().clone();
    let mut open_local = *is_open;
    let allow_drag = state.drag_guard.update(&ctx);

    let saved_style = ctx.style();
    let mut style = (*saved_style).clone();
    style.interaction.resize_grab_radius_side = 0.0;
    ctx.set_style(style);

    let window_id = egui::Id::new("Export Animation");
    let mut fixed_pos = None;

    if let Some(rect) = ctx.memory(|mem| mem.area_rect(window_id)) {
        let screen_rect = ctx.screen_rect();
        let mut new_pos = rect.min;
        let mut changed = false;
        if new_pos.y < screen_rect.top() { new_pos.y = screen_rect.top(); changed = true; }
        if new_pos.y > screen_rect.bottom() - 30.0 { new_pos.y = screen_rect.bottom() - 30.0; changed = true; }
        if new_pos.x + rect.width() - 50.0 < screen_rect.left() { new_pos.x = screen_rect.left() - rect.width() + 50.0; changed = true; }
        if new_pos.x + 50.0 > screen_rect.right() { new_pos.x = screen_rect.right() - 50.0; changed = true; }
        if changed { fixed_pos = Some(new_pos); }
    }

    let mut window = egui::Window::new("Export Animation")
        .id(window_id).open(&mut open_local).order(egui::Order::Foreground)
        .constrain(false).movable(allow_drag).collapsible(false).resizable(true)
        .min_size(egui::vec2(250.0, 300.0)).default_size(egui::vec2(400.0, 560.0))
        .default_pos(ctx.screen_rect().center() - egui::vec2(200.0, 260.0));

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
        
    window.show(&ctx, |ui| { render_content(ui, state, model, anim, sheet, is_open, start_region_selection); });
    
    ctx.set_style(saved_style);
    if !open_local { *is_open = false; }
}

fn render_content(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    model: Option<&Model>,
    anim: Option<&Animation>,
    sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    if state.anim_name.is_empty() {
        if let Some(a) = anim {
            if state.max_frame == 0 || state.max_frame == 100 {
                state.max_frame = a.max_frame;
            }
            if state.frame_end_str.is_empty() { state.frame_end = a.max_frame; }
        }
        state.anim_name = "Animation".to_string(); 
    }

    let bottom_height = 114.0; 
    let available_height = ui.available_height() - bottom_height;

    ui.add_enabled_ui(!state.is_processing, |ui| {
        egui::ScrollArea::vertical().max_height(available_height).auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(5.0);
            ui.heading("Input"); 
            ui.add_space(5.0);

            // Export Mode Dropdown
            ui.horizontal(|ui| {
                 ui.label("Export Mode:");
                 let mut mode = state.export_mode.clone();
                 egui::ComboBox::from_id_salt("ex_mode").selected_text(match mode {
                     ExportMode::Manual => "Manual",
                     ExportMode::Loop => "Loop",
                     ExportMode::Showcase => "Showcase",
                 }).show_ui(ui, |ui| {
                     ui.selectable_value(&mut mode, ExportMode::Manual, "Manual");
                     
                     if state.loop_supported {
                         ui.selectable_value(&mut mode, ExportMode::Loop, "Loop");
                     } else {
                         let r = ui.add_enabled(false, egui::SelectableLabel::new(false, "Loop"));
                         r.on_disabled_hover_text("Walk and Idle only");
                     }
                     
                     ui.selectable_value(&mut mode, ExportMode::Showcase, "Showcase");
                 });
                 if mode != state.export_mode {
                     // Mode Switch Logic
                     if mode == ExportMode::Showcase {
                         state.showcase_walk_str.clear();
                         state.showcase_idle_str.clear();
                         state.showcase_attack_str.clear();
                         state.showcase_kb_str.clear();
                         state.frame_start = 0;
                     }
                     if mode == ExportMode::Manual && state.export_mode == ExportMode::Loop {
                        state.frame_start = 0;
                        state.frame_end = 0;
                        state.frame_start_str.clear();
                        state.frame_end_str.clear();
                     }
                     
                     state.completion_time = None; 
                     state.current_progress = 0;
                     state.encoded_frames = 0;
                     state.export_mode = mode;
                 }
            });
            ui.add_space(5.0);

            match state.export_mode {
                ExportMode::Manual => {
                    ui.horizontal(|ui| {
                        ui.label("Frames");
                        let start_hint = egui::RichText::new("0").color(egui::Color32::GRAY);
                        let r1 = ui.add(egui::TextEdit::singleline(&mut state.frame_start_str).hint_text(start_hint).desired_width(40.0));
                        if state.frame_start_str.trim().is_empty() { state.frame_start = 0; } else if let Ok(val) = state.frame_start_str.trim().parse::<i32>() { state.frame_start = val; }
                        
                        ui.label("to");
                        let hint_val = anim.map_or(0, |a| a.max_frame);
                        let end_hint = egui::RichText::new(hint_val.to_string()).color(egui::Color32::GRAY);
                        let r2 = ui.add(egui::TextEdit::singleline(&mut state.frame_end_str).hint_text(end_hint).desired_width(40.0));
                        if state.frame_end_str.trim().is_empty() { state.frame_end = hint_val; } else if let Ok(val) = state.frame_end_str.trim().parse::<i32>() { state.frame_end = val; }
    
                        if r1.changed() || r2.changed() {
                            state.completion_time = None;
                            state.current_progress = 0;
                            state.encoded_frames = 0;
                        }
                    });
                },
                ExportMode::Loop => {
                    ui.horizontal(|ui| {
                        ui.label("Loop Tolerance");
                        let hint = egui::RichText::new("30").color(egui::Color32::GRAY);
                        if ui.add(egui::TextEdit::singleline(&mut state.loop_tolerance_str).hint_text(hint).desired_width(40.0)).changed() {
                            if let Ok(v) = state.loop_tolerance_str.parse::<i32>() { state.loop_tolerance = v; }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Loop Minimum");
                        let hint = egui::RichText::new("15").color(egui::Color32::GRAY);
                        if ui.add(egui::TextEdit::singleline(&mut state.loop_min_str).hint_text(hint).desired_width(40.0)).changed() {
                             if let Ok(v) = state.loop_min_str.parse::<i32>() { state.loop_min = v; }
                        }
                    });

                    // Locked Frames Fields
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.add_enabled(false, egui::Label::new("Frames"));
                        let mut start = state.frame_start.to_string();
                        let mut end = state.frame_end.to_string();
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut start).desired_width(40.0));
                        ui.add_enabled(false, egui::Label::new("to"));
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut end).desired_width(40.0));
                    });
                    
                    ui.add_space(5.0);
                    if state.is_loop_searching {
                         let btn = egui::Button::new("Abort Loop").fill(egui::Color32::from_rgb(180, 50, 50));
                         if ui.add_sized(egui::vec2(ui.available_width(), 24.0), btn).clicked() {
                             if let Some(flag) = &state.loop_abort {
                                 flag.store(true, Ordering::Relaxed);
                             }
                         }
                    } else {
                        if ui.add_sized(egui::vec2(ui.available_width(), 24.0), egui::Button::new("Find Loop")).clicked() {
                             if let (Some(m), Some(a)) = (model, anim) {
                                 let use_tol = if state.loop_tolerance_str.is_empty() { 30 } else { state.loop_tolerance_str.parse().unwrap_or(30) };
                                 let use_min = if state.loop_min_str.is_empty() { 15 } else { state.loop_min_str.parse().unwrap_or(15) };
                                 
                                 state.loop_tolerance = use_tol;
                                 state.loop_min = use_min;

                                 let (tx, rx) = std::sync::mpsc::channel();
                                 state.loop_rx = Some(rx);
                                 state.is_loop_searching = true;
                                 state.loop_frames_searched = 0;
                                 let abort = Arc::new(AtomicBool::new(false));
                                 state.loop_abort = Some(abort.clone());
                                 
                                 findloop::start_search(m.clone(), a.clone(), use_tol, use_min, tx, abort);
                             }
                        }
                    }
                },
                ExportMode::Showcase => {
                    let hint_90 = egui::RichText::new("90").color(egui::Color32::GRAY);
                    egui::Grid::new("showcase_grid").spacing([10.0, 4.0]).show(ui, |ui| {
                        ui.label("Walk Frames");
                        if ui.add(egui::TextEdit::singleline(&mut state.showcase_walk_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                            state.showcase_walk_len = state.showcase_walk_str.trim().parse().unwrap_or(if state.showcase_walk_str.trim().is_empty() { 90 } else { 0 });
                            state.completion_time = None;
                        }
                        if state.showcase_walk_str.trim().is_empty() { state.showcase_walk_len = 90; }
                        ui.end_row();
    
                        ui.label("Idle Frames");
                        if ui.add(egui::TextEdit::singleline(&mut state.showcase_idle_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                            state.showcase_idle_len = state.showcase_idle_str.trim().parse().unwrap_or(if state.showcase_idle_str.trim().is_empty() { 90 } else { 0 });
                            state.completion_time = None;
                        }
                        if state.showcase_idle_str.trim().is_empty() { state.showcase_idle_len = 90; }
                        ui.end_row();
    
                        ui.label("Attack Frames");
                        let hint_atk = egui::RichText::new(state.detected_attack_len.to_string()).color(egui::Color32::GRAY);
                        if ui.add(egui::TextEdit::singleline(&mut state.showcase_attack_str).hint_text(hint_atk).desired_width(50.0)).changed() {
                            state.showcase_attack_len = state.showcase_attack_str.trim().parse().unwrap_or(if state.showcase_attack_str.trim().is_empty() { state.detected_attack_len } else { 0 });
                            state.completion_time = None;
                        }
                        if state.showcase_attack_str.trim().is_empty() { state.showcase_attack_len = state.detected_attack_len; }
                        ui.end_row();
    
                        ui.label("Knockback");
                        if ui.add(egui::TextEdit::singleline(&mut state.showcase_kb_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                            state.showcase_kb_len = state.showcase_kb_str.trim().parse().unwrap_or(if state.showcase_kb_str.trim().is_empty() { 90 } else { 0 });
                            state.completion_time = None;
                        }
                        if state.showcase_kb_str.trim().is_empty() { state.showcase_kb_len = 90; }
                        ui.end_row();
                    });
                }
            }

            ui.add_space(20.0);
            ui.heading("Camera"); 
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Set Camera").on_hover_text("Right-click and drag on the viewport to select area").clicked() { *start_region_selection = true; *is_open = false; }
                
                if ui.button("Use Bounds").on_hover_text("Auto-calculate camera from unit size").clicked() { 
                    let mut calculated = false;
                    if let (Some(m), Some(s)) = (model, sheet) {
                        if let Some(bounds) = bounds::calculate_tight_bounds(m, anim, s) {
                            state.region_x = bounds.min.x;
                            state.region_y = bounds.min.y;
                            state.region_w = bounds.width();
                            state.region_h = bounds.height();
                            state.zoom = 1.0;
                            calculated = true;
                        }
                    }

                    if !calculated {
                        state.region_x = -150.0; 
                        state.region_y = -150.0; 
                        state.region_w = 300.0; 
                        state.region_h = 300.0; 
                        state.zoom = 1.0; 
                    }
                }
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label("X"); ui.add(egui::DragValue::new(&mut state.region_x).speed(1.0));
                ui.add_space(10.0);
                ui.label("Y"); ui.add(egui::DragValue::new(&mut state.region_y).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label("W"); ui.add(egui::DragValue::new(&mut state.region_w).range(0.0..=10000.0).speed(1.0));
                ui.add_space(8.0);
                ui.label("H"); ui.add(egui::DragValue::new(&mut state.region_h).range(0.0..=10000.0).speed(1.0));
            });

            ui.add_space(20.0);
            ui.heading("Output"); 
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Name");
                let (disp_start, disp_end) = if state.export_mode == ExportMode::Showcase {
                     let total = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
                     let end_disp = if total > 0 { total - 1 } else { 0 };
                     (0, end_disp)
                } else { (state.frame_start, state.frame_end) };

                let range_part = if disp_start == disp_end { format!("{}f", disp_start) } else { format!("{}f~{}f", disp_start, disp_end) };
                let clean_prefix = state.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");
                let prefix_display = if state.export_mode == ExportMode::Showcase {
                     let p: Vec<&str> = clean_prefix.split('.').collect();
                     if !p.is_empty() { format!("{}.showcase", p[0]) } else { "unit.showcase".to_string() }
                } else { clean_prefix.clone() };

                let hint_str = if prefix_display.is_empty() { "animation".to_string() } else { format!("{}.{}", prefix_display, range_part) };
                ui.add(egui::TextEdit::singleline(&mut state.file_name).hint_text(egui::RichText::new(&hint_str).color(egui::Color32::GRAY)).desired_width(120.0));
            });

            egui::Grid::new("out_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label("Format");
                    egui::ComboBox::from_id_salt("fmt_combo").selected_text(match state.format {
                            ExportFormat::Gif => "GIF", ExportFormat::WebP => "WebP (Animated)", ExportFormat::Avif => "AVIF (Animated)", ExportFormat::PngSequence => "PNG Sequence",
                        }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.format, ExportFormat::Gif, "GIF");
                            ui.selectable_value(&mut state.format, ExportFormat::WebP, "WebP (Animated)");
                            ui.selectable_value(&mut state.format, ExportFormat::Avif, "AVIF (Animated)");
                            ui.selectable_value(&mut state.format, ExportFormat::PngSequence, "PNG Sequence");
                        });
                    ui.end_row();
                    ui.label("Quality");
                    egui::ComboBox::from_id_salt("qual_combo").selected_text(format!("{:?}", state.quality)).show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.quality, QualityLevel::Low, "Low");
                            ui.selectable_value(&mut state.quality, QualityLevel::Medium, "Medium");
                            ui.selectable_value(&mut state.quality, QualityLevel::High, "High");
                        });
                    ui.end_row();
            });
            
            ui.horizontal(|ui| { toggle_ui(ui, &mut state.interpolation); ui.label("Interpolation"); });
            ui.add_space(20.0); ui.heading("OPET"); ui.add_space(5.0); ui.label("Optional Performance Enhancing Tools"); ui.add_space(5.0);
        });
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(5.0); 
        ui.add_enabled_ui(!state.is_processing && !state.is_loop_searching, |ui| {
            let is_valid = state.region_w > 0.1 && state.region_h > 0.1;
            let btn_text = if is_valid { "Begin Export" } else { "No Camera Set" };
            
            if ui.add_enabled_ui(is_valid, |ui| {
                ui.add_sized(egui::vec2(ui.available_width(), 30.0), egui::Button::new(btn_text))
            }).inner.clicked() { 
                start_export(state); 
            }
        });
        ui.add_space(5.0);
        
        let count = (state.frame_end - state.frame_start).abs() + 1;
        let (progress_val, label_text) = if state.is_loop_searching {
            // Hijacked Status
            let p_anim = (ui.input(|i| i.time) % 1.0) as f32; // Indeterminate pulse
            (p_anim, format!("Searching | {} frames", state.loop_frames_searched))
        } else if state.is_processing {
            if state.current_progress < count {
                let ratio = if count == 0 { 0.0 } else { (state.current_progress as f32 / count as f32).min(1.0) };
                let percent = (ratio * 100.0) as i32;
                (ratio, format!("Rendering | {}f/{}f ({}%)", state.current_progress, count, percent))
            } else {
                let ratio = if count == 0 { 0.0 } else { (state.encoded_frames as f32 / count as f32).min(1.0) };
                let percent = (ratio * 100.0) as i32;
                (ratio, format!("Encoding | {}f/{}f ({}%)", state.encoded_frames, count, percent))
            }
        } else {
            match state.completion_time {
                Some(done_time) => {
                    let is_focused = ui.input(|i| i.focused);
                    let seen_id = egui::Id::new("export_done_seen");
                    let mut has_seen = ui.ctx().data(|d| d.get_temp(seen_id).unwrap_or(false));

                    if is_focused && !has_seen {
                        has_seen = true;
                        ui.ctx().data_mut(|d| d.insert_temp(seen_id, true));
                    }

                    if !has_seen && !is_focused {
                        state.completion_time = Some(ui.input(|i| i.time));
                        ui.ctx().request_repaint(); 
                        (1.0, "Done".to_string())
                    } else {
                        let elapsed = ui.input(|i| i.time) - done_time;
                        if elapsed < 3.0 { 
                            ui.ctx().request_repaint(); 
                            (1.0, "Done".to_string()) 
                        } 
                        else { 
                            state.completion_time = None; 
                            (1.0, "Ready".to_string()) 
                        }
                    }
                },
                None => {
                    let ratio = if count == 0 { 0.0 } else { (state.current_progress as f32 / count as f32).min(1.0) };
                    if ratio > 0.0 && ratio < 1.0 { 
                         let percent = (ratio * 100.0) as i32;
                        (ratio, format!("Paused | {}f/{}f ({}%)", state.current_progress, count, percent)) 
                    } else { 
                        (1.0, "Ready".to_string()) 
                    }
                }
            }
        };

        ui.label(label_text);
        ui.add(egui::ProgressBar::new(progress_val));
        
        ui.add_space(5.0); ui.separator(); 
    });
}