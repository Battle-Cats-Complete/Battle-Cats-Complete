use std::fs;
use std::path::Path;
use crate::core::utils;

#[derive(Clone, Debug)]
pub struct Keyframe {
    pub frame: i32,
    pub value: i32,
    pub ease_mode: i32,
    pub ease_power: i32,
}

#[derive(Clone, Debug)]
pub struct AnimModification {
    pub part_id: usize,
    pub modification_type: i32,
    pub loop_count: i32,
    pub keyframes: Vec<Keyframe>,
    pub min_frame: i32,
    pub max_frame: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Animation {
    pub curves: Vec<AnimModification>,
    pub max_frame: i32,
}

impl Animation {
    pub fn load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        let mut curves = Vec::new();
        let mut i = 0;

        // Skip [maanim]
        if i < lines.len() && lines[i].trim().starts_with('[') { i += 1; }
        // Skip Version
        if i < lines.len() && lines[i].trim().len() < 5 && lines[i].trim().parse::<i32>().is_ok() { i += 1; }
        // Skip Total Parts
        if i < lines.len() && lines[i].trim().parse::<usize>().is_ok() { i += 1; }

        let mut global_max_frame = 0;

        while i < lines.len() {
            let header_line = lines[i];
            let p: Vec<&str> = header_line.split(delimiter).collect();
            if p.len() < 3 { i += 1; continue; }

            let part_id: usize = p[0].trim().parse().unwrap_or(0);
            let mod_type: i32 = p[1].trim().parse().unwrap_or(0);
            let loop_behavior: i32 = p[2].trim().parse().unwrap_or(0);

            if i + 1 >= lines.len() { break; }
            let count_line = lines[i+1];
            let p_count: Vec<&str> = count_line.split(delimiter).collect();
            let key_count: usize = p_count.get(0).and_then(|s| s.trim().parse().ok()).unwrap_or(0);

            let mut keyframes = Vec::new();
            let mut min_f = i32::MAX;
            let mut max_f = i32::MIN;

            for k in 0..key_count {
                let k_idx = i + 2 + k;
                if k_idx >= lines.len() { break; }
                let k_line = lines[k_idx];
                let kp: Vec<&str> = k_line.split(delimiter).collect();
                
                if kp.len() >= 2 {
                    let frame = kp[0].trim().parse().unwrap_or(0);
                    let value = kp[1].trim().parse().unwrap_or(0);
                    let ease = kp.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let power = kp.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    
                    if frame > global_max_frame { global_max_frame = frame; }
                    if frame < min_f { min_f = frame; }
                    if frame > max_f { max_f = frame; }
                    
                    keyframes.push(Keyframe { frame, value, ease_mode: ease, ease_power: power });
                }
            }

            if !keyframes.is_empty() {
                curves.push(AnimModification {
                    part_id,
                    modification_type: mod_type,
                    loop_count: loop_behavior,
                    keyframes,
                    min_frame: if min_f == i32::MAX { 0 } else { min_f },
                    max_frame: if max_f == i32::MIN { 0 } else { max_f },
                });
            }

            i += 2 + key_count;
        }

        Some(Animation { curves, max_frame: global_max_frame })
    }
}