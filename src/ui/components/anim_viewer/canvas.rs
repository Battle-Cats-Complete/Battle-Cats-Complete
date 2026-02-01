use eframe::egui;
use crate::data::global::imgcut::SpriteSheet;
use super::transform::WorldTransform;

pub fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    sheet: &SpriteSheet,
    parts: &[WorldTransform],
    pan: egui::Vec2,
    zoom: f32,
    show_debug: bool
) {
    painter.rect_filled(rect, egui::Rounding::ZERO, egui::Color32::from_gray(20));
    let center = rect.center() + pan;
    
    // Origin Dot
    painter.circle_filled(center, 4.0, egui::Color32::GREEN);
    
    let texture_id = match &sheet.texture_handle {
        Some(t) => t.id(),
        None => return,
    };

    for (i, part) in parts.iter().enumerate() {
        if part.hidden || part.opacity < 0.01 { continue; }

        let idx = part.sprite_index;

        if let Some(cut) = sheet.cuts_map.get(&idx) {
            let final_scale = part.scale.abs() * zoom;
            let display_size = cut.original_size * final_scale;

            // POSITION LOGIC:
            // transform.rs now produces standard Screen Coordinates.
            // We just map (0,0) to Center.
            let screen_bone_pos = center + (part.pos * zoom);
            
            // PIVOT LOGIC:
            // Pivot is offset from Top-Left.
            let pivot_offset = part.pivot * final_scale;
            let screen_top_left = screen_bone_pos - pivot_offset;
            
            let dest_rect = egui::Rect::from_min_size(screen_top_left, display_size);
            
            let mut tint = egui::Color32::WHITE;
            if part.glow > 0 {
                tint = egui::Color32::from_rgb(200, 255, 255);
            }
            
            let final_alpha = if show_debug { 1.0 } else { part.opacity };
            
            // Note: painter.image does NOT rotate. 
            // If parts are rotated, this will look stiff, but it won't be a "cyclone".
            // To support rotation with this method, you would need the Mesh approach,
            // but you asked to revert to this version.
            painter.image(
                texture_id,
                dest_rect,
                cut.uv_coordinates,
                tint.gamma_multiply(final_alpha)
            );

            // Debug
            if show_debug && i == 0 {
                 let info = format!("Root:\nPos: {:.0},{:.0}", part.pos.x, part.pos.y);
                 painter.text(rect.min + egui::vec2(10.0, 50.0), egui::Align2::LEFT_TOP, info, egui::FontId::monospace(14.0), egui::Color32::YELLOW);
            }
        }
    }
}