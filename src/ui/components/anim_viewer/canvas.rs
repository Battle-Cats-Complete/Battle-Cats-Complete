use eframe::egui;
use crate::data::global::imgcut::SpriteSheet;
use super::transform::WorldTransform;

pub fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    sheet: &SpriteSheet,
    parts: &[WorldTransform],
    pan: egui::Vec2,
    zoom: f32
) {
    painter.rect_filled(rect, egui::Rounding::ZERO, egui::Color32::from_gray(20));
    let center = rect.center() + pan;
    
    painter.line_segment(
        [center - egui::vec2(200.0, 0.0), center + egui::vec2(200.0, 0.0)], 
        egui::Stroke::new(1.0, egui::Color32::GREEN)
    );
    
    let texture_id = match &sheet.texture_handle {
        Some(t) => t.id(),
        None => return,
    };

    for part in parts {
        // Culling Check:
        // 1. Explicitly hidden (from transform.rs)
        // 2. Opacity near zero
        // 3. Scale near zero (common "hide" trick in animations)
        if part.hidden 
           || part.opacity < 0.005 
           || part.scale.x.abs() < 0.001 
           || part.scale.y.abs() < 0.001 
        { 
            continue; 
        }

        if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
            let w = cut.original_size.x;
            let h = cut.original_size.y;
            let px = part.pivot.x;
            let py = part.pivot.y;

            // Quad construction (Y-Up logic)
            // TL: (-px, py)
            // TR: (w-px, py) ...
            let corners = [
                egui::vec2(0.0 - px, py),       
                egui::vec2(w - px,   py),       
                egui::vec2(w - px,   py - h),   
                egui::vec2(0.0 - px, py - h),   
            ];

            let mut screen_corners = [egui::Pos2::ZERO; 4];
            let (sin, cos) = part.rotation.sin_cos();

            for i in 0..4 {
                let lx = corners[i].x;
                let ly = corners[i].y;

                // 1. Scale
                let sx = lx * part.scale.x;
                let sy = ly * part.scale.y;

                // 2. Rotate
                // Match JS Logic: x' = x*cos + y*sin, y' = x*-sin + y*cos
                let rx = sx * cos + sy * sin;
                let ry = sx * -sin + sy * cos;

                // 3. Translate
                let world_x = part.pos.x + rx;
                let world_y = part.pos.y + ry;

                // 4. Project (Flip Y)
                screen_corners[i] = center + egui::vec2(world_x * zoom, -world_y * zoom);
            }

            let mut mesh = egui::Mesh::with_texture(texture_id);
            let color = egui::Color32::WHITE.gamma_multiply(part.opacity);
            let uv = cut.uv_coordinates;

            mesh.vertices.push(egui::epaint::Vertex { pos: screen_corners[0], uv: uv.left_top(), color });
            mesh.vertices.push(egui::epaint::Vertex { pos: screen_corners[1], uv: uv.right_top(), color });
            mesh.vertices.push(egui::epaint::Vertex { pos: screen_corners[2], uv: uv.right_bottom(), color });
            mesh.vertices.push(egui::epaint::Vertex { pos: screen_corners[3], uv: uv.left_bottom(), color });
            
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);

            painter.add(mesh);
        }
    }
}