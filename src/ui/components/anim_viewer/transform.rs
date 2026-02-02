use eframe::egui;
use std::collections::HashMap;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    // 2x3 Matrix [a, b, c, d, tx, ty]
    pub mat: [f32; 6], 
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
    
    // 1. Parent Matrix
    let parent_mat = if part.parent_id >= 0 && (part.parent_id as usize) < parts.len() && (part.parent_id as usize) != index {
        solve_bone(part.parent_id as usize, parts, model, cache).mat
    } else {
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    };

    let parent_opacity = if part.parent_id >= 0 && (part.parent_id as usize) < parts.len() && (part.parent_id as usize) != index {
        cache.get(&(part.parent_id as usize)).map(|t| t.opacity).unwrap_or(1.0)
    } else {
        1.0
    };

    let is_hidden = part.unit_id == -1;

    // 2. Local Transform
    let sx = part.scale_x / model.scale_unit;
    let sy = part.scale_y / model.scale_unit;
    
    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };
    let rotation = (part.rotation / rot_div).to_radians();
    let (sin, cos) = rotation.sin_cos();

    // User Fix: Positive Y
    let tx = part.position_x;
    let ty = part.position_y; 

    // 3. Local Matrix (Scale -> Rotate -> Translate)
    let la = sx * cos;
    let lb = sx * sin;
    let lc = -sy * sin;
    let ld = sy * cos;
    
    // 4. Multiply (Parent * Local)
    let pa = parent_mat[0]; let pc = parent_mat[2]; let ptx = parent_mat[4];
    let pb = parent_mat[1]; let pd = parent_mat[3]; let pty = parent_mat[5];

    let final_mat = [
        pa * la + pc * lb,       
        pb * la + pd * lb,       
        pa * lc + pc * ld,       
        pb * lc + pd * ld,       
        pa * tx + pc * ty + ptx, 
        pb * tx + pd * ty + pty  
    ];

    let alpha_norm = part.alpha / model.alpha_unit;
    let final_opacity = parent_opacity * alpha_norm;

    let result = WorldTransform {
        mat: final_mat,
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