use std::path::Path;
use eframe::egui;
use crate::global::utils::autocrop;

pub fn format_energy_cost(category: &str, energy: u32) -> String {
    if category == "B" {
        return format!("{} Catamin", energy);
    }
    energy.to_string()
}

pub fn format_difficulty_level(difficulty: u16) -> String {
    if difficulty == 0 {
        return "-".to_string();
    }
    format!("★{}", difficulty)
}

pub fn format_crown_display(target_crowns: i8, max_crowns: u8) -> String {
    if target_crowns < 0 {
        return "-".to_string();
    }
    format!("{} / {}", target_crowns, max_crowns)
}

pub fn format_boolean_status(status: bool, true_str: &str, false_str: &str) -> String {
    if status { true_str.to_string() } else { false_str.to_string() }
}

pub fn format_base_display(anim_base_id: u32, base_id: i32) -> (String, String) {
    if anim_base_id > 0 {
        ("Anim Base".to_string(), anim_base_id.to_string())
    } else {
        ("Base ID".to_string(), base_id.to_string())
    }
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