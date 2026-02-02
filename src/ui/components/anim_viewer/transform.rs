use eframe::egui;
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
    pub part_index: usize, // Needed for stable sort
}

#[derive(Clone, Copy)]
struct TransformStep {
    child_pos: [f32; 2],      
    parent_scale: [f32; 2],   
    parent_rot: [f32; 4],     
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut results = Vec::with_capacity(parts.len());

    for (i, _) in parts.iter().enumerate() {
        results.push(solve_single_part(i, parts, model));
    }
    
    // FIX: Add part_index as tie-breaker for stable sorting.
    // Prevents "jittering when they should be still" caused by Z-fighting.
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });
    
    results
}

fn solve_single_part(index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[index];
    
    let mut chain = Vec::new();
    let mut curr = index;
    chain.push(curr);
    
    let mut safety = 0;
    while parts[curr].parent_id != -1 && safety < 100 {
        curr = parts[curr].parent_id as usize;
        if curr >= parts.len() { break; }
        chain.push(curr);
        safety += 1;
    }
    chain.reverse();
    
    let mut vectors = Vec::with_capacity(chain.len());
    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };
    
    let mut acc_flip_x = 1.0;
    let mut acc_flip_y = 1.0;

    for &idx in &chain {
        let p = &parts[idx];
        let pos = [p.position_x, -p.position_y]; 
        let sx = p.scale_x / model.scale_unit;
        let sy = p.scale_y / model.scale_unit;
        
        let current_flip_x = sx.signum();
        let current_flip_y = sy.signum();
        let total_flip_x = acc_flip_x * current_flip_x;
        let total_flip_y = acc_flip_y * current_flip_y;
        
        let rot_rad = (p.rotation / rot_div).to_radians();
        let adjusted_rot = rot_rad * total_flip_x * total_flip_y;
        
        let (sin, cos) = adjusted_rot.sin_cos();
        let rot_matrix = [cos, sin, -sin, cos];

        vectors.push(TransformStep {
            child_pos: pos,
            parent_scale: [sx, sy],
            parent_rot: rot_matrix,
        });
        
        acc_flip_x *= current_flip_x;
        acc_flip_y *= current_flip_y;
    }
    
    let len = vectors.len();
    
    for j in 0..len {
        let scale = vectors[j].parent_scale;
        for k in j..len {
            if k > j { 
                 vectors[k].child_pos[0] *= scale[0];
                 vectors[k].child_pos[1] *= scale[1];
            }
        }
    }

    let mut g_pos = egui::Vec2::ZERO;
    for j in 0..len {
        let rot = vectors[j].parent_rot; 
        for l in j..len {
            if l > j {
                let x = vectors[l].child_pos[0];
                let y = vectors[l].child_pos[1];
                let nx = x * rot[0] + y * rot[1];
                let ny = x * rot[2] + y * rot[3];
                vectors[l].child_pos[0] = nx;
                vectors[l].child_pos[1] = ny;
            }
        }
        g_pos.x += vectors[j].child_pos[0];
        g_pos.y += vectors[j].child_pos[1];
    }

    let mut g_scale = egui::vec2(1.0, 1.0);
    let mut g_rot = 0.0;
    let mut g_op = 1.0;
    
    acc_flip_x = 1.0;
    acc_flip_y = 1.0;

    for &idx in &chain {
        let p = &parts[idx];
        let sx = p.scale_x / model.scale_unit;
        let sy = p.scale_y / model.scale_unit;
        
        g_scale.x *= sx;
        g_scale.y *= sy;
        
        let current_flip_x = sx.signum();
        let current_flip_y = sy.signum();
        
        let local_r = (p.rotation / rot_div).to_radians();
        g_rot += local_r * acc_flip_x * current_flip_x * acc_flip_y * current_flip_y;
        
        g_op *= p.alpha / model.alpha_unit;
        
        acc_flip_x *= current_flip_x;
        acc_flip_y *= current_flip_y;
    }

    let is_ghost = target_part.unit_id == -1 || target_part.sprite_index == -1;

    WorldTransform {
        pos: g_pos,
        scale: g_scale,
        rotation: g_rot,
        opacity: g_op,
        z_order: target_part.drawing_layer,
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x, target_part.pivot_y),
        hidden: is_ghost,
        glow: target_part.glow_mode,
        part_index: index, // Correctly assigned
    }
}