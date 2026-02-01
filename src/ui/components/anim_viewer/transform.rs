use eframe::egui;
use std::collections::HashMap;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    pub pos: egui::Vec2,
    pub scale: egui::Vec2,
    pub rotation: f32,
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub pivot: egui::Vec2,
    pub hidden: bool,
    pub glow: i32,
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut cache: HashMap<usize, WorldTransform> = HashMap::new();
    let mut results: Vec<WorldTransform> = Vec::new();

    for i in 0..parts.len() {
        results.push(solve_bone(i, parts, model, &mut cache));
    }
    
    results.sort_by_key(|t| t.z_order);
    results
}

fn solve_bone(index: usize, parts: &[ModelPart], model: &Model, cache: &mut HashMap<usize, WorldTransform>) -> WorldTransform {
    if let Some(cached) = cache.get(&index) { return *cached; }

    let part = &parts[index];
    
    let parent = if part.parent_id >= 0 && (part.parent_id as usize) < parts.len() && (part.parent_id as usize) != index {
        solve_bone(part.parent_id as usize, parts, model, cache)
    } else {
        WorldTransform {
            pos: egui::Vec2::ZERO,
            scale: egui::Vec2::splat(1.0),
            rotation: 0.0,
            opacity: 1.0,
            z_order: 0,
            sprite_index: 0,
            pivot: egui::Vec2::ZERO,
            hidden: false,
            glow: 0,
        }
    };

    let is_hidden = part.unit_id == -1;

    let sx = part.scale_x / model.scale_unit;
    let sy = part.scale_y / model.scale_unit;
    let local_scale = egui::vec2(sx, sy);

    // FIX: Removed the "-y" inversion. Trusting the data as Y-Down (Screen Space).
    // This often fixes the "Below Origin" issue if the file used positive Y for "Down".
    let local_pos = egui::vec2(part.position_x, part.position_y);

    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };
    let local_rot = (part.rotation / rot_div).to_radians();

    let (sin, cos) = parent.rotation.sin_cos();
    
    let scaled_pos = local_pos * parent.scale;
    
    let rot_x = scaled_pos.x * cos - scaled_pos.y * sin;
    let rot_y = scaled_pos.x * sin + scaled_pos.y * cos;
    
    let final_pos = parent.pos + egui::vec2(rot_x, rot_y);
    let final_scale = parent.scale * local_scale;
    let final_rot = parent.rotation + local_rot;
    
    let alpha_norm = part.alpha / model.alpha_unit;
    let final_opacity = parent.opacity * alpha_norm;

    let result = WorldTransform {
        pos: final_pos,
        scale: final_scale,
        rotation: final_rot,
        opacity: final_opacity,
        z_order: part.drawing_layer,
        sprite_index: part.sprite_index as usize,
        pivot: egui::vec2(part.pivot_x, part.pivot_y),
        hidden: is_hidden,
        glow: part.glow_mode,
    };

    cache.insert(index, result);
    result
}