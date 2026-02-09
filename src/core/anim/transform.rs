use eframe::egui;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    // 3x3 Matrix (Column-Major for OpenGL/Glow)
    // We keep this f32 because Glow/OpenGL requires f32 slices
    pub matrix: [f32; 9], 
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub pivot: egui::Vec2,
    pub hidden: bool,
    pub glow: i32,
    pub part_index: usize, 
}

// Represents the normalized local state of a part (similar to spritesNow objects in JS)
// Using f64 for calculations to match JS precision
#[derive(Clone, Copy, Debug)]
struct LocalState {
    x: f64,
    y: f64,
    scale_x: f64,
    scale_y: f64,
    angle: f64,   // Degrees (already normalized by unit)
    opacity: f64, // 0.0 - 1.0
    flip_x: f64,  // 1.0 or -1.0
    flip_y: f64,  // 1.0 or -1.0
}

// Represents the accumulated global state needed for hierarchy calculations
#[derive(Clone, Copy, Debug)]
struct GlobalState {
    // Note: Position is calculated separately via vectors, so not stored here
    scale_x: f64,
    scale_y: f64,
    angle: f64,   // Degrees
    flip_x: f64,
    flip_y: f64,
    opacity: f64,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            flip_x: 1.0,
            flip_y: 1.0,
            opacity: 1.0,
        }
    }
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut results = Vec::with_capacity(parts.len());

    for (i, _) in parts.iter().enumerate() {
        results.push(solve_single_part(i, parts, model));
    }
    
    // Stable sort by Z-order then ID (Painter's Algorithm)
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });

    results
}

fn solve_single_part(target_index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[target_index];
    
    // 1. Build Parent Chain: Target -> Parent -> ... -> Root
    let mut chain = Vec::new();
    let mut curr = target_index;
    let mut safety = 0;
    
    loop {
        chain.push(curr);
        let next_parent = parts[curr].parent_id;
        
        // Root check
        if next_parent == -1 { break; }
        // Cycle check
        if next_parent as usize == curr { break; } 
        
        curr = next_parent as usize;
        // Bounds check
        if curr >= parts.len() { break; }
        
        safety += 1;
        if safety > 100 { break; } // Prevent infinite loops
    }
    
    // Chain is now [Target, Parent, ..., Root]
    // We often need to iterate from Root -> Target, so let's reverse iteration where needed.

    // 2. Accumulate Global States (Iterate Root -> Target)
    // We need the Global State of every parent in the chain to calculate the vectors later.
    let mut global_states = Vec::with_capacity(chain.len());
    let mut current_global = GlobalState::default();

    // Iterate backwards through chain (Root -> Target)
    for &part_idx in chain.iter().rev() {
        let local = get_local_state(&parts[part_idx], model);
        
        // JS Logic:
        // getScaleX(id) = local.scaleX * getScaleX(parent)
        // getFlipX(id) = local.flipX * getFlipX(parent)
        
        let new_flip_x = local.flip_x * current_global.flip_x;
        let new_flip_y = local.flip_y * current_global.flip_y;
        
        let new_scale_x = local.scale_x * current_global.scale_x;
        let new_scale_y = local.scale_y * current_global.scale_y;
        
        // FIX: The Angle Accumulation Bug
        // JS: getAngle(id) = local.angle * getFlipX(parent) * getFlipY(parent) + getAngle(parent)
        // WAIT. JS logic for getAngle is:
        // this.spritesNow[id].angle * this.getFlipX(id) * this.getFlipY(id) + this.getAngle(this.spritesNow[id].parent);
        // Note that it uses `getFlipX(id)` which includes the CURRENT PART'S flip.
        // Therefore, we must multiply by (local.flip * parent.flip).
        
        let new_angle = local.angle * new_flip_x * new_flip_y + current_global.angle;
        
        let new_opacity = local.opacity * current_global.opacity;

        current_global = GlobalState {
            scale_x: new_scale_x,
            scale_y: new_scale_y,
            angle: new_angle,
            flip_x: new_flip_x,
            flip_y: new_flip_y,
            opacity: new_opacity,
        };
        
        global_states.push(current_global);
    }
    // global_states is now ordered [RootState, ..., TargetState]

    // 3. Calculate Global Position (The Vector Loop)
    // JS Logic uses a list of vectors.
    // vectors.unshift pushes to the front. 
    // It pushes: [LocalPos, ParentLocalScale+Flip, ParentAdjustedAngle]
    
    // We reconstruct the `vectors` list as described in JS logic.
    // The list in JS ends up being [ (Root->Child), ... , (Parent->Target) ]
    
    struct VectorStep {
        pos: [f64; 2],      // The local position of the node
        matrix_scale: [f64; 2], // The scale/flip matrix params from the PARENT
        matrix_rot: [f64; 4],   // The rotation matrix params from the PARENT
    }

    let mut vector_steps = Vec::with_capacity(chain.len());

    // Iterate Root -> Target
    // The chain is [Target, Parent, Root].
    // global_states is [RootState, ParentState, TargetState].
    
    // JS: "while (data.parent != -1)"
    if chain.len() > 1 {
        // We iterate from the first Child (index 1 in global_states) to the Target.
        // For each node, we determine the transformation imposed by its PARENT.
        
        for i in 0..chain.len() - 1 {
            // i=0 corresponds to the relationship between Root(parent) and the Next Node(child).
            
            // We want the node at global_states[i+1] (The Child)
            // We want the parent at global_states[i] (The Parent)
            
            let child_idx = chain[chain.len() - 1 - (i + 1)]; // i+1 from Root
            let parent_idx = chain[chain.len() - 1 - i];     // i from Root
            
            let child_local = get_local_state(&parts[child_idx], model);
            let parent_local = get_local_state(&parts[parent_idx], model);

            // Access Parent's GLOBAL state for flips
            // global_states[i] corresponds to parent_idx
            let parent_global_flip_x = global_states[i].flip_x;
            let parent_global_flip_y = global_states[i].flip_y;

            // JS: vectors.unshift(...)
            // pos: [data.x, -data.y] (Child's local pos)
            let pos = [child_local.x, -child_local.y];

            // matrix scale: [parent.scaleX * parent.flipX, ...]
            // Uses PARENT'S LOCAL scale and flip
            let sx = parent_local.scale_x * parent_local.flip_x;
            let sy = parent_local.scale_y * parent_local.flip_y;

            // matrix rot:
            // "const a = parent.angle * degToRad * getFlipX(parent) * getFlipY(parent)"
            // Uses PARENT'S LOCAL angle and PARENT'S GLOBAL flips
            let a_rad = parent_local.angle.to_radians() * parent_global_flip_x * parent_global_flip_y;
            let c = a_rad.cos();
            let s = a_rad.sin();
            
            // JS Rot Matrix: [cos, sin, -sin, cos]
            let rot = [c, s, -s, c];

            vector_steps.push(VectorStep {
                pos,
                matrix_scale: [sx, sy],
                matrix_rot: rot,
            });
        }
    }

    // JS Loop 1 & 2 combined (Matrix Application)
    // Iterate `j` (transforms) and apply to `k` (vectors)
    // JS: for (let j = 0; j < len; j++) ... for (let k = j; k < len; k++)
    
    // 1. Scale Application
    let len = vector_steps.len();
    for j in 0..len {
        let scale = vector_steps[j].matrix_scale;
        for k in j..len {
            vector_steps[k].pos[0] *= scale[0];
            vector_steps[k].pos[1] *= scale[1];
        }
    }

    // 2. Rotation Application and Summation
    let mut final_pos = [0.0, 0.0];
    for j in 0..len {
        let rot = vector_steps[j].matrix_rot;
        // JS applyMatrix: [m0*x + m1*y, m2*x + m3*y]
        // m0=c, m1=s, m2=-s, m3=c
        // x' = x*c + y*s
        // y' = x*-s + y*c
        
        for k in j..len {
            let x = vector_steps[k].pos[0];
            let y = vector_steps[k].pos[1];
            
            let nx = x * rot[0] + y * rot[1];
            let ny = x * rot[2] + y * rot[3];
            
            vector_steps[k].pos = [nx, ny];
        }
        
        final_pos[0] += vector_steps[j].pos[0];
        final_pos[1] += vector_steps[j].pos[1];
    }

    // 4. Construct Final Matrix
    // Use the Target's Global State
    let target_global = if !global_states.is_empty() {
        global_states.last().unwrap()
    } else {
        &current_global // Should default if list is empty (root only)
    };

    let sx = target_global.scale_x * target_global.flip_x;
    let sy = target_global.scale_y * target_global.flip_y;
    
    let angle_rad = target_global.angle.to_radians();
    let c = angle_rad.cos();
    let s = angle_rad.sin();
    
    // JS drawFrame Matrix logic ported to Column-Major
    // Note on Y-Axis:
    // JS logic is "Y-Up". Renderer is "Y-Down".
    // We must apply a Basis Transformation to map JS Space to Screen Space.
    // T_screen = S_flip * T_js * S_flip
    // This results in:
    // m10 (Index 1) becoming Positive (sx * s)
    // m01 (Index 3) becoming Negative (-sy * s)
    // m11 (Index 4) becoming Positive (sy * c)
    // m12 (Index 7) becoming Negative (-y)
    
    let matrix = [
        (sx * c) as f32,     (sx * s) as f32,          0.0,
        (-sy * s) as f32,    (sy * c) as f32,          0.0,
        final_pos[0] as f32, -final_pos[1] as f32,     1.0 
    ];

    WorldTransform {
        matrix,
        opacity: target_global.opacity as f32,
        z_order: target_part.drawing_layer,
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x as f32, target_part.pivot_y as f32),
        hidden: target_part.unit_id == -1 || target_part.sprite_index == -1 || target_global.opacity < 0.001,
        glow: target_part.glow_mode,
        part_index: target_index,
    }
}

fn get_local_state(part: &ModelPart, model: &Model) -> LocalState {
    let scale_unit = if model.scale_unit == 0.0 { 1000.0 } else { model.scale_unit as f64 };
    let angle_unit = if model.angle_unit == 0.0 { 1000.0 } else { model.angle_unit as f64 };
    let alpha_unit = if model.alpha_unit == 0.0 { 1000.0 } else { model.alpha_unit as f64 };

    LocalState {
        x: part.position_x as f64, // Raw pixels
        y: part.position_y as f64, // Raw pixels
        scale_x: part.scale_x as f64 / scale_unit,
        scale_y: part.scale_y as f64 / scale_unit,
        // JS: angle / (maxValues[1] / 360) -> angle * 360 / unit
        angle: (part.rotation as f64) * 360.0 / angle_unit,
        opacity: part.alpha as f64 / alpha_unit,
        flip_x: if part.flip_x { -1.0 } else { 1.0 },
        flip_y: if part.flip_y { -1.0 } else { 1.0 },
    }
}