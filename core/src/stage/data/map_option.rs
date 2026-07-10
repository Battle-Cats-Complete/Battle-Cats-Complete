use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;
use nyanko::common::utils::csv::detect_separator;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ResetType {
    #[default]
    None,
    ResetRewards,
    ResetRewardsAndClear,
    ResetMaxClears,
    Unknown(u8),
}

impl From<u8> for ResetType {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::None,
            1 => Self::ResetRewards,
            2 => Self::ResetRewardsAndClear,
            3 => Self::ResetMaxClears,
            _ => Self::Unknown(val),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MapOption {
    pub map_id: u32,
    pub max_crowns: u8,
    pub has_abyss: bool,
    pub crown_1_mag: Option<u32>,
    pub crown_2_mag: Option<u32>,
    pub crown_3_mag: Option<u32>,
    pub crown_4_mag: Option<u32>,
    pub reset_type: ResetType,
    pub max_clears: u32,
    pub cooldown_minutes: u32,
    pub hidden_upon_clear: bool,
    pub comment: String,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, MapOption> {
    let mut map_result = HashMap::new();
    let paths = resolver::get(dir, [filename], priority);

    let Some(path) = paths.first() else { return map_result; };
    let Ok(content) = fs::read_to_string(path) else { return map_result; };
    let sep = detect_separator(&content);

    let mut lines = content.lines();
    let Some(header_line) = lines.next() else { return map_result; };

    let headers: HashMap<&str, usize> = header_line
        .split(sep)
        .enumerate()
        .map(|(i, s)| (s.trim(), i))
        .collect();

    for line in lines {
        let comment = line.split("//").nth(1).unwrap_or("").trim().to_string();

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
        
        let offset = if parts.get(2).map_or(true, |s| s.trim().is_empty() || s.trim().parse::<u32>().is_err()) { 1 } else { 0 };

        let Some(map_id_str) = get_val("stageID", 0) else { continue; };
        let Ok(map_id) = map_id_str.parse::<u32>() else { continue; };

        let max_crowns = get_val("星解放", 1).and_then(|s| s.parse().ok()).unwrap_or(1);
        let has_abyss = get_val("裏星解放", 2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0) == 1;

        let crown_1_mag = get_val("星1倍率", 3 + offset).and_then(|s| s.parse().ok());
        let crown_2_mag = get_val("星2倍率", 4 + offset).and_then(|s| s.parse().ok());
        let crown_3_mag = get_val("星3倍率", 5 + offset).and_then(|s| s.parse().ok());
        let crown_4_mag = get_val("星4倍率", 6 + offset).and_then(|s| s.parse().ok());

        let reset_type = get_val("報酬リセットType", 8 + offset).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
        let max_clears = get_val("1度きり表示", 9 + offset).and_then(|s| s.parse().ok()).unwrap_or(0);
        let cooldown = get_val("インターバル", 11 + offset).and_then(|s| s.parse().ok()).unwrap_or(0);
        let hidden = get_val("クリア後非表示", 14 + offset).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0) == 1;

        map_result.insert(map_id, MapOption {
            map_id,
            max_crowns,
            has_abyss,
            crown_1_mag,
            crown_2_mag,
            crown_3_mag,
            crown_4_mag,
            reset_type: ResetType::from(reset_type),
            max_clears,
            cooldown_minutes: cooldown,
            hidden_upon_clear: hidden,
            comment,
        });
    }

    map_result
}