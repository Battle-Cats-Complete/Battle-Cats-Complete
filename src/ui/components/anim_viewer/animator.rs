use crate::data::global::mamodel::{Model, ModelPart};
use crate::data::global::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

    for curve in &animation.curves {
        if curve.part_id >= parts.len() { continue; }
        
        let loop_count = curve.loop_count; 
        let fir = curve.min_frame;
        let smax = curve.max_frame;
        let lmax = smax - fir;
        let mut local_frame = global_frame;

        if loop_count == -1 {
            if lmax > 0 {
                let frame_in_loop = (local_frame as i32 - fir).rem_euclid(lmax);
                local_frame = (fir + frame_in_loop) as f32 + (global_frame.fract());
            }
        } else if loop_count > 0 && lmax > 0 {
            let end_time = fir + loop_count * lmax;
            // FIX: Use >= to Clamp "Once" animations at the end. 
            // Prevents wrapping to 0 when frame == end_time (due to Jitter Fix).
            if global_frame >= end_time as f32 {
                local_frame = smax as f32; 
            } else if (global_frame as i32) > fir {
                let frame_in_loop = (global_frame as i32 - fir) % lmax;
                local_frame = (fir + frame_in_loop) as f32 + (global_frame.fract());
            }
        }
        
        let is_discrete = curve.modification_type == 0 || curve.modification_type == 1 || curve.modification_type == 3;
        
        let (val, slope) = interpolate_curve(curve, local_frame, is_discrete);
        
        let part = &mut parts[curve.part_id];
        
        match curve.modification_type {
            0 => part.parent_id = val as i32,
            1 => part.unit_id = val as i32,
            3 => part.drawing_layer = val as i32, 
            
            // Sprite Index (Mod 2)
            2 => {
                if slope < 0.0 {
                    part.sprite_index = val.ceil() as i32;
                } else {
                    part.sprite_index = val.floor() as i32;
                }
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
            
            // GLOW HACK: Store "True Flip" state in high bits of glow_mode
            // This separates Mod 13 (Flip) from Mod 8 (Scale)
            13 => {
                if val != 0.0 { 
                    part.scale_x *= -1.0; 
                    part.glow_mode |= 0x10000; // Flag Flip X
                } else {
                    part.glow_mode &= !0x10000;
                }
            },
            14 => {
                if val != 0.0 { 
                    part.scale_y *= -1.0; 
                    part.glow_mode |= 0x20000; // Flag Flip Y
                } else {
                    part.glow_mode &= !0x20000;
                }
            },
            _ => {}
        }
    }
    
    parts
}

fn interpolate_curve(curve: &AnimModification, frame: f32, is_discrete: bool) -> (f32, f32) {
    if curve.keyframes.is_empty() { return (0.0, 0.0); }

    let mut start_k = &curve.keyframes[0];
    let mut end_k = &curve.keyframes[0];

    if frame <= start_k.frame as f32 { return (start_k.value as f32, 0.0); }

    let mut found = false;
    for k in &curve.keyframes {
        if (k.frame as f32) > frame {
            end_k = k;
            found = true;
            break;
        }
        start_k = k;
    }
    
    if !found {
        return (start_k.value as f32, 0.0);
    }
    
    if is_discrete { return (start_k.value as f32, 0.0); }
    if start_k.frame == end_k.frame { return (start_k.value as f32, 0.0); }

    let t_duration = (end_k.frame - start_k.frame) as f32;
    let t_current = frame - (start_k.frame as f32);
    let x = t_current / t_duration;

    let start_val = start_k.value as f32;
    let change = (end_k.value - start_k.value) as f32;
    let slope = change; 

    let result = match start_k.ease_mode {
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
    
    (result, slope)
}