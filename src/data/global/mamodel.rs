use std::fs;
use std::path::Path;
use crate::core::utils;

#[derive(Clone, Debug, Default)]
pub struct ModelPart {
    pub parent_id: i32,
    pub unit_id: i32,
    pub sprite_index: i32,
    pub drawing_layer: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub alpha: f32,
    pub glow_mode: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub parts: Vec<ModelPart>,
    pub version: u32,
    pub scale_unit: f32, 
    pub angle_unit: f32,
    pub alpha_unit: f32,
}

impl Model {
    pub fn load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        // --- ROBUST HEADER PARSING ---
        let mut part_count = 0;
        let mut data_start_index = 0;

        // Try to find the part count in the first 5 lines
        for (i, line) in lines.iter().take(5).enumerate() {
            // If line is a single number and seemingly valid count (1-1000)
            if !line.contains(',') {
                if let Ok(val) = line.trim().parse::<usize>() {
                    // BCU format: Line 0=[header], Line 1=Ver, Line 2=Count
                    // Raw format: Line 0=Ver, Line 1=Count OR Line 0=Count
                    // We assume the LAST single number we see before CSV data is the count.
                    part_count = val;
                    data_start_index = i + 1;
                }
            } else {
                // Found CSV data (commas), stop searching.
                break;
            }
        }

        if part_count == 0 { return None; }

        // "Units" line (Scale, Angle, Alpha) is immediately after the parts
        let unit_line_index = data_start_index + part_count;

        let mut scale_unit = 1000.0;
        let mut angle_unit = 10.0; // 3600 = 360 deg
        let mut alpha_unit = 255.0; // 255 = 1.0 opacity

        if lines.len() > unit_line_index {
            let unit_line = lines[unit_line_index];
            let p: Vec<&str> = unit_line.split(delimiter).collect();
            if p.len() >= 3 {
                 if let Ok(s) = p[0].trim().parse::<f32>() { scale_unit = s; }
                 if let Ok(a) = p[1].trim().parse::<f32>() { angle_unit = a; }
                 if let Ok(o) = p[2].trim().parse::<f32>() { alpha_unit = o; }
            }
        }

        // Safety Defaults
        if scale_unit.abs() < 1.0 { scale_unit = 1000.0; }
        if angle_unit.abs() < 0.01 { angle_unit = 10.0; }
        if alpha_unit.abs() < 1.0 { alpha_unit = 255.0; }

        let mut parts = Vec::new();

        for i in 0..part_count {
            let line_idx = data_start_index + i;
            if line_idx >= lines.len() { break; }
            let line = lines[line_idx];
            
            let p: Vec<&str> = line.split(delimiter).collect();
            if p.len() < 13 { continue; } 

            let part = ModelPart {
                parent_id:     p[0].trim().parse().unwrap_or(-1),
                unit_id:       p[1].trim().parse().unwrap_or(0),
                sprite_index:  p[2].trim().parse().unwrap_or(0),
                drawing_layer: p[3].trim().parse().unwrap_or(0),
                position_x:    p[4].trim().parse().unwrap_or(0.0),
                position_y:    p[5].trim().parse().unwrap_or(0.0),
                pivot_x:       p[6].trim().parse().unwrap_or(0.0),
                pivot_y:       p[7].trim().parse().unwrap_or(0.0),
                scale_x:       p[8].trim().parse().unwrap_or(scale_unit), 
                scale_y:       p[9].trim().parse().unwrap_or(scale_unit),
                rotation:      p[10].trim().parse().unwrap_or(0.0),
                alpha:         p[11].trim().parse().unwrap_or(alpha_unit),
                glow_mode:     p[12].trim().parse().unwrap_or(0),
            };
            parts.push(part);
        }

        Some(Model { parts, version: 1, scale_unit, angle_unit, alpha_unit })
    }
}