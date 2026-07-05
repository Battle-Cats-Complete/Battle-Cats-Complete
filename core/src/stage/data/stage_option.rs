use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use crate::global::resolver;
use nyanko::common::utils::csv::detect_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageOption {
    pub map_id: u32,
    pub target_crowns: i8,
    pub target_stage: i32,
    pub rarity_mask: u8,
    pub deploy_limit: u32,
    pub allowed_rows: u8,
    pub min_cost: u32,
    pub max_cost: u32,
    pub charagroup_id: u32,
}

fn redirect_map_id(id: u32) -> u32 {
    match id {
        20000 => 3008, // EoC 1 Zombie
        20001 => 3009, // EoC 2 Zombie
        20002 => 3010, // EoC 3 Zombie

        21000 => 3011, // ItF 1 Zombie
        21001 => 3012, // ItF 2 Zombie
        21002 => 3013, // ItF 3 Zombie

        22000 => 3015, // CotC 1/2 Zombie edge case
        22001 => 3015, // CotC 2 Zombie
        22002 => 3016, // CotC 3 Zombie

        23000 => 3007, // CotC 3 Invasion
        30000 => 3018, // Aku Realms
        38000 => 3017, // CotC 3 Invasion (Zombie)

        _ => id,
    }
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, Vec<StageOption>> {
    let mut map: HashMap<u32, Vec<StageOption>> = HashMap::new();
    let paths = resolver::get(dir, [filename], priority);

    let Some(path) = paths.first() else {
        warn!(filename, "StageOption file not found in resolver paths");
        return map;
    };

    let Ok(content) = fs::read_to_string(path) else {
        warn!(path = %path.display(), "Failed to read StageOption file contents");
        return map;
    };

    let sep = detect_separator(&content);
    let mut valid_lines = 0;

    for line in content.lines().skip(1) {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }

        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 9 { continue; }

        let Ok(raw_map_id) = parts[0].trim().parse::<u32>() else { continue; };

        let map_id = redirect_map_id(raw_map_id);

        let opt = StageOption {
            map_id,
            target_crowns: parts[1].trim().parse().unwrap_or(-1),
            target_stage: parts[2].trim().parse().unwrap_or(-1),
            rarity_mask: parts[3].trim().parse().unwrap_or(0),
            deploy_limit: parts[4].trim().parse().unwrap_or(0),
            allowed_rows: parts[5].trim().parse().unwrap_or(0),
            min_cost: parts[6].trim().parse().unwrap_or(0),
            max_cost: parts[7].trim().parse().unwrap_or(0),
            charagroup_id: parts[8].trim().parse().unwrap_or(0),
        };

        map.entry(map_id).or_default().push(opt);
        valid_lines += 1;
    }

    debug!(
        file = filename,
        entries = valid_lines,
        unique_maps = map.len(),
        "Successfully parsed stage options"
    );

    map
}