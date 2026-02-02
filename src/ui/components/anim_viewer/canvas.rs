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
    let center = rect.center() + pan + egui::vec2(0.0, 200.0);
    
    painter.line_segment(
        [center - egui::vec2(200.0, 0.0), center + egui::vec2(200.0, 0.0)], 
        egui::Stroke::new(1.0, egui::Color32::GREEN)
    );
    
    let texture_id = match &sheet.texture_handle {
        Some(t) => t.id(),
        None => return,
    };

    for part in parts {
        if part.hidden || part.opacity < 0.01 { continue; }

        if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
            let w = cut.original_size.x;
            let h = cut.original_size.y;
            let px = part.pivot.x;
            let py = part.pivot.y;

            // Local Corners (Top-Left is origin for image, so subtract pivot)
            let local_corners = [
                egui::vec2(0.0 - px, 0.0 - py), 
                egui::vec2(w - px,   0.0 - py), 
                egui::vec2(w - px,   h - py),   
                egui::vec2(0.0 - px, h - py),   
            ];

            let mut screen_corners = [egui::Pos2::ZERO; 4];
            let m = part.mat; 

            for i in 0..4 {
                let lx = local_corners[i].x;
                let ly = local_corners[i].y;

                // Affine Transform
                let world_x = m[0] * lx + m[2] * ly + m[4];
                let world_y = m[1] * lx + m[3] * ly + m[5];

                screen_corners[i] = center + egui::vec2(world_x * zoom, world_y * zoom);
            }

            let mut mesh = egui::Mesh::with_texture(texture_id);
            let color = egui::Color32::WHITE.gamma_multiply(part.opacity);
            let uv = cut.uv_coordinates;

            // FIX: Using egui::epaint::Vertex to avoid compilation errors
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