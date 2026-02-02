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
}

// Internal struct to match the JS "vectors" array
#[derive(Clone, Copy)]
struct TransformStep {
    child_pos: [f32; 2],      // [data.x, -data.y]
    parent_scale: [f32; 2],   // [parent.scaleX, parent.scaleY]
    parent_rot: [f32; 4],     // Rotation matrix of parent
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut results = Vec::with_capacity(parts.len());

    for (i, _) in parts.iter().enumerate() {
        results.push(solve_single_part(i, parts, model));
    }
    
    // Sort by Z-Index
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
    });
    
    results
}

fn solve_single_part(index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[index];
    
    // 1. Build Chain
    let mut chain: Vec<usize> = Vec::new();
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
    
    // 2. Prepare Vectors
    let mut vectors = Vec::with_capacity(chain.len());
    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };

    for &idx in &chain {
        let p = &parts[idx];
        
        // POS: JS uses [x, -y].
        let pos = [p.position_x, -p.position_y];
        
        // SCALE
        let sx = p.scale_x / model.scale_unit;
        let sy = p.scale_y / model.scale_unit;
        let scale = [sx, sy];

        // ROT: JS uses degToRad. 
        // JS Rotation Matrix: [cos, sin, -sin, cos]
        // This effectively encodes a Clockwise rotation (or inverted axis rotation).
        let rot_rad = (p.rotation / rot_div).to_radians();
        let (sin, cos) = rot_rad.sin_cos();
        let rot_matrix = [cos, sin, -sin, cos];

        vectors.push(TransformStep {
            child_pos: pos,
            parent_scale: scale,
            parent_rot: rot_matrix,
        });
    }
    
    // 3. Apply Transforms Iteratively (updateSprites Algorithm)
    let len = vectors.len();
    
    // Loop 1: Apply Scales
    for j in 0..len {
        let scale = vectors[j].parent_scale;
        for k in j..len {
            if k > j { 
                 vectors[k].child_pos[0] *= scale[0];
                 vectors[k].child_pos[1] *= scale[1];
            }
        }
    }

    // Loop 2: Apply Rotations and Sum Positions
    let mut g_pos = egui::Vec2::ZERO;

    for j in 0..len {
        let rot = vectors[j].parent_rot; // [cos, sin, -sin, cos]
        
        for l in j..len {
            if l > j {
                // Apply JS "applyMatrix": [m0*x + m1*y, m2*x + m3*y]
                // m0=cos, m1=sin, m2=-sin, m3=cos
                // x' = x*cos + y*sin
                // y' = x*-sin + y*cos
                // This rotates Clockwise.
                let x = vectors[l].child_pos[0];
                let y = vectors[l].child_pos[1];
                
                let nx = x * rot[0] + y * rot[1];
                let ny = x * rot[2] + y * rot[3];
                
                vectors[l].child_pos[0] = nx;
                vectors[l].child_pos[1] = ny;
            }
        }
        
        // Sum Vectors
        g_pos.x += vectors[j].child_pos[0];
        g_pos.y += vectors[j].child_pos[1];
    }

    // --- Global Properties Accumulation ---
    let mut g_scale = egui::vec2(1.0, 1.0);
    let mut g_rot = 0.0;
    let mut g_op = 1.0;
    
    for &idx in &chain {
        let p = &parts[idx];
        g_scale.x *= p.scale_x / model.scale_unit;
        g_scale.y *= p.scale_y / model.scale_unit;
        g_rot += (p.rotation / rot_div).to_radians();
        g_op *= p.alpha / model.alpha_unit;
    }

    WorldTransform {
        pos: g_pos,
        scale: g_scale,
        rotation: g_rot,
        opacity: g_op,
        z_order: target_part.drawing_layer,
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x, target_part.pivot_y),
        hidden: target_part.sprite_index == -1,
        glow: target_part.glow_mode,
    }
}