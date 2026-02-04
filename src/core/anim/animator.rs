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
        
        // FIX: Handle Pre-Animation (return None)
        if let Some(val) = interpolate_curve(curve, local_frame, is_discrete) {
            let part = &mut parts[curve.part_id];
            
            match curve.modification_type {
                0 => part.parent_id = val as i32,
                1 => part.unit_id = val as i32,
                3 => part.drawing_layer = val as i32, 
                
                2 => {
                    // FIX: Mod 2 (Sprite) is now rounded correctly inside interpolate_curve
                    // so we can just cast it here.
                    part.sprite_index = val as i32;
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
                
                // FIX: Flip Logic (Mod 13/14)
                // Do NOT multiply scale by -1.0. Just set the flag.
                // transform.rs handles the negative multiplication based on the flag.
                13 => {
                    part.flip_x = val != 0.0;
                },
                14 => {
                    part.flip_y = val != 0.0;
                },
                _ => {}
            }
        }
    }
    
    parts
}

fn interpolate_curve(curve: &AnimModification, frame: f32, is_discrete: bool) -> Option<f32> {
    if curve.keyframes.is_empty() { return None; }

    // FIX: Return None if before first frame (preserves default model state)
    if frame < curve.keyframes[0].frame as f32 {
        return None;
    }

    let mut start_idx = 0;
    let mut end_idx = 0;
    let mut found = false;

    for (i, k) in curve.keyframes.iter().enumerate() {
        if (k.frame as f32) > frame {
            end_idx = i;
            start_idx = if i > 0 { i - 1 } else { 0 };
            found = true;
            break;
        }
    }
    
    if !found {
        return Some(curve.keyframes.last().unwrap().value as f32);
    }

    let start_k = &curve.keyframes[start_idx];
    let end_k = &curve.keyframes[end_idx];

    if is_discrete { return Some(start_k.value as f32); }
    if start_k.frame == end_k.frame { return Some(start_k.value as f32); }

    // --- EASE MODE 3 (LAGRANGE) ---
    if start_k.ease_mode == 3 {
        let mut points = Vec::new();
        
        let mut i = start_idx as isize;
        while i >= 0 {
            let k = &curve.keyframes[i as usize];
            if (i as usize) != start_idx && k.ease_mode != 3 { 
                break; 
            }
            points.push((k.frame as f32, k.value as f32));
            i -= 1;
        }
        points.reverse(); 

        let mut i = end_idx;
        while i < curve.keyframes.len() {
            let k = &curve.keyframes[i];
            points.push((k.frame as f32, k.value as f32));
            if k.ease_mode != 3 { break; }
            i += 1;
        }

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
        return Some(result);
    }
    // -------------------------------------------

    let t_duration = (end_k.frame - start_k.frame) as f32;
    let t_current = frame - (start_k.frame as f32);
    let x = t_current / t_duration;

    let start_val = start_k.value as f32;
    let change = (end_k.value - start_k.value) as f32;

    let interpolated_val = match start_k.ease_mode {
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
    };

    // FIX: Special Rounding for Sprite Switching (Mod 2)
    // JS Logic: if decreasing, use Ceil. If increasing, use Floor.
    if curve.modification_type == 2 {
        if change < 0.0 {
            return Some(interpolated_val.ceil());
        } else {
            return Some(interpolated_val.floor());
        }
    }

    Some(interpolated_val)
}