use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::error;
use crate::global::resolver;
use nyanko::common::utils::csv::detect_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixedFormation {
    pub map_id: u32,
    pub level: u8,
    pub stage_no: u32,
    pub preset_file_name: String,
    pub memo: String,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<(u32, u8, u32), FixedFormation> {
    let mut map_result = HashMap::new();
    let paths = resolver::get(dir, [filename], priority);

    let Some(path) = paths.first() else {
        return map_result;
    };

    let Ok(content) = fs::read_to_string(path) else {
        error!("Failed to read fixed_formation file at {:?}", path);
        return map_result;
    };

    let sep = detect_separator(&content);
    let mut lines = content.lines();

    let Some(header_line) = lines.next() else {
        return map_result;
    };

    let headers: HashMap<&str, usize> = header_line
        .split(sep)
        .enumerate()
        .map(|(i, s)| (s.trim(), i))
        .collect();

    for line in lines {
        let Some(clean) = line.split("//").next() else { continue; };
        let clean = clean.trim();
        if clean.is_empty() { continue; }

        let parts: Vec<&str> = clean.split(sep).collect();

        let get_val = |header: &str, fallback_idx: usize| -> Option<&str> {
            if let Some(&idx) = headers.get(header) {
                parts.get(idx).copied().map(|s| s.trim())
            } else {
                parts.get(fallback_idx).copied().map(|s| s.trim())
            }
        };

        let Some(map_id_str) = get_val("MapID", 0) else { continue; };
        let Ok(map_id) = map_id_str.parse::<u32>() else { continue; };

        let Some(level_str) = get_val("Level", 1) else { continue; };
        let Ok(level) = level_str.parse::<u8>() else { continue; };

        let Some(stage_no_str) = get_val("StageNo", 2) else { continue; };
        let Ok(stage_no) = stage_no_str.parse::<u32>() else { continue; };

        let Some(preset_file_name) = get_val("PresetFileName", 3) else { continue; };

        let memo = get_val("MEMO", 4).unwrap_or("").to_string();

        map_result.insert((map_id, level, stage_no), FixedFormation {
            map_id,
            level,
            stage_no,
            preset_file_name: preset_file_name.to_string(),
            memo,
        });
    }

    map_result
}