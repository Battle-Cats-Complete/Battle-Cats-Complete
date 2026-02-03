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
    pub debug_show_info: bool,
    pub interpolation: bool,
    
    pub current_anim: Option<Animation>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    
    pub loaded_anim_index: usize, 
    pub loaded_id: String,
    
    last_loaded_id: String,
    pub pending_initial_center: bool,
    
    pub renderer: Arc<Mutex<Option<canvas::GlowRenderer>>>,
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self { 
            zoom_level: 1.0, 
            target_zoom_level: 1.0,
            pan_offset: egui::vec2(0.0, 0.0),
            debug_show_info: false,
            interpolation: false,
            current_anim: None,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loaded_anim_index: 0, 
            loaded_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            renderer: Arc::new(Mutex::new(None)),
        }
    }
}

impl AnimViewer {
    pub fn reset(&mut self) {
        self.current_anim = None;
        self.current_frame = 0.0;
        self.loaded_anim_index = 0;
        self.loaded_id.clear();
        self.last_loaded_id = "FORCE_RESET".to_string();
        self.pending_initial_center = false;
        self.pan_offset = egui::vec2(0.0, 0.0);
        self.zoom_level = 1.0;
        self.target_zoom_level = 1.0;
        self.interpolation = false;
        
        if let Ok(mut r) = self.renderer.lock() {
            *r = None;
        }
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

    pub fn center_view(&mut self, model: &Model, sheet: &SpriteSheet) -> bool {
        if let Some((offset, _)) = center::calculate_center_offset(model, self.current_anim.as_ref(), sheet) {
            self.pan_offset = offset;
            true
        } else {
            false
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, sprite_sheet: &SpriteSheet, model: &Model) {
        let dt = ui.input(|i| i.stable_dt);

        // 1. DETECT NEW UNIT
        if self.loaded_id != self.last_loaded_id {
            self.last_loaded_id = self.loaded_id.clone();
            self.pending_initial_center = true;
        }

        // 2. RETRY LOOP FOR CENTERING
        if self.pending_initial_center {
            if !model.parts.is_empty() && self.current_anim.is_some() {
                if self.center_view(model, sprite_sheet) {
                    self.pending_initial_center = false;
                }
            }
        }

        // 3. SMOOTH ZOOM INTERPOLATION
        let diff = self.target_zoom_level - self.zoom_level;
        if diff.abs() > 0.001 {
            self.zoom_level += diff * 15.0 * dt;
        } else {
            self.zoom_level = self.target_zoom_level;
        }

        // 4. ANIMATION PLAYBACK
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

        // 5. ALLOCATE CANVAS
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        
        // 6. DELEGATE INPUT TO CONTROLS
        controls::handle_viewport_input(
            ui, 
            &response, 
            &mut self.pan_offset, 
            &mut self.zoom_level, 
            &mut self.target_zoom_level, 
            &mut self.pending_initial_center
        );

        // 7. SOLVE ANIMATION HIERARCHY
        let parts_to_draw = if let Some(anim) = &self.current_anim {
            let frame = if self.interpolation { self.current_frame } else { (self.current_frame + 0.01).floor() };
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

        // 8. RENDER CANVAS
        canvas::paint(ui, rect, self.renderer.clone(), sheet_arc, parts_to_draw, self.pan_offset, self.zoom_level);

        // 9. DRAW OVERLAYS
        let border_rect = rect.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165); 
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));

        if self.debug_show_info {
            let clip_rect = rect.shrink(4.0);
            let painter = ui.painter().with_clip_rect(clip_rect);
            
            let cx = rect.min.x + rect.width() / 2.0;
            let cy = rect.min.y + rect.height() / 2.0;
            
            let origin_x = cx + self.pan_offset.x * self.zoom_level;
            let origin_y = cy + self.pan_offset.y * self.zoom_level; 
            let origin_pos = egui::pos2(origin_x, origin_y);

            painter.line_segment([origin_pos - egui::vec2(15.0, 0.0), origin_pos + egui::vec2(15.0, 0.0)], egui::Stroke::new(2.0, egui::Color32::GREEN));
            painter.line_segment([origin_pos - egui::vec2(0.0, 15.0), origin_pos + egui::vec2(0.0, 15.0)], egui::Stroke::new(2.0, egui::Color32::GREEN));
            
            if let Some((_, bound_rect)) = center::calculate_center_offset(model, self.current_anim.as_ref(), sprite_sheet) {
                if bound_rect.width() > 0.0 {
                    let world_min_y = -bound_rect.min.y;
                    let world_max_y = -bound_rect.max.y;
                    let (final_min_y, final_max_y) = if world_min_y < world_max_y { (world_min_y, world_max_y) } else { (world_max_y, world_min_y) };

                    let min_x = cx + (bound_rect.min.x + self.pan_offset.x) * self.zoom_level;
                    let max_x = cx + (bound_rect.max.x + self.pan_offset.x) * self.zoom_level;
                    let min_y = cy + (final_min_y + self.pan_offset.y) * self.zoom_level;
                    let max_y = cy + (final_max_y + self.pan_offset.y) * self.zoom_level;

                    let screen_rect = egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y));
                    
                    painter.rect_stroke(screen_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::RED));
                    painter.text(screen_rect.min, egui::Align2::LEFT_BOTTOM, "Calculated Bounds (Frame 0)", egui::FontId::proportional(12.0), egui::Color32::RED);
                }
            }
        }
    }
}