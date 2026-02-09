use eframe::egui;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    // 3x3 Matrix (Column-Major) - Output to GPU stays f32
    pub matrix: [f32; 9], 
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub pivot: egui::Vec2,
    pub hidden: bool,
    pub glow: i32,
    pub part_index: usize, 
}

struct VectorRow {
    pos: [f64; 2],
    scale: [f64; 2], // [sx, sy]
    rot: [f64; 4],   // [cos, sin, -sin, cos]
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut results = Vec::with_capacity(parts.len());

    // Pre-calculate unit divisors (Switching to f64 for internal logic)
    // Fixed: Added .0 to literals to match f32/f64 types
    let scale_unit = if model.scale_unit == 0.0 { 1000.0 } else { model.scale_unit as f64 };
    let angle_unit = if model.angle_unit == 0.0 { 1000.0 } else { model.angle_unit as f64 };
    let alpha_unit = if model.alpha_unit == 0.0 { 100.0 } else { model.alpha_unit as f64 };

    for (i, _) in parts.iter().enumerate() {
        results.push(solve_single_part(i, parts, scale_unit, angle_unit, alpha_unit));
    }
    
    // Painter's Algorithm Sort (Z-Order)
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });

    results
}

fn solve_single_part(
    target_index: usize, 
    parts: &[ModelPart], 
    scale_unit: f64, 
    angle_unit: f64,
    alpha_unit: f64
) -> WorldTransform {
    let target_part = &parts[target_index];

    // 1. Build Ancestry Chain (Target -> Parent -> ... -> Root)
    let mut chain = Vec::with_capacity(16);
    let mut curr = target_index;
    let mut safety = 0;
    
    loop {
        chain.push(curr);
        let next_parent = parts[curr].parent_id;
        
        if next_parent == -1 { break; }
        if next_parent as usize == curr { break; } 
        
        curr = next_parent as usize;
        if curr >= parts.len() { break; }
        
        safety += 1;
        if safety > 100 { break; } 
    }
    
    let mut vectors: Vec<VectorRow> = Vec::with_capacity(chain.len());
    
    // Global Accumulators (Using f64)
    let mut g_scale_x = 1.0;
    let mut g_scale_y = 1.0;
    let mut g_angle = 0.0;
    let mut g_opacity = 1.0;
    let mut g_flip_x = 1.0;
    let mut g_flip_y = 1.0;

    // Iterate Root -> Target to build globals and vector list
    for i in (0..chain.len()).rev() {
        let idx = chain[i];
        let p = &parts[idx];

        // Normalize values using f64
        let raw_sx = p.scale_x as f64 / scale_unit;
        let raw_sy = p.scale_y as f64 / scale_unit;
        let raw_angle = p.rotation as f64 / angle_unit * 360.0; 
        
        // Accumulate Globals
        g_scale_x *= raw_sx;
        g_scale_y *= raw_sy;
        
        let flip_x_val = if p.flip_x { -1.0 } else { 1.0 };
        let flip_y_val = if p.flip_y { -1.0 } else { 1.0 };
        
        g_flip_x *= flip_x_val;
        g_flip_y *= flip_y_val;
        
        g_angle += raw_angle * g_flip_x * g_flip_y;
        
        // Fix: Use alpha_unit properly to avoid deepfried sprites
        g_opacity *= p.alpha as f64 / alpha_unit;
        
        // Build Vector Row
        if i < chain.len() - 1 {
            let parent_idx = chain[i+1];
            let parent = &parts[parent_idx];
            
            let p_sx = parent.scale_x as f64 / scale_unit;
            let p_sy = parent.scale_y as f64 / scale_unit;
            
            let p_fx = if parent.flip_x { -1.0 } else { 1.0 };
            let p_fy = if parent.flip_y { -1.0 } else { 1.0 };
            
            let p_angle_raw = parent.rotation as f64 / angle_unit * 360.0;
            let p_angle_rad = p_angle_raw.to_radians() * p_fx * p_fy;
            
            let (sin, cos) = p_angle_rad.sin_cos();
            
            // Revert: Restore the negative Y to match JS [[data.x, -data.y]]
            // Since canvas.rs has the negative projection, this should result in correct orientation.
            let pos_x = p.position_x as f64;
            let pos_y = -p.position_y as f64; 
            
            vectors.push(VectorRow {
                pos: [pos_x, pos_y],
                scale: [p_sx * p_fx, p_sy * p_fy], 
                rot: [cos, sin, -sin, cos],
            });
        }
    }

    // 2. The Vector Loop (Standard Matrix Accumulation)
    let len = vectors.len();
    
    // Apply Scale
    for j in 0..len {
        let s = vectors[j].scale;
        for k in j..len {
            vectors[k].pos[0] *= s[0];
            vectors[k].pos[1] *= s[1];
        }
    }
    
    // Apply Rotation & Accumulate
    let mut final_pos = [0.0, 0.0];
    
    for j in 0..len {
        let r = vectors[j].rot; 
        
        for l in j..len {
            let x = vectors[l].pos[0];
            let y = vectors[l].pos[1];
            
            let nx = x * r[0] + y * r[1];
            let ny = x * r[2] + y * r[3];
            
            vectors[l].pos[0] = nx;
            vectors[l].pos[1] = ny;
        }
        
        final_pos[0] += vectors[j].pos[0];
        final_pos[1] += vectors[j].pos[1];
    }
    
    // 3. Final Matrix Construction
    let sx = g_scale_x * g_flip_x;
    let sy = g_scale_y * g_flip_y;
    
    let rad = g_angle.to_radians();
    let c = rad.cos();
    let s = rad.sin();
    
    // Column-Major Layout: [sx*c, -sx*s, 0, sy*s, sy*c, 0, tx, ty, 1]
    let matrix = [
        (sx * c) as f32,       (-sx * s) as f32,      0.0,
        (sy * s) as f32,       (sy * c) as f32,       0.0,
        final_pos[0] as f32,   final_pos[1] as f32,   1.0,
    ];

    WorldTransform {
        matrix,
        // Clamp opacity to prevent deepfrying if units drift
        opacity: g_opacity.clamp(0.0, 1.0) as f32,
        z_order: target_part.drawing_layer, 
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x, target_part.pivot_y),
        hidden: target_part.unit_id == -1 || target_part.sprite_index == -1 || g_opacity < 0.001,
        glow: target_part.glow_mode,
        part_index: target_index,
    }
}