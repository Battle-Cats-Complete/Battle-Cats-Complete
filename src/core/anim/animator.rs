use crate::data::global::mamodel::{Model, ModelPart};
use crate::data::global::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

    for curve in &animation.curves {
        if curve.part_id >= parts.len() { continue; }
        
        let loop_count = curve.loop_count; 
        let fir = curve.min_frame;
        let smax = curve.max_frame;
        let lmax = (smax - fir).max(1);
        let mut local_frame = global_frame;

        if loop_count == -1 {
            if smax > fir {
                let frame_in_loop = (local_frame as i32 - fir).rem_euclid(lmax);
                local_frame = (fir + frame_in_loop) as f32 + (global_frame.fract());
            }
        } else if loop_count > 0 {
            let end_time = fir + loop_count * lmax;
            if global_frame >= end_time as f32 {
                local_frame = smax as f32; 
            } else if (global_frame as i32) > fir {
                let frame_in_loop = (global_frame as i32 - fir) % lmax;
                local_frame = (fir + frame_in_loop) as f32 + (global_frame.fract());
            }
        }
        
        // Mod 13/14 are discrete triggers
        let is_discrete = matches!(curve.modification_type, 0 | 1 | 3 | 13 | 14);
        
        let val = interpolate_curve(curve, local_frame, is_discrete);
        
        let part = &mut parts[curve.part_id];
        
        match curve.modification_type {
            0 => part.parent_id = val as i32,
            1 => part.unit_id = val as i32,
            3 => part.drawing_layer = val as i32, 
            
            2 => {
                // Sprite switching requires careful rounding based on direction
                // If we assume linear movement, we can check the delta. 
                // However, 'interpolate_curve' returns just the value here.
                // Standard BC rounding:
                part.sprite_index = val.round() as i32;
            },

            4 => part.position_x += val, 
            5 => part.position_y += val,
            6 => part.pivot_x += val,
            7 => part.pivot_y += val,
            8 => { 
                let factor = val / model.scale_unit;
                part.scale_x *= factor;
                part.scale_y *= factor;
            },
            9 => part.scale_x *= val / model.scale_unit,
            10 => part.scale_y *= val / model.scale_unit,
            11 => part.rotation += val,
            12 => part.alpha *= val / model.alpha_unit,
            
            // REVERTED: Mod 13/14 MUST modify scale to visually flip the sprite
            13 => {
                if val != 0.0 { 
                    part.scale_x *= -1.0; 
                    part.flip_x = true;   
                } else {
                    part.flip_x = false;
                }
            },
            14 => {
                if val != 0.0 { 
                    part.scale_y *= -1.0; 
                    part.flip_y = true;   
                } else {
                    part.flip_y = false;
                }
            },
            _ => {}
        }
    }
    
    parts
}

fn interpolate_curve(curve: &AnimModification, frame: f32, is_discrete: bool) -> f32 {
    if curve.keyframes.is_empty() { return 0.0; }

    let mut start_idx = 0;
    let mut end_idx = 0;
    let mut found = false;

    // Find the surrounding keyframes
    if frame < curve.keyframes[0].frame as f32 {
        return curve.keyframes[0].value as f32;
    }

    for (i, k) in curve.keyframes.iter().enumerate() {
        if (k.frame as f32) > frame {
            end_idx = i;
            start_idx = if i > 0 { i - 1 } else { 0 };
            found = true;
            break;
        }
    }
    
    // If past the last frame, hold the last value
    if !found {
        return curve.keyframes.last().unwrap().value as f32;
    }

    let start_k = &curve.keyframes[start_idx];
    let end_k = &curve.keyframes[end_idx];

    if is_discrete { return start_k.value as f32; }
    if start_k.frame == end_k.frame { return start_k.value as f32; }

    // --- NEW: Ease Mode 3 (Lagrange) Support ---
    if start_k.ease_mode == 3 {
        // Collect points that belong to this continuous Ease 3 segment
        let mut points = Vec::new();
        
        // Scan backwards
        let mut i = start_idx as isize;
        while i >= 0 {
            let k = &curve.keyframes[i as usize];
            points.push((k.frame as f32, k.value as f32));
            if k.ease_mode != 3 { break; } // Boundary of the spline
            i -= 1;
        }
        points.reverse(); // We pushed backwards, so reverse to time-order

        // Scan forwards
        let mut i = end_idx;
        while i < curve.keyframes.len() {
            let k = &curve.keyframes[i];
            points.push((k.frame as f32, k.value as f32));
            if k.ease_mode != 3 { break; }
            i += 1;
        }

        // Lagrange Interpolation
        let mut result = 0.0;
        let n = points.len();
        
        for j in 0..n {
            let (xj, yj) = points[j];
            let mut prod = yj;
            
            for m in 0..n {
                if j == m { continue; }
                let (xm, _) = points[m];
                if (xj - xm).abs() > 0.0001 {
                    prod *= (frame - xm) / (xj - xm);
                }
            }
            result += prod;
        }
        return result;
    }
    // -------------------------------------------

    // Standard Interpolation (Linear, Step, Exponential)
    let t_duration = (end_k.frame - start_k.frame) as f32;
    let t_current = frame - (start_k.frame as f32);
    let x = t_current / t_duration;

    let start_val = start_k.value as f32;
    let change = (end_k.value - start_k.value) as f32;

    match start_k.ease_mode {
        0 => start_val + (change * x), 
        1 => if x >= 1.0 { end_k.value as f32 } else { start_val }, 
        2 => { 
            let p = if start_k.ease_power != 0 { start_k.ease_power as f32 } else { 1.0 };
            let x_clamped = x.clamp(0.0, 1.0);
            let factor = if p >= 0.0 {
                1.0 - (1.0 - x_clamped.powf(p)).sqrt()
            } else {
                (1.0 - (1.0 - x_clamped).powf(-p)).sqrt()
            };
            if factor.is_nan() { start_val + (change * x) } else { start_val + (change * factor) }
        },
        _ => start_val + (change * x) 
    }
}