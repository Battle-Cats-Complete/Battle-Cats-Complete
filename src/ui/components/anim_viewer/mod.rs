use eframe::egui;
use std::path::Path;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;

pub mod transform;
mod canvas;
mod animator;

pub struct AnimViewer {
    pub zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub debug_show_info: bool,
    
    // Toggle: Unchecked = Game Accurate (30FPS Snap), Checked = Smooth
    pub interpolation: bool, 
    
    pub current_anim: Option<Animation>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    
    pub loaded_anim_index: usize, 
    pub loaded_id: String, 
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self { 
            zoom_level: 1.0, 
            pan_offset: egui::vec2(0.0, 0.0),
            debug_show_info: false,
            interpolation: false, // Default to Game Accurate
            current_anim: None,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loaded_anim_index: 0, 
            loaded_id: String::new(),
        }
    }
}

impl AnimViewer {
    pub fn reset(&mut self) {
        self.current_anim = None;
        self.current_frame = 0.0;
        self.loaded_anim_index = 0;
        self.loaded_id.clear();
        self.pan_offset = egui::vec2(0.0, 0.0);
        self.zoom_level = 1.0;
        self.interpolation = false;
    }

    pub fn load_anim(&mut self, path: &Path) {
        if let Some(anim) = Animation::load(path) {
            self.current_anim = Some(anim);
            self.current_frame = 0.0;
        } else {
            eprintln!("Failed to load animation at {:?}", path);
            self.current_anim = None;
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, sprite_sheet: &SpriteSheet, model: &Model) {
        if self.is_playing {
            if let Some(anim) = &self.current_anim {
                let dt = ui.input(|i| i.stable_dt);
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

        ui.horizontal(|ui| {
            if ui.button(if self.is_playing { "Pause" } else { "Play" }).clicked() {
                self.is_playing = !self.is_playing;
            }
            if let Some(anim) = &self.current_anim {
                ui.label(format!("F: {:.1} / {}", self.current_frame, anim.max_frame));
            }
            if ui.button("Reset View").clicked() {
                self.pan_offset = egui::vec2(0.0, 0.0);
                self.zoom_level = 1.0;
            }
            ui.separator();
            ui.checkbox(&mut self.interpolation, "Interpolation");
            ui.checkbox(&mut self.debug_show_info, "Debug");
        });

        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
        if response.dragged() { self.pan_offset += response.drag_delta(); }
        ui.input(|i| { if i.zoom_delta() != 1.0 { self.zoom_level *= i.zoom_delta(); } });

        let parts_to_draw = if let Some(anim) = &self.current_anim {
            // JITTER FIX: Use floor with epsilon to handle float drift.
            let frame = if self.interpolation { 
                self.current_frame 
            } else { 
                (self.current_frame + 0.01).floor() 
            };
            
            let animated_parts = animator::animate(model, anim, frame);
            transform::solve_hierarchy(&animated_parts, model)
        } else {
            transform::solve_hierarchy(&model.parts, model)
        };

        canvas::paint(&painter, response.rect, sprite_sheet, &parts_to_draw, self.pan_offset, self.zoom_level);
    }
}