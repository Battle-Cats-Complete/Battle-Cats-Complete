/*
type: uploaded file
fileName: anim_controls.rs
*/
use eframe::egui;
use std::path::PathBuf;
use crate::ui::components::anim_viewer::AnimViewer;

// --- LAYOUT CONSTANTS ---
const TILE_HEIGHT: f32 = 28.0; 
const GAP: f32 = 4.0;
const OVERLAY_BOTTOM_OFFSET: f32 = 35.0; 

// This is the constant you asked for!
// It defines how far the controls slide down when minimized.
pub const CONTROLS_SLIDE_DISTANCE: f32 = 180.0;

// Column 1: Play/Orient
const ICON_W: f32 = 60.0;

// Column 2: Frame Controls 
const COL2_W: f32 = 148.0; 
const NAV_W: f32 = 30.0;
const INPUT_W: f32 = 80.0; 

// Column 3: Export/Speed
const COL3_W: f32 = 100.0;

// Animation Indices
pub const IDX_WALK: usize = 0;
pub const IDX_IDLE: usize = 1;
pub const IDX_ATTACK: usize = 2;
pub const IDX_KB: usize = 3;
pub const IDX_SPIRIT: usize = 4;
pub const IDX_BURROW: usize = 5;
pub const IDX_SURFACE: usize = 6;
pub const IDX_MODEL: usize = 99;

pub fn render_controls_overlay(
    ui: &mut egui::Ui,
    rect: egui::Rect, 
    anim_viewer: &mut AnimViewer,
    available_anims: &Vec<(usize, &str, PathBuf)>,
    spirit_available: bool,
    base_assets_available: bool,
    is_loading_new: bool,
    spirit_sheet_id: &str,
    form_viewer_id: &str,
    spirit_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool, 
    native_fps: f32, // NEW PARAM
) {
    let mut clip_rect = rect;
    clip_rect = clip_rect.shrink(4.0); 
    ui.set_clip_rect(clip_rect);

    // SLIDE ANIMATION LOGIC
    let target_slide = if anim_viewer.is_controls_expanded { 0.0 } else { 1.0 };
    
    let anim_id = egui::Id::new("controls_slide").with(&anim_viewer.loaded_id);
    let slide_factor = ui.ctx().animate_value_with_time(anim_id, target_slide, 0.35);
    
    let current_offset = CONTROLS_SLIDE_DISTANCE * slide_factor;
    let bottom_margin = 5.0 + OVERLAY_BOTTOM_OFFSET - current_offset;

    // ROOT BUILDER: Bottom-Up
    let builder = egui::UiBuilder::new()
        .max_rect(clip_rect)
        .layout(egui::Layout::bottom_up(egui::Align::Min));
    
    ui.allocate_new_ui(builder, |ui| {
        egui::Frame::window(ui.style())
            .fill(egui::Color32::from_black_alpha(160)) 
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 8.0, bottom: 18.0 })
            .outer_margin(egui::Margin { left: 3.0, bottom: bottom_margin, ..Default::default() })
            .rounding(8.0)
            .show(ui, |ui| {
                
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    
                    // 1. Main Controls (Bottom)
                    render_internal_ui(
                        ui, 
                        anim_viewer, 
                        available_anims, 
                        spirit_available, 
                        base_assets_available, 
                        is_loading_new,
                        spirit_sheet_id,
                        form_viewer_id,
                        spirit_pack,
                        interpolation,
                        native_fps
                    );

                    let width_to_use = if anim_viewer.cached_controls_width > 1.0 {
                        anim_viewer.cached_controls_width
                    } else {
                        ui.available_width()
                    };

                    // 2. Separator (Middle)
                    ui.add_sized(egui::vec2(width_to_use, 1.0), egui::Separator::default().horizontal());
                    
                    // 3. Toggle Button (Top)
                    let icon = if anim_viewer.is_controls_expanded { "▼" } else { "▲" };
                    let btn = egui::Button::new(egui::RichText::new(icon).strong().size(14.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE);

                    if ui.add_sized(egui::vec2(width_to_use, 18.0), btn).clicked() {
                        anim_viewer.is_controls_expanded = !anim_viewer.is_controls_expanded;
                    }
                });
            });
    });
}

fn render_internal_ui(
    ui: &mut egui::Ui,
    anim_viewer: &mut AnimViewer,
    available_anims: &Vec<(usize, &str, PathBuf)>,
    spirit_available: bool,
    base_assets_available: bool,
    is_loading_new: bool,
    spirit_sheet_id: &str,
    form_viewer_id: &str,
    spirit_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool,
    native_fps: f32,
) {
    let mut clicked_index: Option<usize> = None;
    let active_color = egui::Color32::from_rgb(31, 106, 165);
    let inactive_color = egui::Color32::from_gray(60);
    
    let btn_w = 70.0;
    let grid_gap = 5.0;
    let btn_size = egui::vec2(btn_w, 25.0);

    // =========================================================
    // 1. DISPLAY LOGIC (Scaling & Capping)
    // =========================================================

    // Calculate display multiplier (Game Frame -> Real Frame)
    let display_multiplier = if interpolation {
        native_fps / 30.0
    } else {
        1.0
    };

    // State Snapshot
    let (lcm_result, max_frame_val) = if let Some(anim) = &anim_viewer.current_anim {
        if anim_viewer.loaded_anim_index <= 1 { 
            let res = anim.calculate_true_loop();
            (res, res.unwrap_or(0)) 
        } else { 
            (Some(anim.max_frame), anim.max_frame) 
        }
    } else { 
        (Some(0), 0) 
    };
    
    // CAP CHECK: If value > 999,999, show "???"
    let display_max_str = match lcm_result {
        Some(v) if v > 999_999 => "???".to_string(), 
        Some(v) => ((v as f32 * display_multiplier).round() as i32).to_string(),
        None => "???".to_string()
    };

    let cur_frame_val = anim_viewer.current_frame;
    let loop_range_0 = anim_viewer.loop_range.0;
    let loop_range_1 = anim_viewer.loop_range.1;
    let is_playing = anim_viewer.is_playing;
    let is_model_mode = anim_viewer.loaded_anim_index == IDX_MODEL;
    
    // Scale current frame
    let cur_display_val = (cur_frame_val * display_multiplier).round() as i32;
    
    let effective_display_max = if is_model_mode {
        "0".to_string()
    } else if let Some(override_end) = loop_range_1 {
        ((override_end as f32 * display_multiplier).round() as i32).to_string()
    } else {
        display_max_str.clone()
    };
    
    // Tile Styling
    let tile_frame = egui::Frame::none()
        .fill(egui::Color32::from_gray(40))
        .rounding(4.0)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .inner_margin(0.0);

    // --- RENDER BLOCK ---
    let controls_response = ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = GAP;

        // COLUMN 1: Play / Orient
        ui.vertical(|ui| {
            let play_icon = if is_playing { "⏸" } else { "▶" };
            if ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new(egui::RichText::new(play_icon).size(16.0))).clicked() {
                anim_viewer.is_playing = !anim_viewer.is_playing;
            }
            
            ui.add_space(GAP);
            
            if ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new("Orient")).clicked() { 
                anim_viewer.pan_offset = egui::Vec2::ZERO; 
            }
        });

        // Separator
        let sep_height = (TILE_HEIGHT * 2.0) + GAP;
        ui.add_sized(egui::vec2(10.0, sep_height), egui::Separator::default().vertical());

        // COLUMN 2: Frame Controls / Info
        ui.vertical(|ui| {
            // Row 1: Nav
            ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    
                    if !is_playing {
                        // [◀]
                        let left_btn = ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("◀").sense(egui::Sense::click().union(egui::Sense::drag())));
                        
                        // [F] (Single Frame Input)
                        tile_frame.show(ui, |ui| {
                            ui.set_width(INPUT_W);
                            ui.set_height(TILE_HEIGHT);
                            
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                
                                let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.single_frame_str)
                                    .frame(false)
                                    .desired_width(INPUT_W)
                                    .vertical_align(egui::Align::Center)
                                    .horizontal_align(egui::Align::Center));
                                
                                if re.changed() {
                                    if let Ok(val) = anim_viewer.single_frame_str.parse::<i32>() {
                                        // Input is in displayed (potentially scaled) frames
                                        // Convert back to game frames (30 base)
                                        anim_viewer.current_frame = val as f32 / display_multiplier;
                                    }
                                }
                                if !re.has_focus() {
                                    anim_viewer.single_frame_str = format!("{}", cur_display_val);
                                }
                            });
                        });

                        // [▶]
                        let right_btn = ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("▶").sense(egui::Sense::click().union(egui::Sense::drag())));

                        // Hold Logic
                        if left_btn.is_pointer_button_down_on() { anim_viewer.hold_dir = -1; } 
                        else if right_btn.is_pointer_button_down_on() { anim_viewer.hold_dir = 1; } 
                        else { anim_viewer.hold_dir = 0; }
                        
                        // Step by 1 Game Frame
                        if left_btn.clicked() {
                            let f = cur_frame_val - 1.0;
                            let wrap_target = if lcm_result.is_some() { max_frame_val as f32 } else { 0.0 }; 
                            anim_viewer.current_frame = if f < 0.0 { wrap_target } else { f };
                        }
                        if right_btn.clicked() {
                            let f = cur_frame_val + 1.0;
                            if let Some(mx) = lcm_result {
                                anim_viewer.current_frame = if f > mx as f32 { 0.0 } else { f };
                            } else {
                                anim_viewer.current_frame = f;
                            }
                        }

                    } else {
                        // [F] ~ [F] (Range Mode)
                        
                        // Start Bound
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0);
                            ui.set_height(TILE_HEIGHT);
                            
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                
                                if loop_range_0.is_some() && anim_viewer.range_str_cache.0.is_empty() {
                                    // Scale for display
                                    let v = (loop_range_0.unwrap() as f32 * display_multiplier).round() as i32;
                                    anim_viewer.range_str_cache.0 = v.to_string();
                                }
                                
                                let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.0)
                                    .hint_text(egui::RichText::new("0").color(egui::Color32::GRAY))
                                    .frame(false)
                                    .desired_width(60.0)
                                    .vertical_align(egui::Align::Center)
                                    .horizontal_align(egui::Align::Center));
                                    
                                if re.changed() {
                                    if anim_viewer.range_str_cache.0.is_empty() {
                                        anim_viewer.loop_range.0 = None;
                                    } else if let Ok(val) = anim_viewer.range_str_cache.0.parse::<i32>() {
                                        // Unscale input back to game frames
                                        let val_raw = (val as f32 / display_multiplier).round() as i32;
                                        anim_viewer.loop_range.0 = Some(val_raw);
                                        if cur_frame_val < val_raw as f32 { anim_viewer.current_frame = val_raw as f32; }
                                    }
                                }
                                if re.secondary_clicked() {
                                    anim_viewer.loop_range.0 = None;
                                    anim_viewer.range_str_cache.0.clear();
                                }
                            });
                        });
                        
                        // Separator
                        tile_frame.show(ui, |ui| {
                            ui.set_width(20.0);
                            ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label("~"); 
                            });
                        });
                        
                        // End Bound
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0);
                            ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                
                                if loop_range_1.is_some() && anim_viewer.range_str_cache.1.is_empty() {
                                    // Scale for display
                                    let v = (loop_range_1.unwrap() as f32 * display_multiplier).round() as i32;
                                    anim_viewer.range_str_cache.1 = v.to_string();
                                }
                                
                                let hint = egui::RichText::new(&display_max_str).color(egui::Color32::GRAY);
                                let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.1)
                                    .hint_text(hint)
                                    .frame(false)
                                    .desired_width(60.0)
                                    .vertical_align(egui::Align::Center)
                                    .horizontal_align(egui::Align::Center));
                                    
                                if re.changed() {
                                    if anim_viewer.range_str_cache.1.is_empty() {
                                        anim_viewer.loop_range.1 = None;
                                    } else if let Ok(val) = anim_viewer.range_str_cache.1.parse::<i32>() {
                                        // Unscale input
                                        let val_raw = (val as f32 / display_multiplier).round() as i32;
                                        anim_viewer.loop_range.1 = Some(val_raw);
                                        let start = loop_range_0.unwrap_or(0);
                                        if cur_frame_val > val_raw as f32 { anim_viewer.current_frame = start as f32; }
                                    }
                                }
                                if re.secondary_clicked() {
                                    anim_viewer.loop_range.1 = None;
                                    anim_viewer.range_str_cache.1.clear();
                                }
                            });
                        });
                    }
                });
            });

            ui.add_space(GAP);

            // Row 2: Info (Cur / Max)
            ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    
                    tile_frame.show(ui, |ui| {
                        ui.set_width(60.0);
                        ui.set_height(TILE_HEIGHT);
                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(egui::RichText::new(format!("{}", cur_display_val)).color(egui::Color32::WHITE)); 
                        });
                    });
                    tile_frame.show(ui, |ui| {
                        ui.set_width(20.0);
                        ui.set_height(TILE_HEIGHT);
                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label("/"); 
                        });
                    });
                    tile_frame.show(ui, |ui| {
                        ui.set_width(60.0);
                        ui.set_height(TILE_HEIGHT);
                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(egui::RichText::new(&effective_display_max).color(egui::Color32::WHITE)); 
                        });
                    });
                });
            });
        });

        // Separator
        ui.add_sized(egui::vec2(10.0, sep_height), egui::Separator::default().vertical());

        // COLUMN 3: Export / Speed
        ui.vertical(|ui| {
            // Export Button
            let id = ui.make_persistent_id("export_popup");
            let btn_resp = ui.add_sized(egui::vec2(COL3_W, TILE_HEIGHT), egui::Button::new("Export"));
            if btn_resp.clicked() { ui.memory_mut(|mem| mem.open_popup(id)); }
            
            if ui.memory(|mem| mem.is_popup_open(id)) {
                egui::popup_below_widget(ui, id, &btn_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui: &mut egui::Ui| {
                    ui.label("Export\noptions\ncoming\nsoon!");
                    if ui.button("Close").clicked() { ui.memory_mut(|mem| mem.close_popup()); }
                });
            }

            ui.add_space(GAP);

            // Speed Input
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                
                let lbl_w = 50.0;
                let inp_w = COL3_W - lbl_w - GAP;

                // Tile 1: "Speed" Label
                tile_frame.show(ui, |ui| {
                    ui.set_width(lbl_w);
                    ui.set_height(TILE_HEIGHT);
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.label(egui::RichText::new("Speed").color(egui::Color32::WHITE).size(12.0));
                    });
                });

                // Tile 2: Input Field
                tile_frame.show(ui, |ui| {
                    ui.set_width(inp_w);
                    ui.set_height(TILE_HEIGHT);
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        
                        let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.speed_str)
                            .hint_text(egui::RichText::new("1.0").color(egui::Color32::GRAY))
                            .frame(false)
                            .desired_width(inp_w)
                            .vertical_align(egui::Align::Center)
                            .horizontal_align(egui::Align::Center));
                            
                        if re.changed() {
                            if anim_viewer.speed_str.is_empty() {
                                anim_viewer.playback_speed = 1.0;
                            } else if let Ok(val) = anim_viewer.speed_str.parse::<f32>() {
                                anim_viewer.playback_speed = val.clamp(0.1, 10.0);
                            }
                        }
                        if !re.has_focus() && anim_viewer.speed_str.is_empty() {
                            anim_viewer.playback_speed = 1.0;
                        }
                    });
                });
            });
        });
    });

    // Update width cache
    let actual_width = controls_response.response.rect.width();
    if (anim_viewer.cached_controls_width - actual_width).abs() > 0.1 {
        anim_viewer.cached_controls_width = actual_width;
        ui.ctx().request_repaint();
    }

    ui.add_sized(
    egui::vec2(actual_width, 1.0), 
    egui::Separator::default().horizontal()
    );

    // =========================================================
    // 2. RENDER BUTTONS (Top Row)
    // =========================================================
    
    // Math for centering
    let top_row_w = (btn_w * 4.0) + (grid_gap * 3.0);
    let left_pad = if actual_width > top_row_w { (actual_width - top_row_w) / 2.0 } else { 0.0 };

    // Reserve height so BottomUp cursor moves correctly
    let grid_alloc = ui.allocate_ui(egui::vec2(ui.available_width(), anim_viewer.cached_grid_height), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(left_pad);
                egui::Grid::new("anim_controls_grid")
                    .spacing(egui::vec2(grid_gap, grid_gap))
                    .show(ui, |ui| {
                        let mut draw_anim_btn = |ui: &mut egui::Ui, label: &str, idx: usize, is_enabled: bool| {
                            let is_active = anim_viewer.loaded_anim_index == idx;
                            let fill = if is_active { active_color } else { inactive_color };
                            let btn = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(13.0)).fill(fill);
                            if ui.add_enabled_ui(is_enabled, |ui| { ui.add_sized(btn_size, btn) }).inner.clicked() { clicked_index = Some(idx); }
                        };

                        // Row 1
                        let has_walk = available_anims.iter().any(|(i, _, _)| *i == IDX_WALK); draw_anim_btn(ui, "Walk", IDX_WALK, has_walk);
                        let has_idle = available_anims.iter().any(|(i, _, _)| *i == IDX_IDLE); draw_anim_btn(ui, "Idle", IDX_IDLE, has_idle);
                        let has_atk = available_anims.iter().any(|(i, _, _)| *i == IDX_ATTACK); draw_anim_btn(ui, "Attack", IDX_ATTACK, has_atk);
                        let has_kb = available_anims.iter().any(|(i, _, _)| *i == IDX_KB); draw_anim_btn(ui, "Knockback", IDX_KB, has_kb);
                        ui.end_row();

                        // Row 2
                        let has_burrow = available_anims.iter().any(|(i, _, _)| *i == IDX_BURROW); draw_anim_btn(ui, "Burrow", IDX_BURROW, has_burrow);
                        let has_surface = available_anims.iter().any(|(i, _, _)| *i == IDX_SURFACE); draw_anim_btn(ui, "Surface", IDX_SURFACE, has_surface);
                        draw_anim_btn(ui, "Spirit", IDX_SPIRIT, spirit_available);
                        draw_anim_btn(ui, "Model", IDX_MODEL, base_assets_available);
                        ui.end_row();
                    });
            });
        });
    });

    // Update height cache
    let actual_grid_height = grid_alloc.response.rect.height();
    if (anim_viewer.cached_grid_height - actual_grid_height).abs() > 0.1 {
        anim_viewer.cached_grid_height = actual_grid_height;
        ui.ctx().request_repaint();
    }

    if let Some(target_idx) = clicked_index {
        if !is_loading_new {
            anim_viewer.loaded_anim_index = target_idx;
            let intended_target_id = if target_idx == IDX_SPIRIT { spirit_sheet_id.to_string() } else { form_viewer_id.to_string() };
            if anim_viewer.loaded_id == intended_target_id {
                let anim_path = if target_idx == IDX_SPIRIT { spirit_pack.as_ref().map(|(_, _, _, a)| a) }
                else { available_anims.iter().find(|(i, _, _)| *i == target_idx).map(|(_, _, p)| p) };
                if let Some(a_path) = anim_path { anim_viewer.load_anim(a_path); } else if target_idx == IDX_MODEL { anim_viewer.current_anim = None; }
            }
        }
    }
}