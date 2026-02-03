use eframe::egui;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::core::anim::{animator, transform};

/// Calculates the camera offset.
/// Returns None if the calculation fails (e.g., missing animation OR missing textures).
pub fn calculate_center_offset(
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet
) -> Option<(egui::Vec2, egui::Rect)> {
    
    // 1. Strict Fallback: Need Animation
    let anim_ref = match anim {
        Some(a) => a,
        None => return None, // Return None so we retry later
    };

    // 2. Frame 0 Calculation
    let animated = animator::animate(model, anim_ref, 0.0);
    let parts = transform::solve_hierarchy(&animated, model);

    let mut total_weight = 0.0;
    let mut weighted_sum_x = 0.0;
    let mut weighted_sum_y = 0.0;
    
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    
    let mut found_any = false;

    for part in parts {
        if part.hidden || part.opacity <= 0.01 { continue; }
        
        // CRITICAL CHECK: If textures aren't loaded yet, this returns None.
        let cut = match sheet.cuts_map.get(&part.sprite_index) {
            Some(c) => c,
            None => continue,
        };
        
        let w = cut.original_size.x;
        let h = cut.original_size.y;
        
        // Skip logic markers
        if w * h <= 16.0 { continue; }

        let px = part.pivot.x;
        let py = part.pivot.y;
        let local_corners = [
            egui::vec2(-px, py),
            egui::vec2(w - px, py),
            egui::vec2(-px, py - h),
            egui::vec2(w - px, py - h),
        ];
        let (sin, cos) = part.rotation.sin_cos();
        let sx = part.scale.x;
        let sy = part.scale.y;

        for local in local_corners {
            let x = local.x * sx * cos - local.y * sx * sin + part.pos.x;
            let y = local.x * sy * sin + local.y * sy * cos + part.pos.y;
            if x < min.x { min.x = x; }
            if x > max.x { max.x = x; }
            if y < min.y { min.y = y; }
            if y > max.y { max.y = y; }
        }
        found_any = true;

        let scale_area = (sx.abs() * w) * (sy.abs() * h);
        let weight = scale_area * part.opacity;
        let local_mid_x = w / 2.0 - px;
        let local_mid_y = py - h / 2.0;
        let center_x = local_mid_x * sx * cos - local_mid_y * sx * sin + part.pos.x;
        let center_y = local_mid_x * sy * sin + local_mid_y * sy * cos + part.pos.y;

        weighted_sum_x += center_x * weight;
        weighted_sum_y += center_y * weight;
        total_weight += weight;
    }

    if !found_any {
        // Textures probably haven't loaded yet.
        // Return None so 'anim.rs' keeps the pending flag true.
        return None;
    }

    // Calculate Final Offset
    let offset = if total_weight > 0.0 {
        let final_x = weighted_sum_x / total_weight;
        let final_y = weighted_sum_y / total_weight;
        egui::vec2(-final_x, final_y)
    } else {
        egui::vec2(-((min.x + max.x) / 2.0), (min.y + max.y) / 2.0)
    };

    let rect = egui::Rect::from_min_max(min, max);

    Some((offset, rect))
}