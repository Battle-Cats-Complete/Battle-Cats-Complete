// ... (imports same as before) ...
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::core::anim::{animator, smooth, canvas, transform, center, controls};
use crate::ui::components::anim_controls;
use crate::ui::components::anim_exporter::{self, state::ExporterState};

pub struct AnimViewer {
    // ... (fields same as before) ...
    pub zoom_level: f32,
    pub target_zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub current_anim: Option<Animation>,
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
    pub loaded_id: String,
    pub summoner_id: String, // ADDED: Tracks the main unit ID (e.g. 700) even if Spirit (701) is loaded
    last_loaded_id: String,
    pub pending_initial_center: bool,
    pub staging_model: Option<Model>,
    pub staging_sheet: Option<SpriteSheet>,
    pub held_model: Option<Model>,
    pub held_sheet: Option<SpriteSheet>,
    pub renderer: Arc<Mutex<Option<canvas::GlowRenderer>>>,

    pub cached_controls_width: f32,
    pub cached_grid_height: f32, 
    
    pub is_expanded: bool,          
    pub is_controls_expanded: bool, 
    pub texture_version: u64,
    
    pub is_pointer_over_controls: bool,
    pub is_viewport_dragging: bool,

    // Export Fields
    pub is_selecting_export_region: bool,
    pub export_selection_start: Option<egui::Pos2>,
    pub export_state: ExporterState,
    pub show_export_popup: bool,
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
            loaded_id: String::new(),
            summoner_id: String::new(), // ADDED
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            staging_model: None,
            staging_sheet: None,
            held_model: None,
            held_sheet: None,
            renderer: Arc::new(Mutex::new(None)),
            cached_controls_width: 0.0,
            cached_grid_height: 55.0, 
            is_expanded: false,
            is_controls_expanded: true,
            texture_version: 0,
            is_pointer_over_controls: false,
            is_viewport_dragging: false,
            
            // Export Defaults
            is_selecting_export_region: false,
            export_selection_start: None,
            export_state: ExporterState::default(),
            show_export_popup: false,
        }
    }
}

impl AnimViewer {
    fn update_export_state(&mut self) {
        // Handle Frame limits (safe for None anim)
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

        // 1. Determine Animation Type String
        let type_str = match self.loaded_anim_index {
            anim_controls::IDX_WALK => "walk",
            anim_controls::IDX_IDLE => "idle",
            anim_controls::IDX_ATTACK => "attack",
            anim_controls::IDX_KB => "kb",
            anim_controls::IDX_BURROW => "burrow",
            anim_controls::IDX_SURFACE => "surface",
            anim_controls::IDX_SPIRIT => "spirit",
            anim_controls::IDX_MODEL => "model",
            _ => "anim",
        };

        // 2. Determine which ID to use
        // If Spirit: Use Summoner ID (e.g. 700) instead of Spirit ID (701)
        // If Model/Other: Use Loaded ID
        let raw_id = if self.loaded_anim_index == anim_controls::IDX_SPIRIT {
            if self.summoner_id.is_empty() { &self.loaded_id } else { &self.summoner_id }
        } else {
            &self.loaded_id
        };

        // 3. Clean ID (Remove leading zeros, map Form char)
        let mut clean_id = raw_id.clone();
        let parts: Vec<&str> = raw_id.split('_').collect();
        
        if parts.len() >= 2 {
            if parts[0].chars().all(char::is_numeric) {
                let form_num = match parts[1].chars().next() {
                    Some('f') => 1,
                    Some('c') => 2,
                    Some('s') => 3,
                    Some('u') => 4,
                    _ => 0
                };

                if form_num > 0 {
                    // Use string directly to preserve padding like "001" -> "001-1"
                    // Or parse if you WANT to remove zeros. 
                    // User request: "remove the leading 0... but [later] do not remove trailing zeros (padding?)"
                    // Clarification: "do not remove zero padding in the unit id" -> Keep "001".
                    clean_id = format!("{}-{}", parts[0], form_num);
                }
            }
        }
        
        self.export_state.name_prefix = format!("{}.{}", clean_id, type_str);
    }

    pub fn load_anim(&mut self, path: &Path) {
        if let Some(anim) = Animation::load(path) {
            self.current_frame = 0.0;
            self.loop_range = (None, None);
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
            
            self.current_anim = Some(anim);
            self.update_export_state();
            
        } else {
            self.current_anim = None;
            self.current_frame = 0.0;
            self.loop_range = (None, None); 
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
        }
    }

    pub fn render(
        &mut self, 
        ui: &mut egui::Ui, 
        interpolation: bool,
        debug_show_info: bool,
        centering_behavior: usize,
        allow_update: bool,
        available_anims: &Vec<(usize, &str, PathBuf)>,
        spirit_available: bool,
        base_assets_available: bool,
        is_loading_new: bool,
        spirit_sheet_id: &str,
        form_viewer_id: &str,
        spirit_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        native_fps: f32, 
    ) {
        let dt = ui.input(|i| i.stable_dt);

        // Update Summoner ID tracking
        if !form_viewer_id.is_empty() {
            self.summoner_id = form_viewer_id.to_string();
        }

        if self.loaded_id != self.last_loaded_id {
            self.last_loaded_id = self.loaded_id.clone();
            self.pending_initial_center = true;
            
            self.export_state = ExporterState::default();
            
            // FIXED: Always update export state on unit switch, even if model (anim=None)
            self.update_export_state();
        }

        let mut new_center: Option<(egui::Vec2, f32)> = None;
        let mut should_clear_pending = false;

        if let (Some(model), Some(sheet)) = (&self.held_model, &self.held_sheet) {
            if self.pending_initial_center {
                if centering_behavior == 0 { 
                    if !model.parts.is_empty() {
                        if let Some((offset, bounds)) = center::calculate_center_offset(model, self.current_anim.as_ref(), sheet) {
                            let fit_zoom = center::calculate_zoom_fit(bounds, ui.available_size(), 0.75);
                            new_center = Some((offset, fit_zoom));
                        }
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

        if let (Some(offset), Some(zoom)) = (new_center.map(|x| x.0), new_center.map(|x| x.1)) {
            self.pan_offset = offset;
            if centering_behavior == 0 { self.target_zoom_level = zoom; }
            self.pending_initial_center = false;
        } else if should_clear_pending {
            self.pending_initial_center = false;
        }

        let diff = self.target_zoom_level - self.zoom_level;
        if diff.abs() > 0.001 {
            self.zoom_level += diff * 15.0 * dt;
        } else {
            self.zoom_level = self.target_zoom_level;
        }

        if let Some(anim) = &self.current_anim {
            let lcm_max = if self.loaded_anim_index <= 1 {
                anim.calculate_true_loop()
            } else {
                Some(anim.max_frame)
            };

            let start = self.loop_range.0.unwrap_or(0);
            
            let (effective_max, is_infinite, has_user_override) = match (self.loop_range.1, lcm_max) {
                (Some(user_override), _) => (user_override as f32, false, true),
                (None, Some(calc)) => (calc as f32, false, false),
                (None, None) => (0.0, true, false), 
            };
            
            if self.hold_dir != 0 {
                self.hold_timer += dt;
                ui.ctx().request_repaint();

                if self.hold_timer > 0.2 {
                   let speed_factor = if self.playback_speed.abs() < 0.05 { 1.0 } else { self.playback_speed.abs() };
                   let delta = self.hold_dir as f32 * dt * 30.0 * speed_factor;
                   
                   let mut new_frame = self.current_frame + delta;
                   
                   if !is_infinite {
                       if new_frame > effective_max { new_frame = 0.0; }
                       else if new_frame < 0.0 { new_frame = effective_max; }
                   } else {
                       if new_frame < 0.0 { new_frame = 0.0; }
                   }
                   self.current_frame = new_frame;
                }
            } else {
                self.hold_timer = 0.0;
            }

            if self.is_playing {
                if !is_infinite && effective_max < 1.0 && !has_user_override {
                    self.current_frame = 0.0;
                } else {
                    self.current_frame += dt * 30.0 * self.playback_speed;
                    if !is_infinite {
                        if self.current_frame > effective_max {
                            self.current_frame = start as f32;
                        }
                    }
                }
                ui.ctx().request_repaint();
            }
        }

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        
        let (hover_pos, right_down, left_down) = ui.input(|i| (i.pointer.hover_pos(), i.pointer.secondary_down(), i.pointer.primary_down()));
        
        if self.is_selecting_export_region && left_down && hover_pos.is_some() {
            controls::handle_viewport_input(
                ui, &response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, 
                &mut self.pending_initial_center, false, &mut self.is_viewport_dragging 
            );
        } else {
            let block_input = self.is_pointer_over_controls || (self.is_selecting_export_region && right_down);
            controls::handle_viewport_input(
                ui, &response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, 
                &mut self.pending_initial_center, block_input, &mut self.is_viewport_dragging 
            );
        }

        if self.is_selecting_export_region {
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(50));

            ui.painter().text(
                rect.center(), 
                egui::Align2::CENTER_CENTER, 
                "Right-Click Drag to Select Region\n(Left-Click to Pan)", 
                egui::FontId::proportional(20.0), 
                egui::Color32::WHITE
            );

            if let Some(pos) = hover_pos {
                if right_down {
                    if self.export_selection_start.is_none() {
                        self.export_selection_start = Some(pos);
                    }
                    if let Some(start) = self.export_selection_start {
                        let selection_rect = egui::Rect::from_two_pos(start, pos);
                        ui.painter().rect_stroke(selection_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));
                        ui.painter().rect_filled(selection_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30));
                    }
                } else if self.export_selection_start.is_some() {
                    let start = self.export_selection_start.take().unwrap();
                    let selection_rect = egui::Rect::from_two_pos(start, pos);
                    let area = selection_rect.width() * selection_rect.height();

                    if area < 25.0 {
                        self.is_selecting_export_region = false;
                        self.show_export_popup = true;
                    } else {
                        let center_screen = rect.center();
                        let to_world = |p: egui::Pos2| -> egui::Vec2 {
                            ((p - center_screen) / self.zoom_level) - self.pan_offset
                        };

                        let min_w = to_world(selection_rect.min);
                        let max_w = to_world(selection_rect.max);
                        
                        self.export_state.region_x = min_w.x;
                        self.export_state.region_y = min_w.y;
                        self.export_state.region_w = (max_w.x - min_w.x).abs();
                        self.export_state.region_h = (max_w.y - min_w.y).abs();
                        self.export_state.zoom = 1.0; 
                        
                        self.is_selecting_export_region = false;
                        self.show_export_popup = true;
                    }
                }
            }
        }

        if self.export_state.is_processing {
            if let (Some(model), Some(anim), Some(sheet)) = (&self.held_model, &self.current_anim, &self.held_sheet) {
                anim_exporter::process_frame(
                    ui, 
                    rect,
                    &mut self.export_state, 
                    model, 
                    anim, 
                    sheet, 
                    self.renderer.clone()
                );
                ui.ctx().request_repaint();
            }
        }

        if let (Some(model), Some(sprite_sheet)) = (&self.held_model, &self.held_sheet) {
            let parts_to_draw = if let Some(anim) = &self.current_anim {
                let render_frame = self.current_frame;

                let animated_parts = if interpolation {
                    smooth::animate(model, anim, render_frame)
                } else {
                    let discrete_frame = (render_frame + 0.01).floor();
                    animator::animate(model, anim, discrete_frame)
                };
                
                transform::solve_hierarchy(&animated_parts, model)
            } else {
                transform::solve_hierarchy(&model.parts, model)
            };

            let sheet_arc = Arc::new(SpriteSheet {
                texture_handle: sprite_sheet.texture_handle.clone(),
                image_data: sprite_sheet.image_data.clone(),
                cuts_map: sprite_sheet.cuts_map.clone(),
                is_loading_active: sprite_sheet.is_loading_active,
                data_receiver: None, 
                sheet_name: sprite_sheet.sheet_name.clone(),
            });

            canvas::paint(ui, rect, self.renderer.clone(), sheet_arc, parts_to_draw, self.pan_offset, self.zoom_level, allow_update);
            
            if debug_show_info {
                let center = rect.center() + self.pan_offset * self.zoom_level;
                let size = 15.0;
                let color = egui::Color32::GREEN;
                let stroke = egui::Stroke::new(2.0, color);
                
                ui.painter().line_segment([center - egui::vec2(size, 0.0), center + egui::vec2(size, 0.0)], stroke);
                ui.painter().line_segment([center - egui::vec2(0.0, size), center + egui::vec2(0.0, size)], stroke);
            }
            
            if self.show_export_popup {
                 let center_screen = rect.center();
                 let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                     let world_pos = egui::vec2(wx, wy);
                     let screen_vec = (world_pos + self.pan_offset) * self.zoom_level;
                     center_screen + screen_vec
                 };
                 
                 let min = to_screen(self.export_state.region_x, self.export_state.region_y);
                 let max = to_screen(self.export_state.region_x + self.export_state.region_w, self.export_state.region_y + self.export_state.region_h);
                 let r = egui::Rect::from_min_max(min, max);
                 
                 ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::YELLOW));
            }

        } else {
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 20));
        }

        let border_rect = rect.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165); 
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));

        let btn_size = egui::vec2(30.0, 30.0);
        let margin = 8.0;
        let btn_pos = rect.min + egui::vec2(margin, margin);
        let btn_rect = egui::Rect::from_min_size(btn_pos, btn_size);

        let bg_fill = if self.is_expanded {
            egui::Color32::from_rgb(31, 106, 165)
        } else {
             egui::Color32::from_gray(60)
        };

        let btn_response = ui.put(btn_rect, |ui: &mut egui::Ui| {
             let btn = egui::Button::new(egui::RichText::new("⛶").size(20.0).color(egui::Color32::WHITE))
                .fill(bg_fill) 
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .rounding(4.0);
            
            let response = ui.add_sized(btn_size, btn);
            if response.clicked() {
                self.is_expanded = !self.is_expanded;
            }
            response
        });

        let controls_hovered = anim_controls::render_controls_overlay(
            ui,
            rect,
            self,
            available_anims,
            spirit_available,
            base_assets_available,
            is_loading_new, 
            spirit_sheet_id,
            form_viewer_id,
            spirit_pack,
            interpolation,
            native_fps, 
        );

        self.is_pointer_over_controls = controls_hovered || btn_response.hovered();

        // Render Export Popup
        let state = &mut self.export_state;
        let show_popup = &mut self.show_export_popup;
        let model = self.held_model.as_ref();
        let anim = self.current_anim.as_ref();
        let sheet = self.held_sheet.as_ref();
        let start_select = &mut self.is_selecting_export_region;

        anim_exporter::show_popup(
            ui, 
            state, 
            model, 
            anim, 
            sheet, 
            show_popup,
            start_select
        );
    }
}