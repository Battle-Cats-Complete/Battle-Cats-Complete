use std::path::Path;
use eframe::egui;
use crate::global::utils::autocrop;
use crate::features::stage::data::{map_name, lockskipdata, scatcpusetting};
use std::collections::HashMap;

pub fn format_difficulty_level(difficulty: u16) -> String {
    if difficulty == 0 {
        return "-".to_string();
    }
    format!("★{}", difficulty)
}

pub fn format_energy_cost(category_prefix: &str, raw_energy_cost: u32) -> String {
    if category_prefix != "B" {
        return raw_energy_cost.to_string();
    }

    if raw_energy_cost < 1000 {
        return format!("{}A", raw_energy_cost);
    }
    
    if raw_energy_cost < 2000 {
        return format!("{}B", raw_energy_cost % 1000);
    }
    
    format!("{}C", raw_energy_cost % 1000)
}

pub fn format_crown_display(target_crowns: i8, max_crowns: u8) -> String {
    let crown_symbol = "♔"; 
    
    if target_crowns != -1 {
        return format!("{}{}", target_crowns + 1, crown_symbol);
    }
    
    if max_crowns > 1 {
        return format!("1{}~{}{}", crown_symbol, max_crowns, crown_symbol);
    }
    
    format!("1{}", crown_symbol)
}

pub fn format_base_display(anim_base_id: u32, standard_base_id: i32) -> (String, String) {
    if anim_base_id != 0 {
        let calculated_enemy_id = if anim_base_id >= 2 { anim_base_id - 2 } else { 0 };
        return ("Anim Base".to_string(), format!("E-{:03}", calculated_enemy_id));
    }
    ("Base Img".to_string(), standard_base_id.to_string())
}


pub fn format_boolean_status(status: bool, true_str: &str, false_str: &str) -> String {
    if status { true_str.to_string() } else { false_str.to_string() }
}

pub fn format_global_respawn(min_spawn: u32, max_spawn: u32) -> String {
    if min_spawn == max_spawn {
        return format!("{}f", min_spawn);
    }
    format!("{}f ~ {}f", min_spawn, max_spawn)
}

pub fn format_boss_track(boss_track: u32, init_track: u32, bgm_change_percent: u32) -> String {
    if boss_track == init_track || bgm_change_percent == 100 {
        return "-".to_string();
    }
    boss_track.to_string()
}

pub fn format_time_limit(time_limit: u32) -> String {
    if time_limit == 0 {
        return "-".to_string();
    }
    format!("{}m", time_limit)
}

pub fn format_category_prefix(category: &str) -> String {
    let upper = category.to_uppercase();
    if upper.starts_with('R') && upper.len() > 1 {
        return upper[1..].to_string();
    }
    upper
}

pub fn get_cpu_skip_status(
    category: &str, 
    map_id: u32, 
    lock_registry: &HashMap<u32, lockskipdata::LockSkipEntry>,
    cpu_setting: &scatcpusetting::ScatCpuSetting
) -> String {
    let global_map_id = map_name::get_global_map_id(category, map_id);

    if let Some(mid) = global_map_id {
        if let Some(entry) = lock_registry.get(&mid) {
            if entry.excluded_map_id == mid {
                return "N/A".to_string();
            }
        }
    }

    if cpu_setting.super_cpu_consume_amount > 0 {
        return format!("{} CPUs", cpu_setting.super_cpu_consume_amount);
    }
    "-".to_string()
}

pub fn get_map_image_filenames(map_id: u32, category: &str, lang_priority: &[String]) -> Vec<String> {
    let cat_lower = format_category_prefix(category).to_lowercase();
    let mut filenames = Vec::new();
    for lang in lang_priority {
        filenames.push(format!("mapname{:03}_{}_{}.png", map_id, cat_lower, lang));
    }
    filenames.push(format!("mapname{:03}_{}.png", map_id, cat_lower)); 
    filenames
}

pub fn get_stage_image_filenames(map_id: u32, stage_id: u32, category: &str, lang_priority: &[String]) -> Vec<String> {
    let cat_lower = format_category_prefix(category).to_lowercase();
    let mut filenames = Vec::new();
    for lang in lang_priority {
        filenames.push(format!("mapsn{:03}_{:02}_{}_{}.png", map_id, stage_id, cat_lower, lang));
    }
    filenames.push(format!("mapsn{:03}_{:02}_{}.png", map_id, stage_id, cat_lower)); 
    filenames
}

pub fn process_texture(image_file_path: &Path) -> Option<egui::ColorImage> {
    let Ok(loaded_raw_image_data) = image::open(image_file_path) else {
        return None;
    };
    
    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let image_dimensions = [autocropped_rgba_image.width() as usize, autocropped_rgba_image.height() as usize];
    
    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, autocropped_rgba_image.as_flat_samples().as_slice()))
}