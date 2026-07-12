use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;
use nyanko::graphics::rig::{Animation, Unit};

use core::animation::export::state::{ExportMode, ExporterState};
use core::animation::logic::canvas::GlowRenderer;
use core::animation::logic::constants::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT,
    IDX_SURFACE, IDX_WALK,
};
use core::settings::logic::state::Settings;

use crate::global::shared::DragGuard;

use super::controls::render_controls_overlay;
use super::{canvas, controls, export, process};

pub struct AnimViewer {
    pub zoom_level: f32,
    pub target_zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub current_anim: Option<Arc<Animation>>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub loop_range: (Option<i32>, Option<i32>),
    pub range_str_cache: (String, String),
    pub single_frame_str: String,
    pub speed_str: String,
    pub hold_timer: f32,
    pub hold_dir: i8,
    pub loaded_anim_index: usize,
    pub last_anim_index: usize,
    pub loaded_id: String,
    pub summoner_id: String,
    last_loaded_id: String,
    pub pending_initial_center: bool,
    pub held_unit: Option<Arc<Unit>>,
    pub renderer: Arc<Mutex<Option<GlowRenderer>>>,
    pub cached_controls_width: f32,
    pub cached_grid_height: f32,
    pub is_expanded: bool,
    pub texture_version: u64,
    pub is_pointer_over_controls: bool,
    pub is_viewport_dragging: bool,
    pub is_selecting_export_region: bool,
    pub export_selection_start: Option<egui::Pos2>,
    pub export_state: ExporterState,
    pub has_scanned_showcase: bool,
    pub was_export_popup_open: bool,
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self {
            zoom_level: 1.0,
            target_zoom_level: 1.0,
            pan_offset: egui::vec2(0.0, 0.0),
            current_anim: None,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loop_range: (None, None),
            range_str_cache: (String::new(), String::new()),
            single_frame_str: String::new(),
            speed_str: String::new(),
            hold_timer: 0.0,
            hold_dir: 0,
            loaded_anim_index: 0,
            last_anim_index: usize::MAX,
            loaded_id: String::new(),
            summoner_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            held_unit: None,
            renderer: Arc::new(Mutex::new(None)),
            cached_controls_width: 0.0,
            cached_grid_height: 55.0,
            is_expanded: false,
            texture_version: 0,
            is_pointer_over_controls: false,
            is_viewport_dragging: false,
            is_selecting_export_region: false,
            export_selection_start: None,
            export_state: ExporterState::default(),
            has_scanned_showcase: false,
            was_export_popup_open: false,
        }
    }
}

impl AnimViewer {
    fn update_export_state(&mut self, _settings: &Settings) {
        self.export_state.loop_supported = self.loaded_anim_index == IDX_WALK || self.loaded_anim_index == IDX_IDLE;

        if self.export_state.export_mode != ExportMode::Showcase {
            if let Some(anim) = &self.current_anim {
                self.export_state.max_frame = anim.max_frame;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = anim.max_frame;
            } else {
                self.export_state.max_frame = 0;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = 0;
            }
            self.export_state.frame_start_str.clear();
            self.export_state.frame_end_str.clear();
        }

        let type_string = match self.loaded_anim_index {
            IDX_WALK => "walk",
            IDX_IDLE => "idle",
            IDX_ATTACK => "attack",
            IDX_KB => "kb",
            IDX_BURROW => "burrow",
            IDX_SURFACE => "surface",
            IDX_SPIRIT => "spirit",
            IDX_MODEL => "model",
            _ => "anim",
        };

        let raw_id = if self.loaded_anim_index == IDX_SPIRIT {
            if self.summoner_id.is_empty() { &self.loaded_id } else { &self.summoner_id }
        } else { &self.loaded_id };

        let id_parts: Vec<&str> = raw_id.split('_').collect();
        let mut clean_id = id_parts[0].to_string();

        if id_parts.len() >= 2
            && id_parts[0].chars().all(char::is_numeric) {
                let form_number = match id_parts[1].chars().next() {
                    Some('f') => 1, Some('c') => 2, Some('s') => 3, Some('u') => 4, _ => 0
                };
                if form_number > 0 { clean_id = format!("{}-{}", id_parts[0], form_number); }
            }

        self.export_state.name_prefix = format!("{}.{}", clean_id, type_string);
    }

    pub fn load_anim(&mut self, path: &Path, settings: &Settings) {
        if let Ok(anim_bytes) = std::fs::read(path)
            && let Some(anim) = Animation::parse(&anim_bytes) {
                self.current_frame = 0.0;
                self.loop_range = (None, None);
                self.range_str_cache = (String::new(), String::new());
                self.single_frame_str = "0".to_string();

                self.current_anim = Some(Arc::new(anim));
                self.update_export_state(settings);
                return;
            }

        self.current_anim = None;
        self.current_frame = 0.0;
        self.loop_range = (None, None);
        self.range_str_cache = (String::new(), String::new());
        self.single_frame_str = "0".to_string();
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        primary_id: &str,
        secondary_id: &str,
        available_anims: &[(usize, PathBuf)],
        primary_assets: Option<(PathBuf, PathBuf, PathBuf)>,
        secondary_assets: Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        unit_sync: &mut Option<Arc<Unit>>,
        settings: &mut Settings,
        drag_guard: &mut DragGuard,
    ) {
        let base_assets_available = primary_assets.is_some();
        let secondary_available = secondary_assets.is_some();

        let current_index = self.loaded_anim_index;
        let mut valid_index = current_index;

        let is_current_valid = if current_index == IDX_NONE {
            false
        } else if current_index == IDX_SPIRIT {
            secondary_available
        } else if current_index == IDX_MODEL {
            base_assets_available
        } else {
            base_assets_available && available_anims.iter().any(|(index, _)| *index == current_index)
        };

        if !is_current_valid {
            valid_index = IDX_NONE;

            if base_assets_available {
                let priority_list = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
                for check_index in priority_list {
                    if available_anims.iter().any(|(index, _)| *index == check_index) {
                        valid_index = check_index;
                        break;
                    }
                }
            }

            if valid_index == IDX_NONE && secondary_available { valid_index = IDX_SPIRIT; }
            if valid_index == IDX_NONE && base_assets_available { valid_index = IDX_MODEL; }
        }

        if valid_index != current_index {
            self.loaded_anim_index = valid_index;
            if valid_index == IDX_NONE {
                self.current_anim = None;
                self.held_unit = None;
                *unit_sync = None;
            }
            if current_index == IDX_NONE && valid_index != IDX_NONE {
                self.loaded_id.clear();
            }
        }

        let mut anim_index_changed = false;
        if self.last_anim_index != valid_index {
            self.last_anim_index = valid_index;
            anim_index_changed = true;
        }

        let target_viewer_id = if self.loaded_anim_index == IDX_SPIRIT { secondary_id.to_string() } else { primary_id.to_string() };

        let is_stable = self.loaded_id == target_viewer_id;
        let is_first_launch = self.held_unit.is_none() && unit_sync.is_none();

        if valid_index == IDX_NONE && !is_stable {
            self.loaded_id = target_viewer_id.clone();
        }

        if is_stable
            && let Some(unit) = unit_sync {
                self.held_unit = Some(unit.clone());
            }
        
        if (!is_stable || is_first_launch) && valid_index != IDX_NONE {
            let (resolved_png, resolved_cut, resolved_model, resolved_anim) = resolve_paths(valid_index, &primary_assets, &secondary_assets, available_anims);

            let mut load_success = false;
            if let (Some(png_path), Some(cut_path), Some(model_path)) = (resolved_png, resolved_cut, resolved_model)
                && let (Ok(png_bytes), Ok(cut_bytes), Ok(model_bytes)) = (
                    std::fs::read(png_path), std::fs::read(cut_path), std::fs::read(model_path)
                )
                    && let Some(parsed_unit) = Unit::parse(&png_bytes, &cut_bytes, &model_bytes) {
                        let arc_unit = Arc::new(parsed_unit);
                        self.held_unit = Some(arc_unit.clone());
                        *unit_sync = Some(arc_unit);

                        self.loaded_id = target_viewer_id.clone();
                        self.pending_initial_center = true;
                        load_success = true;
                    }

            if !load_success {
                self.loaded_id = target_viewer_id.clone();
                self.held_unit = None;
            } else if let Some(animation_path) = resolved_anim {
                self.load_anim(animation_path, settings);
            } else {
                self.current_anim = None;
                self.update_export_state(settings);
            }
        } else if anim_index_changed && is_stable && valid_index != IDX_NONE {
            let (_, _, _, resolved_anim) = resolve_paths(valid_index, &primary_assets, &secondary_assets, available_anims);
            if let Some(animation_path) = resolved_anim {
                self.load_anim(animation_path, settings);
            } else {
                self.current_anim = None;
                self.update_export_state(settings);
            }
        }
        
        if self.is_expanded {
            egui::Area::new("expanded_anim_viewer_area".into())
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    egui::Frame::window(&ctx.style())
                        .inner_margin(0.0)
                        .shadow(egui::epaint::Shadow::NONE)
                        .show(ui, |ui| {
                            ui.set_min_size(screen_rect.size());
                            ui.set_max_size(screen_rect.size());
                            let (viewport_rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                            ui.put(viewport_rect, |ui: &mut egui::Ui| {
                                self.draw_viewport(
                                    ui, viewport_rect, available_anims, base_assets_available,
                                    primary_id, secondary_id, &secondary_assets, settings, drag_guard
                                );
                                ui.allocate_rect(viewport_rect, egui::Sense::hover())
                            });
                        });
                });

            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("Animation Expanded").size(16.0).weak());
                if ui.button("Restore View").clicked() {
                    self.is_expanded = false;
                }
            });
        } else {
            ui.vertical(|ui| {
                let (viewport_rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                ui.put(viewport_rect, |ui: &mut egui::Ui| {
                    self.draw_viewport(
                        ui, viewport_rect, available_anims, base_assets_available,
                        primary_id, secondary_id, &secondary_assets, settings, drag_guard
                    );
                    ui.allocate_rect(viewport_rect, egui::Sense::hover())
                });
            });
        }
    }

    fn draw_viewport(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        available_anims: &[(usize, PathBuf)],
        base_assets_available: bool,
        primary_id: &str,
        secondary_id: &str,
        secondary_assets: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        settings: &mut Settings,
        drag_guard: &mut DragGuard,
    ) {
        let delta_time = ui.input(|input_state| input_state.stable_dt);
        let interpolation = settings.animation.interpolation;
        let debug_show_info = settings.animation.debug_view;
        let centering_behavior = settings.animation.centering_behavior;
        let native_fps = settings.animation.native_fps;

        if !primary_id.is_empty() { self.summoner_id = primary_id.to_string(); }

        if self.loaded_id != self.last_loaded_id {
            self.last_loaded_id = self.loaded_id.clone();
            self.pending_initial_center = true;
            self.has_scanned_showcase = false;

            let mut preserved_loop_msg: Option<String> = None;
            let mut preserved_export_msg: Option<String> = None;
            let mut preserved_time: Option<f64> = None;

            if self.export_state.is_loop_searching {
                if let Some(abort_signal) = &self.export_state.loop_abort {
                    abort_signal.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                preserved_loop_msg = Some("Loop Terminated!".to_string());
                preserved_time = Some(ui.input(|input_state| input_state.time));
            }

            if self.export_state.is_processing {
                if let Some(abort_signal) = &self.export_state.abort {
                    abort_signal.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                preserved_export_msg = Some("Export Terminated!".to_string());
                preserved_time = Some(ui.input(|input_state| input_state.time));
            }

            let previous_mode = self.export_state.export_mode.clone();

            self.export_state = ExporterState::with_settings(settings);
            self.export_state.export_mode = previous_mode;

            if let Some(message) = preserved_loop_msg {
                self.export_state.loop_result_msg = Some(message);
                self.export_state.completion_time = preserved_time;
            }
            if let Some(message) = preserved_export_msg {
                self.export_state.export_result_msg = Some(message);
                self.export_state.completion_time = preserved_time;
            }

            self.update_export_state(settings);
        }

        if self.export_state.export_mode == ExportMode::Loop
            && !self.export_state.loop_supported {
                self.export_state.export_mode = ExportMode::Manual;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = 0;
                self.export_state.frame_start_str.clear();
                self.export_state.frame_end_str.clear();
            }

        let mut new_center: Option<(egui::Vec2, f32)> = None;
        let mut should_clear_pending = false;

        if let Some(unit) = &self.held_unit {
            if self.pending_initial_center {
                if centering_behavior == 0 {
                    let tolerance_level = if settings.animation.use_tight_bounds { 1.0 } else { 0.0 };
                    let anims_to_check = if let Some(a) = &self.current_anim { vec![a.as_ref()] } else { vec![] };

                    if let Some((bounds_x, bounds_y, w, h)) = unit.calculate_bounds(&anims_to_check, tolerance_level) {
                        let center_x = bounds_x + (w / 2.0);
                        let center_y = bounds_y + (h / 2.0);
                        let pan = egui::vec2(-center_x, -center_y);

                        let scale_x = ui.available_size().x / w.max(1.0);
                        let scale_y = ui.available_size().y / h.max(1.0);
                        let breathing_room = 0.45;
                        let fit_zoom = scale_x.min(scale_y).clamp(0.05, 5.0) * breathing_room;

                        new_center = Some((pan, fit_zoom));
                    }
                } else if centering_behavior == 1 {
                    new_center = Some((egui::Vec2::ZERO, self.target_zoom_level));
                } else {
                    should_clear_pending = true;
                }
            }
        } else {
            should_clear_pending = true;
        }

        if let (Some(offset), Some(zoom)) = (new_center.map(|c| c.0), new_center.map(|c| c.1)) {
            self.pan_offset = offset;
            if centering_behavior == 0 { self.target_zoom_level = zoom; }
            self.pending_initial_center = false;
        } else if should_clear_pending { self.pending_initial_center = false; }

        let zoom_difference = self.target_zoom_level - self.zoom_level;
        if zoom_difference.abs() > 0.001 { self.zoom_level += zoom_difference * 15.0 * delta_time; } else { self.zoom_level = self.target_zoom_level; }

        if let Some(animation) = &self.current_anim {
            let loop_lcm_max = if self.loaded_anim_index <= 1 { animation.calculate_true_loop() } else { Some(animation.max_frame) };
            let range_start = self.loop_range.0.unwrap_or(0);
            let (effective_max, is_infinite, has_user_override) = match (self.loop_range.1, loop_lcm_max) {
                (Some(user_override), _) => (user_override as f32, false, true),
                (None, Some(calculated_max)) => (calculated_max as f32, false, false),
                (None, None) => (0.0, true, false),
            };

            if self.hold_dir != 0 {
                self.hold_timer += delta_time;
                ui.ctx().request_repaint();
                if self.hold_timer > 0.2 {
                    let speed_factor = if self.playback_speed.abs() < 0.05 { 1.0 } else { self.playback_speed.abs() };
                    let frame_delta = self.hold_dir as f32 * delta_time * 30.0 * speed_factor;
                    let mut new_frame = self.current_frame + frame_delta;
                    if !is_infinite {
                        if new_frame > effective_max { new_frame = 0.0; } else if new_frame < 0.0 { new_frame = effective_max; }
                    } else { if new_frame < 0.0 { new_frame = 0.0; } }
                    self.current_frame = new_frame;
                }
            } else { self.hold_timer = 0.0; }

            if self.is_playing {
                if !is_infinite && effective_max < 1.0 && !has_user_override { self.current_frame = 0.0; } else {
                    self.current_frame += delta_time * 30.0 * self.playback_speed;
                    if !is_infinite && self.current_frame > effective_max { self.current_frame = range_start as f32; }
                }
                ui.ctx().request_repaint();
            }
        }

        let (rect_alloc, viewport_response) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
        let (hover_position, right_mouse_down, left_mouse_down) = ui.input(|input_state| (input_state.pointer.hover_pos(), input_state.pointer.secondary_down(), input_state.pointer.primary_down()));

        let block_input = self.is_pointer_over_controls || (self.is_selecting_export_region && right_mouse_down);
        if self.is_selecting_export_region && left_mouse_down && hover_position.is_some() {
            controls::handle_viewport_input(ui, &viewport_response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, &mut self.pending_initial_center, false, &mut self.is_viewport_dragging);
        } else {
            controls::handle_viewport_input(ui, &viewport_response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, &mut self.pending_initial_center, block_input, &mut self.is_viewport_dragging);
        }

        if self.is_selecting_export_region {
            ui.painter().rect_filled(rect_alloc, 0.0, egui::Color32::from_black_alpha(50));
            let selection_painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("anim_export_tip")));
            let tip_text = "Right click & drag to set camera";
            let tooltip_font_id = egui::FontId::proportional(13.0);
            let tooltip_galley = selection_painter.layout_no_wrap(tip_text.to_string(), tooltip_font_id, egui::Color32::WHITE);
            let tooltip_margin = 6.0;
            let tooltip_width = tooltip_galley.size().x + tooltip_margin * 2.0;
            let tooltip_height = tooltip_galley.size().y + tooltip_margin * 2.0;
            let top_center_position = rect_alloc.center_top() + egui::vec2(0.0, 30.0);
            let tooltip_rect = egui::Rect::from_center_size(top_center_position, egui::vec2(tooltip_width, tooltip_height));
            selection_painter.rect(tooltip_rect, 4.0, egui::Color32::from_black_alpha(180), egui::Stroke::new(1.0, egui::Color32::from_gray(180)));
            selection_painter.galley(tooltip_rect.min + egui::vec2(tooltip_margin, tooltip_margin), tooltip_galley, egui::Color32::WHITE);

            if let Some(pointer_pos) = hover_position {
                if right_mouse_down {
                    if self.export_selection_start.is_none()
                        && rect_alloc.contains(pointer_pos) && ui.input(|input_state| input_state.pointer.button_pressed(egui::PointerButton::Secondary)) {
                            self.export_selection_start = Some(pointer_pos);
                        }
                    if let Some(selection_start) = self.export_selection_start {
                        let selection_rect = egui::Rect::from_two_pos(selection_start, pointer_pos);
                        ui.painter().with_clip_rect(rect_alloc).rect_stroke(selection_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));
                        ui.painter().with_clip_rect(rect_alloc).rect_filled(selection_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30));
                    }
                } else if let Some(selection_start) = self.export_selection_start.take() {
                    let selection_rect = egui::Rect::from_two_pos(selection_start, pointer_pos);
                    if selection_rect.width() * selection_rect.height() > 25.0 {
                        let screen_center = rect_alloc.center();
                        let to_world = |screen_pos: egui::Pos2| -> egui::Vec2 { ((screen_pos - screen_center) / self.zoom_level) - self.pan_offset };
                        let min_world = to_world(selection_rect.min);
                        let max_world = to_world(selection_rect.max);
                        self.export_state.region_x = min_world.x;
                        self.export_state.region_y = min_world.y;
                        self.export_state.region_w = (max_world.x - min_world.x).abs();
                        self.export_state.region_h = (max_world.y - min_world.y).abs();
                        self.export_state.zoom = 1.0;
                        self.is_selecting_export_region = false;
                        settings.animation.export_popup_open = true;
                    } else {
                        self.is_selecting_export_region = false;
                        settings.animation.export_popup_open = true;
                    }
                }
            }
        }

        let popup_just_opened = settings.animation.export_popup_open && !self.was_export_popup_open;
        if popup_just_opened {
            if self.export_state.format == core::animation::export::encoding::ExportFormat::Gif && settings.animation.last_export_format != 0 {
                self.export_state.format = match settings.animation.last_export_format {
                    1 => core::animation::export::encoding::ExportFormat::WebP,
                    2 => core::animation::export::encoding::ExportFormat::Avif,
                    3 => core::animation::export::encoding::ExportFormat::Png,
                    4 => core::animation::export::encoding::ExportFormat::Mp4,
                    5 => core::animation::export::encoding::ExportFormat::Mkv,
                    6 => core::animation::export::encoding::ExportFormat::Webm,
                    7 => core::animation::export::encoding::ExportFormat::Zip,
                    _ => core::animation::export::encoding::ExportFormat::Gif,
                };
            }

            if settings.animation.auto_set_camera_region && !self.is_selecting_export_region
                && let Some(unit_data) = &self.held_unit {
                    let tolerance_level = if settings.animation.use_tight_bounds { 1.0 } else { 0.0 };
                    let mut showcase_anims = Vec::new();
                    let mut anim_refs = Vec::new();

                    if self.export_state.export_mode == ExportMode::Showcase {
                        for target_idx in [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB] {
                            if let Some((_, path)) = available_anims.iter().find(|(idx, _)| *idx == target_idx)
                                && let Ok(bytes) = std::fs::read(path)
                                    && let Some(anim) = Animation::parse(&bytes) {
                                        showcase_anims.push(anim);
                                    }
                        }
                        for anim in &showcase_anims { anim_refs.push(anim); }
                    } else {
                        if let Some(anim) = &self.current_anim {
                            anim_refs.push(anim.as_ref());
                        }
                    }

                    if let Some((x, y, w, h)) = unit_data.calculate_bounds(&anim_refs, tolerance_level) {
                        self.export_state.region_x = x;
                        self.export_state.region_y = y;
                        self.export_state.region_w = w;
                        self.export_state.region_h = h;
                        self.export_state.zoom = 1.0;
                    }
                }
        }
        self.was_export_popup_open = settings.animation.export_popup_open;

        let walk_mismatch = self.export_state.last_known_walk_default != settings.animation.default_showcase_walk;
        let idle_mismatch = self.export_state.last_known_idle_default != settings.animation.default_showcase_idle;
        let kb_mismatch = self.export_state.last_known_kb_default != settings.animation.default_showcase_kb;

        if walk_mismatch || idle_mismatch || kb_mismatch {
            self.export_state.last_known_walk_default = settings.animation.default_showcase_walk;
            self.export_state.last_known_idle_default = settings.animation.default_showcase_idle;
            self.export_state.last_known_kb_default = settings.animation.default_showcase_kb;

            if self.export_state.showcase_walk_str.is_empty() {
                self.export_state.showcase_walk_len = settings.animation.default_showcase_walk;
            }
            if self.export_state.showcase_idle_str.is_empty() {
                self.export_state.showcase_idle_len = settings.animation.default_showcase_idle;
            }
            if self.export_state.showcase_kb_str.is_empty() {
                self.export_state.showcase_kb_len = settings.animation.default_showcase_kb;
            }

            self.has_scanned_showcase = false;
        }

        if settings.animation.export_popup_open && self.export_state.export_mode == ExportMode::Showcase && !self.has_scanned_showcase {
            let scan_duration = |target_idx: usize| -> Option<i32> {
                available_anims.iter()
                    .find(|(idx, _)| *idx == target_idx)
                    .and_then(|(_, path)| std::fs::read(path).ok())
                    .and_then(|bytes| Animation::parse(&bytes))
                    .map(|anim| anim.max_frame)
            };

            let scan_loop = |target_idx: usize| -> Option<i32> {
                available_anims.iter()
                    .find(|(idx, _)| *idx == target_idx)
                    .and_then(|(_, path)| std::fs::read(path).ok())
                    .and_then(|bytes| Animation::parse(&bytes))
                    .map(|anim| anim.calculate_true_loop().unwrap_or(anim.max_frame))
            };

            if let Some(attack_max) = scan_duration(IDX_ATTACK) {
                let total_attack_frames = attack_max + 1;
                self.export_state.detected_attack_len = total_attack_frames;
                if self.export_state.showcase_attack_str.is_empty() {
                    self.export_state.showcase_attack_len = total_attack_frames;
                }
            }

            if let Some(walk_loop) = scan_loop(IDX_WALK) {
                let is_short_walk = walk_loop <= 1;
                let new_walk_length = if is_short_walk { 0 } else { settings.animation.default_showcase_walk };
                self.export_state.detected_walk_len = new_walk_length;

                if self.export_state.showcase_walk_str.is_empty() || self.export_state.showcase_walk_len == settings.animation.default_showcase_walk {
                    self.export_state.showcase_walk_len = new_walk_length;
                }
            }

            if let Some(idle_loop) = scan_loop(IDX_IDLE) {
                let is_short_idle = idle_loop <= 1;
                let new_idle_length = if is_short_idle { 0 } else { settings.animation.default_showcase_idle };

                self.export_state.detected_idle_len = new_idle_length;

                if self.export_state.showcase_idle_str.is_empty() || self.export_state.showcase_idle_len == settings.animation.default_showcase_idle {
                    self.export_state.showcase_idle_len = new_idle_length;
                }
            }

            self.has_scanned_showcase = true;
        }

        let mut showcase_render_time = 0.0;

        if self.export_state.is_processing && self.export_state.export_mode == ExportMode::Showcase {
            let walk_duration = self.export_state.showcase_walk_len;
            let idle_duration = self.export_state.showcase_idle_len;
            let attack_duration = self.export_state.showcase_attack_len;
            let kb_duration = self.export_state.showcase_kb_len;

            let progress = self.export_state.current_progress;

            let target_index = if progress < walk_duration {
                showcase_render_time = (progress % (if walk_duration < 1 { 1 } else { walk_duration })) as f32;
                IDX_WALK
            } else if progress < walk_duration + idle_duration {
                showcase_render_time = ((progress - walk_duration) % (if idle_duration < 1 { 1 } else { idle_duration })) as f32;
                IDX_IDLE
            } else if progress < walk_duration + idle_duration + attack_duration {
                showcase_render_time = (progress - (walk_duration + idle_duration)) as f32;
                IDX_ATTACK
            } else {
                let kb_relative = progress - (walk_duration + idle_duration + attack_duration);
                showcase_render_time = (kb_relative % (if kb_duration < 1 { 1 } else { kb_duration })) as f32;
                IDX_KB
            };

            if self.loaded_anim_index != target_index
                && let Some((_, path)) = available_anims.iter().find(|(index, _)| *index == target_index) {
                    self.load_anim(path, settings);
                    self.loaded_anim_index = target_index;
                }
        }

        if let Some(unit_data) = &self.held_unit {

            if self.export_state.is_processing {
                let time_to_use = if self.export_state.export_mode == ExportMode::Showcase {
                    if let Some(animation) = &self.current_anim {
                        if animation.max_frame == 0 { 0.0 } else { showcase_render_time }
                    } else { 0.0 }
                } else {
                    let start = self.export_state.frame_start;
                    let step = if self.export_state.frame_start < self.export_state.frame_end { 1 } else { -1 };
                    (start + (self.export_state.current_progress * step)) as f32
                };

                process::process_frame(ui, rect_alloc, &mut self.export_state, unit_data.clone(), self.current_anim.clone(), self.renderer.clone(), time_to_use);
                ui.ctx().request_repaint();
            }

            canvas::paint(ui, rect_alloc, self.renderer.clone(), unit_data.clone(), self.current_anim.clone(), self.current_frame, self.pan_offset, self.zoom_level);

            if debug_show_info {
                let cross_center = rect_alloc.center() + self.pan_offset * self.zoom_level;
                let cross_size = 15.0;
                let cross_color = egui::Color32::GREEN;
                let cross_stroke = egui::Stroke::new(2.0, cross_color);
                ui.painter().line_segment([cross_center - egui::vec2(cross_size, 0.0), cross_center + egui::vec2(cross_size, 0.0)], cross_stroke);
                ui.painter().line_segment([cross_center - egui::vec2(0.0, cross_size), cross_center + egui::vec2(0.0, cross_size)], cross_stroke);
            }

            if settings.animation.export_popup_open
                && self.export_state.region_w > 0.1 && self.export_state.region_h > 0.1 {
                    let screen_center = rect_alloc.center();
                    let to_screen = |world_x: f32, world_y: f32| -> egui::Pos2 { let world_pos = egui::vec2(world_x, world_y); screen_center + (world_pos + self.pan_offset) * self.zoom_level };
                    let min = to_screen(self.export_state.region_x, self.export_state.region_y);
                    let max = to_screen(self.export_state.region_x + self.export_state.region_w, self.export_state.region_y + self.export_state.region_h);
                    ui.painter().with_clip_rect(rect_alloc).rect_stroke(egui::Rect::from_min_max(min, max), 0.0, egui::Stroke::new(1.0, egui::Color32::YELLOW));
                }
        } else { ui.painter().rect_filled(rect_alloc, 0.0, egui::Color32::from_rgb(20, 20, 20)); }

        let border_rect = rect_alloc.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165);
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));

        let button_size = egui::vec2(30.0, 30.0);
        let button_rect = egui::Rect::from_min_size(rect_alloc.min + egui::vec2(8.0, 8.0), button_size);
        let background_fill = if self.is_expanded { egui::Color32::from_rgb(31, 106, 165) } else { egui::Color32::from_gray(60) };

        let expand_button_response = ui.put(button_rect, |ui: &mut egui::Ui| {
            let button_widget = egui::Button::new(egui::RichText::new("⛶").size(20.0).color(egui::Color32::WHITE))
                .fill(background_fill)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .rounding(4.0);

            let response = ui.add_sized(button_size, button_widget);
            if response.clicked() {
                self.is_expanded = !self.is_expanded;
            }
            response
        });

        let controls_hovered = render_controls_overlay(ui, rect_alloc, self, available_anims, base_assets_available, false, secondary_id, primary_id, secondary_assets, interpolation, native_fps, settings);
        self.is_pointer_over_controls = controls_hovered || expand_button_response.hovered();

        export::show_popup(ui, &mut self.export_state, self.held_unit.clone(), self.current_anim.clone(), &mut self.is_selecting_export_region, settings, available_anims, drag_guard);
    }
}

fn resolve_paths<'a>(
    target_index: usize,
    primary_assets: &'a Option<(PathBuf, PathBuf, PathBuf)>,
    secondary_assets: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    available_anims: &'a [(usize, PathBuf)]
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    if target_index == IDX_SPIRIT {
        if let Some((secondary_png, secondary_cut, secondary_model, secondary_anim)) = secondary_assets {
            return (Some(secondary_png), Some(secondary_cut), Some(secondary_model), Some(secondary_anim));
        }
    } else {
        let animation_path = available_anims.iter().find(|(index, _)| *index == target_index).map(|(_, path)| path);
        if let Some((primary_png, primary_cut, primary_model)) = primary_assets {
            return (Some(primary_png), Some(primary_cut), Some(primary_model), animation_path);
        }
    }
    (None, None, None, None)
}