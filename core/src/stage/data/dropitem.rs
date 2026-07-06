use std::collections::HashMap;
use std::fs;
use std::path::Path;

use nyanko::common::utils::csv::detect_separator;
use serde::{Deserialize, Serialize};

use crate::global::resolver;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DropItem {
    pub map_id: u32,
    pub crown_multipliers: [f32; 4],
    pub stage_drops: [u32; 8],
    pub dud_chance: u32,
    pub material_drops: [u32; 16],
}

#[inline(always)]
fn parse_f32<const N: usize>(parts: &[&str], offset: usize) -> Option<[f32; N]> {
    let mut a = [0.0; N];
    for i in 0..N {
        a[i] = parts.get(offset + i)?.trim().parse().ok()?;
    }
    Some(a)
}

#[inline(always)]
fn parse_u32<const N: usize>(parts: &[&str], offset: usize) -> Option<[u32; N]> {
    let mut a = [0; N];
    for i in 0..N {
        a[i] = parts.get(offset + i)?.trim().parse().ok()?;
    }
    Some(a)
}

#[inline(always)]
fn parse_u32_opt<const N: usize>(parts: &[&str], offset: usize) -> [u32; N] {
    let mut a = [0; N];
    for i in 0..N {
        a[i] = parts.get(offset + i).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    }
    a
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, DropItem> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, [filename], priority);

    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_separator(&content);

    for line in content.lines().skip(1) {
        let Some(clean) = line.split("//").next() else { continue; };
        let clean = clean.trim();
        if clean.is_empty() { continue; }

        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 22 { continue; }

        let Ok(map_id) = parts[0].trim().parse::<u32>() else { continue; };

        let Some(crown_multipliers) = parse_f32(&parts, 1) else { continue; };
        let Some(stage_drops) = parse_u32::<8>(&parts, 5) else { continue; };

        let Ok(dud_chance) = parts[13].trim().parse::<u32>() else { continue; };

        let Some(base_mats) = parse_u32::<8>(&parts, 14) else { continue; };
        let z_mats = parse_u32_opt::<8>(&parts, 22);

        let mut material_drops = [0; 16];
        material_drops[..8].copy_from_slice(&base_mats);
        material_drops[8..].copy_from_slice(&z_mats);

        map.insert(map_id, DropItem {
            map_id,
            crown_multipliers,
            stage_drops,
            dud_chance,
            material_drops,
        });
    }

    map
}