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
    
    pub current_anim: Option<Animation>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    
    // 0=Walk, 1=Idle (Continuous) | 2=Attack, 3=KB (Once/Loop at End)
    pub loaded_anim_index: usize, 
    pub loaded_id: String, 
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self { 
            zoom_level: 1.0, 
            pan_offset: egui::vec2(0.0, 0.0),
            debug_show_info: false,
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
        // --- 1. PLAYBACK LOGIC ---
        if self.is_playing {
            if let Some(anim) = &self.current_anim {
                let dt = ui.input(|i| i.stable_dt);
                // Advance frame (30 FPS base)
                self.current_frame += dt * 30.0 * self.playback_speed;
                
                // LOOPING LOGIC
                // Walk (0) and Idle (1) are "Continuous" - they run forever.
                // Attack (2) and KB (3) are "Once" - we loop them at max_frame so the user can see them repeat.
                if self.loaded_anim_index >= 2 {
                    let loop_point = if anim.max_frame > 0 { anim.max_frame as f32 } else { 100.0 };
                    // We add +1.0 because frame 0 is distinct from frame Max
                    if self.current_frame >= loop_point + 1.0 {
                        self.current_frame = 0.0;
                    }
                } else {
                    // For Walk/Idle, we let it run endless.
                    // (Optional: prevent float overflow after days of running)
                    if self.current_frame > 1_000_000.0 { self.current_frame = 0.0; }
                }
                
                ui.ctx().request_repaint();
            }
        }

        // --- 2. TOOLBAR ---
        ui.horizontal(|ui| {
            if ui.button(if self.is_playing { "Pause" } else { "Play" }).clicked() {
                self.is_playing = !self.is_playing;
            }
            
            if let Some(anim) = &self.current_anim {
                // Info display
                let type_str = if self.loaded_anim_index < 2 { "Endless" } else { "Looped" };
                ui.label(format!("F: {:.0} / {} ({})", self.current_frame, anim.max_frame, type_str));
            } else {
                ui.label("No Animation");
            }
            
            if ui.button("Reset View").clicked() {
                self.pan_offset = egui::vec2(0.0, 0.0);
                self.zoom_level = 1.0;
            }
            ui.checkbox(&mut self.debug_show_info, "Debug");
        });

        // --- 3. INPUT ---
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
        if response.dragged() { self.pan_offset += response.drag_delta(); }
        ui.input(|i| { if i.zoom_delta() != 1.0 { self.zoom_level *= i.zoom_delta(); } });

        // --- 4. RENDER ---
        let parts_to_draw = if let Some(anim) = &self.current_anim {
            let animated_parts = animator::animate(model, anim, self.current_frame);
            transform::solve_hierarchy(&animated_parts, model)
        } else {
            transform::solve_hierarchy(&model.parts, model)
        };

        canvas::paint(&painter, response.rect, sprite_sheet, &parts_to_draw, self.pan_offset, self.zoom_level, self.debug_show_info);
    }
}