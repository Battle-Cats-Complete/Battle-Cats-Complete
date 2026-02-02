use eframe::egui;
use std::collections::HashMap;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    // [a, b, c, d, tx, ty]
    pub mat: [f32; 6], 
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub part_index: usize, // Needed for Tie-Breaker
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
    
    // FIX: Stable Sorting (Z-Order -> Part Index)
    // Ensures parts on the same layer don't flicker or disappear.
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });
    
    results
}

fn solve_bone(index: usize, parts: &[ModelPart], model: &Model, cache: &mut HashMap<usize, WorldTransform>) -> WorldTransform {
    if let Some(cached) = cache.get(&index) { return *cached; }

    let part = &parts[index];
    
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

    // FIX: Visibility uses Sprite Index. Unit ID -1 is valid for generic parts.
    let is_hidden = part.sprite_index == -1;

    let sx = part.scale_x / model.scale_unit;
    let sy = part.scale_y / model.scale_unit;
    
    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };
    let rotation = (part.rotation / rot_div).to_radians();
    let (sin, cos) = rotation.sin_cos();

    let tx = part.position_x;
    let ty = part.position_y; 

    // Matrix Construction
    let la = sx * cos;
    let lb = sx * sin;
    let lc = -sy * sin;
    let ld = sy * cos;
    
    // Matrix Multiplication
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
        part_index: index,
        pivot: egui::vec2(part.pivot_x, part.pivot_y),
        hidden: is_hidden,
        glow: part.glow_mode,
    };

    cache.insert(index, result);
    result
}