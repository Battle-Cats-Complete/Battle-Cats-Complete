use eframe::egui;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::core::anim::{animator, canvas, transform, center, controls};

pub struct AnimViewer {
    pub zoom_level: f32,
    pub target_zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub current_anim: Option<Animation>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    
    // Range: None = "Continuous/Default", Some = "User Override"
    pub loop_range: (Option<i32>, Option<i32>),
    pub range_str_cache: (String, String),
    
    // Buffers for Single Frame and Speed inputs
    pub single_frame_str: String,
    pub speed_str: String,
    
    pub hold_timer: f32,
    pub hold_dir: i8, 
    pub loaded_anim_index: usize, 
    pub loaded_id: String,
    last_loaded_id: String,
    pub pending_initial_center: bool,
    pub staging_model: Option<Model>,
    pub staging_sheet: Option<SpriteSheet>,
    pub held_model: Option<Model>,
    pub held_sheet: Option<SpriteSheet>,
    pub renderer: Arc<Mutex<Option<canvas::GlowRenderer>>>,
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
            // CHANGED: Default is empty string so ghost text shows "1.0"
            speed_str: String::new(),
            hold_timer: 0.0,
            hold_dir: 0,
            loaded_anim_index: 0, 
            loaded_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            staging_model: None,
            staging_sheet: None,
            held_model: None,
            held_sheet: None,
            renderer: Arc::new(Mutex::new(None)),
        }
    }
}

impl AnimViewer {
    pub fn load_anim(&mut self, path: &Path) {
        if let Some(anim) = Animation::load(path) {
            self.current_frame = 0.0;
            self.loop_range = (None, None);
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
            // Keep speed persistence, don't reset speed_str
            self.current_anim = Some(anim);
        } else {
            self.current_anim = None;
            self.current_frame = 0.0;
            self.loop_range = (None, None); 
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
        }
    }

    pub fn center_view(&mut self, model: &Model, sheet: &SpriteSheet, viewport_size: egui::Vec2) -> bool {
        if let Some((offset, bounds)) = center::calculate_center_offset(model, self.current_anim.as_ref(), sheet) {
            self.pan_offset = offset;
            let fit_zoom = center::calculate_zoom_fit(bounds, viewport_size, 0.75);
            self.target_zoom_level = fit_zoom;
            true
        } else {
            false
        }
    }

    pub fn render(
        &mut self, 
        ui: &mut egui::Ui, 
        sprite_sheet: &SpriteSheet, 
        model: &Model,
        interpolation: bool,
        _debug_show_info: bool,
        centering_behavior: usize,
        allow_update: bool 
    ) {
        let dt = ui.input(|i| i.stable_dt);

        if self.loaded_id != self.last_loaded_id {
            self.last_loaded_id = self.loaded_id.clone();
            self.pending_initial_center = true;
        }

        if self.pending_initial_center {
            match centering_behavior {
                0 => { 
                    if !model.parts.is_empty() && self.center_view(model, sprite_sheet, ui.available_size()) {
                        self.pending_initial_center = false;
                    }
                },
                1 => { 
                    self.pan_offset = egui::Vec2::ZERO;
                    self.pending_initial_center = false;
                },
                _ => { self.pending_initial_center = false; }
            }
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
            
            // Loop Logic:
            // 1. User Override (Raw): Takes precedence. No clamping.
            // 2. LCM Calc (Auto): Used if finite.
            // 3. Infinite: Fallback.
            let (effective_max, is_infinite, has_user_override) = match (self.loop_range.1, lcm_max) {
                (Some(user_override), _) => (user_override as f32, false, true),
                (None, Some(calc)) => (calc as f32, false, false),
                (None, None) => (0.0, true, false), 
            };
            
            // 1. Hold Logic (Manual Scrub)
            if self.hold_dir != 0 {
                self.hold_timer += dt;
                ui.ctx().request_repaint();

                if self.hold_timer > 0.2 {
                   let delta = self.hold_dir as f32 * dt * 30.0;
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

            // 2. Play Logic (Auto Play)
            if self.is_playing {
                if !is_infinite && effective_max < 1.0 && !has_user_override {
                    self.current_frame = 0.0;
                } else {
                    self.current_frame += dt * 30.0 * self.playback_speed;
                    
                    if !is_infinite {
                        // CHANGED: If user override exists, loop back to start (even if start > end in weird cases, though unlikely)
                        if self.current_frame > effective_max {
                            self.current_frame = start as f32;
                        }
                    }
                }
                ui.ctx().request_repaint();
            }
        }

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        controls::handle_viewport_input(ui, &response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, &mut self.pending_initial_center);

        let parts_to_draw = if let Some(anim) = &self.current_anim {
            let frame = if interpolation { self.current_frame } else { (self.current_frame + 0.01).floor() };
            let animated_parts = animator::animate(model, anim, frame);
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

        let border_rect = rect.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165); 
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));
    }
}