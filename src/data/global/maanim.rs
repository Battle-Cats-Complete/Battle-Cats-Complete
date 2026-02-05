#![allow(dead_code)]
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
        if i < lines.len() && lines[i].starts_with("[") {
            i += 1;
        }

        // Header
        if i < lines.len() {
            i += 1; // version?
        }
        if i < lines.len() {
            i += 1; // total curves?
        }

        while i < lines.len() {
            let line = lines[i];
            let parts: Vec<&str> = line.split(delimiter).collect();
            i += 1;

            if parts.len() < 5 { continue; }

            let part_id = parts[0].trim().parse().unwrap_or(0);
            let mod_type = parts[1].trim().parse().unwrap_or(0);
            let loop_behavior = parts[2].trim().parse().unwrap_or(0);
            
            // FIX: Removed 'mut' (Warning fix)
            let min_f = parts[3].trim().parse().unwrap_or(0);
            let max_f = parts[4].trim().parse().unwrap_or(0);
            
            // Next line: count of keyframes
            if i >= lines.len() { break; }
            let count_line = lines[i];
            i += 1;
            let keyframe_count = count_line.trim().parse::<usize>().unwrap_or(0);

            let mut keyframes = Vec::new();
            let mut global_max_frame = 0;

            for _k in 0..keyframe_count {
                // FIX: Removed unused 'k_idx' (Warning fix)
                
                if i >= lines.len() { break; }
                let k_line = lines[i];
                i += 1;

                let kp: Vec<&str> = k_line.split(delimiter).collect();
                
                if kp.len() >= 2 {
                    let frame = kp[0].trim().parse().unwrap_or(0);
                    let value = kp[1].trim().parse().unwrap_or(0);
                    let ease = kp.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let power = kp.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    
                    if frame > global_max_frame { global_max_frame = frame; }
                    
                    keyframes.push(Keyframe { frame, value, ease_mode: ease, ease_power: power });
                }
            }

            if !keyframes.is_empty() {
                curves.push(AnimModification {
                    part_id,
                    modification_type: mod_type,
                    loop_count: loop_behavior,
                    keyframes,
                    min_frame: min_f,
                    max_frame: max_f,
                });
            }
        }

        // Calculate global max frame
        let mut max_len = 0;
        for c in &curves {
            if let Some(last) = c.keyframes.last() {
                if last.frame > max_len { max_len = last.frame; }
            }
        }

        Some(Self { curves, max_frame: max_len })
    }

    /// Efficiently scans a raw maanim file content to find its total duration
    pub fn scan_duration(file_content: &str) -> i32 {
        let mut max_frame_count = 0;
        let delimiter = utils::detect_csv_separator(file_content);
        
        let maanim_lines: Vec<Vec<i32>> = file_content.lines().map(|line| {
            line.split(delimiter)
                .filter_map(|component| component.trim().parse::<i32>().ok())
                .collect()
        }).collect();

        for (line_index, line_values) in maanim_lines.iter().enumerate() {
            if line_values.len() < 5 { continue; }
            
            let following_lines_count = maanim_lines.get(line_index + 1)
                .and_then(|l| l.get(0)).cloned().unwrap_or(0) as usize;
            
            if following_lines_count == 0 { continue; }
            
            let first_frame = maanim_lines.get(line_index + 2)
                .and_then(|l| l.get(0)).cloned().unwrap_or(0);
                
            let last_frame = maanim_lines.get(line_index + following_lines_count + 1)
                .and_then(|l| l.get(0)).cloned().unwrap_or(0);
                
            let animation_duration = last_frame - first_frame;
            let loop_repeats = std::cmp::max(line_values[2], 1); 
            let final_frame_used = (animation_duration * loop_repeats) + first_frame;
            
            max_frame_count = std::cmp::max(final_frame_used, max_frame_count);
        }
        
        max_frame_count + 1
    }
}