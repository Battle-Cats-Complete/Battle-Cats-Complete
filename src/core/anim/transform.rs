use eframe::egui;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    // 3x3 Matrix (Column-Major)
    pub matrix: [f32; 9], 
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub pivot: egui::Vec2,
    pub hidden: bool,
    pub glow: i32,
    pub part_index: usize, 
}

// Temporary struct to hold hierarchy steps
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
    
    // STABLE SORT (Painter's Algorithm)
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });

    results
}

fn solve_single_part(target_index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[target_index];
    
    // 1. Build Parent Chain
    let mut chain = Vec::new();
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
    chain.reverse(); 

    // 2. Logic: Y-Down Coords + Explicit Flip Application
    let mut vectors = Vec::with_capacity(chain.len());
    let rot_div = if model.angle_unit != 0.0 { model.angle_unit / 360.0 } else { 1.0 };
    
    let mut acc_flip_x = 1.0;
    let mut acc_flip_y = 1.0;

    for &idx in &chain {
        let p = &parts[idx];
        
        // Pos: Standard (Y-Down)
        let pos = [p.position_x, p.position_y]; 
        
        let sx = p.scale_x / model.scale_unit;
        let sy = p.scale_y / model.scale_unit;
        
        let current_flip_x = if p.flip_x { -1.0 } else { 1.0 };
        let current_flip_y = if p.flip_y { -1.0 } else { 1.0 };
        
        // HIERARCHY SCALE: Apply Flip to mirror Child Positions.
        // Since 'sx' is now clean (always positive size), we multiply by flip here.
        let hierarchy_sx = sx * current_flip_x;
        let hierarchy_sy = sy * current_flip_y;
        
        let total_flip_x = acc_flip_x * current_flip_x;
        let total_flip_y = acc_flip_y * current_flip_y;
        
        let rot_rad = (p.rotation / rot_div).to_radians();
        
        // ROTATION: Negative for Y-Down
        let adjusted_rot = -rot_rad * total_flip_x * total_flip_y;
        
        let (sin, cos) = adjusted_rot.sin_cos();
        let rot_matrix = [cos, sin, -sin, cos];

        vectors.push(TransformStep {
            child_pos: pos,
            parent_scale: [hierarchy_sx, hierarchy_sy], 
            parent_rot: rot_matrix,
        });
        
        acc_flip_x *= current_flip_x;
        acc_flip_y *= current_flip_y;
    }
    
    // B. Solve Position
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

    // C. Visual Scale & Rotation
    let mut g_scale = egui::vec2(1.0, 1.0);
    let mut g_rot = 0.0;
    let mut g_op = 1.0;
    
    acc_flip_x = 1.0;
    acc_flip_y = 1.0;

    for &idx in &chain {
        let p = &parts[idx];
        let sx = p.scale_x / model.scale_unit;
        let sy = p.scale_y / model.scale_unit;
        
        // Scale Accumulation
        g_scale.x *= sx;
        g_scale.y *= sy;
        
        let current_flip_x = if p.flip_x { -1.0 } else { 1.0 };
        let current_flip_y = if p.flip_y { -1.0 } else { 1.0 };
        
        let local_r = (p.rotation / rot_div).to_radians();
        
        // Rotation Accumulation
        g_rot += -local_r * acc_flip_x * current_flip_x * acc_flip_y * current_flip_y;
        
        g_op *= p.alpha / model.alpha_unit;
        
        acc_flip_x *= current_flip_x;
        acc_flip_y *= current_flip_y;
    }
    
    // VISUAL FLIP: Apply the final accumulated flip to the scale.
    // This ensures the Sprite flips visually (fixing the Torso Jumble).
    g_scale.x *= acc_flip_x;
    g_scale.y *= acc_flip_y;

    // 3. Final Matrix
    let c = g_rot.cos();
    let s = g_rot.sin();
    
    let matrix = [
        g_scale.x * c,   g_scale.x * -s,  0.0, 
        g_scale.y * s,   g_scale.y * c,   0.0, 
        g_pos.x,         g_pos.y,         1.0  
    ];

    WorldTransform {
        matrix,
        opacity: g_op,
        z_order: target_part.drawing_layer, 
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x, target_part.pivot_y),
        hidden: g_op < 0.001 || target_part.unit_id == -1 || target_part.sprite_index == -1,
        glow: target_part.glow_mode,
        part_index: target_index,
    }
}