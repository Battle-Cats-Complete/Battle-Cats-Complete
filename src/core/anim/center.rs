use eframe::egui;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use super::animator;
use super::transform;

/// Calculates the Pan Offset and Bounding Box for the unit.
/// 
/// Returns:
/// - `Vec2`: The Pan offset required to center the unit (inverted position).
/// - `Rect`: The "Tight" bounding box of the unit (used for zoom fitting).
pub fn calculate_center_offset(
    model: &Model, 
    anim: Option<&Animation>, 
    sheet: &SpriteSheet
) -> Option<(egui::Vec2, egui::Rect)> {
    
    // 1. Simulate Frame 0 (Resting Pose)
    let parts = if let Some(animation) = anim {
        animator::animate(model, animation, 0.0)
    } else {
        model.parts.clone()
    };

    // 2. Solve Hierarchy
    let world_parts = transform::solve_hierarchy(&parts, model);

    // --- ZOOM HANDLING: FILTERING STRATEGY ---
    // We want to calculate the bounds of the "Solid Body" to determine the perfect zoom.
    // Particle effects (Glows) and transparent bits (Opacity < 0.2) often extend way 
    // beyond the unit, causing the camera to zoom out too far.
    // Strategy: Try to calculate bounds on "Solid" parts first. If none exist, fallback to everything.
    
    // Pass 1: Strict Filter (Ignore Glows & Faint Trails)
    let result = calculate_bounds_and_center(&world_parts, sheet, true);
    
    if result.is_some() {
        return result;
    }
    
    // Pass 2: Fallback (Unit might be a ghost or pure effect, use loose filter)
    calculate_bounds_and_center(&world_parts, sheet, false)
}

/// Helper to calculate the zoom level needed to fit 'bounds' into 'viewport_size'.
/// padding: 0.8 = 80% of screen filled (good default).
pub fn calculate_zoom_fit(bounds: egui::Rect, viewport_size: egui::Vec2, padding: f32) -> f32 {
    if bounds.width() <= 1.0 || bounds.height() <= 1.0 {
        return 1.0; 
    }

    // Calculate scale required for X and Y axes
    let scale_x = viewport_size.x / bounds.width();
    let scale_y = viewport_size.y / bounds.height();

    // Use the smaller scale (to fit the whole object)
    // Clamp to prevent infinite zoom on empty units or microscopic zoom on huge ones
    scale_x.min(scale_y).clamp(0.05, 5.0) * padding
}

fn calculate_bounds_and_center(
    world_parts: &[transform::WorldTransform],
    sheet: &SpriteSheet,
    strict_filter: bool
) -> Option<(egui::Vec2, egui::Rect)> {

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    
    let mut weighted_sum_x = 0.0;
    let mut weighted_sum_y = 0.0;
    let mut total_weight = 0.0;
    
    let mut found = false;

    for part in world_parts {
        if part.hidden { continue; }

        // --- VISIBILITY FILTERS ---
        if strict_filter {
            // ZOOM FIT FILTER:
            // 1. Ignore low opacity (faint trails, shadows)
            if part.opacity < 0.2 { continue; }
            // 2. Ignore Glow/Additive effects (Particles/Auras/Explosions)
            //    These are often huge and shouldn't dictate the camera zoom.
            if part.glow > 0 { continue; }
        } else {
            // FALLBACK FILTER: Just skip invisible stuff
            if part.opacity <= 0.01 { continue; }
        }

        if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
            let w = cut.original_size.x;
            let h = cut.original_size.y;
            let px = part.pivot.x;
            let py = part.pivot.y;

            // --- A. Bounding Box (Matrix Logic) ---
            // Calculates the "Tight Bounds" for the zoom to fit into.
            let local_corners = [
                egui::vec2(-px, -py),        
                egui::vec2(w - px, -py),     
                egui::vec2(-px, h - py),     
                egui::vec2(w - px, h - py)   
            ];

            for p in local_corners {
                let (wx, wy) = transform_point(p.x, p.y, &part.matrix);
                if wx < min_x { min_x = wx; }
                if wx > max_x { max_x = wx; }
                if wy < min_y { min_y = wy; }
                if wy > max_y { max_y = wy; }
            }

            // --- B. Weighted Center (Exact Legacy Logic) ---
            let sx = (part.matrix[0].powi(2) + part.matrix[1].powi(2)).sqrt();
            let sy = (part.matrix[3].powi(2) + part.matrix[4].powi(2)).sqrt();
            let tx = part.matrix[6];
            let ty = part.matrix[7];

            let rot = part.matrix[1].atan2(part.matrix[0]);
            let cos = rot.cos();
            let sin = rot.sin();

            let local_mid_x = w / 2.0 - px;
            let local_mid_y = py - h / 2.0; 

            let center_x = local_mid_x * sx * cos - local_mid_y * sx * sin + tx;
            let center_y = local_mid_x * sy * sin + local_mid_y * sy * cos + ty;

            let area = (w * sx) * (h * sy);
            let weight = area * part.opacity;

            weighted_sum_x += center_x * weight;
            weighted_sum_y += center_y * weight;
            total_weight += weight;
            
            found = true;
        }
    }

    if !found { return None; }

    let width = max_x - min_x;
    let height = max_y - min_y;

    // Calculate the Center Point of the unit
    let (focus_x, focus_y) = if total_weight > 0.001 {
        (weighted_sum_x / total_weight, weighted_sum_y / total_weight)
    } else {
        (min_x + width / 2.0, min_y + height / 2.0)
    };

    // Return (Offset, Tight Bounding Box)
    // NOTE: To center the camera, we need the NEGATIVE of the position.
    // If unit is at (100, 100), we must Pan (-100, -100) to bring it to (0,0).
    // Y is inverted because Battle Cats uses Y-Down, but UI might expect standard.
    // We return -focus_x and -focus_y to correctly center it.
    Some((
        egui::vec2(-focus_x, -focus_y),
        egui::Rect::from_min_max(
            egui::pos2(min_x, min_y), 
            egui::pos2(max_x, max_y)
        )
    ))
}

fn transform_point(x: f32, y: f32, m: &[f32; 9]) -> (f32, f32) {
    let nx = x * m[0] + y * m[3] + m[6];
    let ny = x * m[1] + y * m[4] + m[7];
    (nx, ny)
}