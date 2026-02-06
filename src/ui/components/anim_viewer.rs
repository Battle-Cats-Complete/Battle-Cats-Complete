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
    
    pub loaded_anim_index: usize, 
    pub loaded_id: String,
    
    last_loaded_id: String,
    pub pending_initial_center: bool,
    
    // Staging buffers for seamless transitions
    pub staging_model: Option<Model>,
    pub staging_sheet: Option<SpriteSheet>,
    
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
            loaded_anim_index: 0, 
            loaded_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            
            staging_model: None,
            staging_sheet: None,
            
            renderer: Arc::new(Mutex::new(None)),
        }
    }
}

impl AnimViewer {
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current_anim = None;
        self.current_frame = 0.0;
        self.loaded_anim_index = 0;
        self.loaded_id.clear();
        self.last_loaded_id = "FORCE_RESET".to_string();
        self.pending_initial_center = false;
        
        self.staging_model = None;
        self.staging_sheet = None;
        
        if let Ok(mut r) = self.renderer.lock() {
            *r = None;
        }
    }

    pub fn load_anim(&mut self, path: &Path) {
        if let Some(anim) = Animation::load(path) {
            self.current_anim = Some(anim);
            self.current_frame = 0.0;
        } else {
            self.current_anim = None;
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
        _debug_show_info: bool, // Fixed warning
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
                    if !model.parts.is_empty() {
                        if self.center_view(model, sprite_sheet, ui.available_size()) {
                            self.pending_initial_center = false;
                        }
                    }
                },
                1 => { 
                    self.pan_offset = egui::Vec2::ZERO;
                    self.pending_initial_center = false;
                },
                2 => { 
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

        if self.is_playing {
            if let Some(anim) = &self.current_anim {
                self.current_frame += dt * 30.0 * self.playback_speed;
                let max_f = if anim.max_frame > 0 { anim.max_frame as f32 } else { 100.0 };
                
                if self.loaded_anim_index >= 2 {
                    if self.current_frame >= max_f + 1.0 { self.current_frame = 0.0; }
                } else {
                    if self.current_frame > 1_000_000.0 { self.current_frame = 0.0; }
                }
                
                ui.ctx().request_repaint();
            }
        }

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        
        controls::handle_viewport_input(
            ui, 
            &response, 
            &mut self.pan_offset, 
            &mut self.zoom_level, 
            &mut self.target_zoom_level, 
            &mut self.pending_initial_center
        );

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