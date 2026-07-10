use std::collections::HashMap;
use std::path::Path;

use eframe::egui;

use core::global::resolver;
use core::global::utils::autocrop;
use core::stage::data::{lockskipdata, map_name, scatcpusetting};
use core::stage::paths;
use core::stage::registry::Stage;

const MAP_IMG_HEIGHT: f32 = 50.0;
const STAGE_IMG_HEIGHT: f32 = 35.0;
const IMG_SPACING: f32 = 12.0;
const TOP_PADDING: f32 = 3.0;
const BOTTOM_PADDING: f32 = 5.0;

// --- FORMATTERS ---

fn format_difficulty_level(difficulty: u16) -> String {
    if difficulty == 0 {
        return "-".to_string();
    }
    format!("★{}", difficulty)
}

fn format_energy_cost(category_prefix: &str, raw_energy_cost: u32) -> String {
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

fn format_crown_display(target_crowns: i8, max_crowns: u8) -> String {
    let crown_symbol = "♔";

    if target_crowns != -1 {
        return format!("{}{}", target_crowns + 1, crown_symbol);
    }

    if max_crowns > 1 {
        return format!("1{}~{}{}", crown_symbol, max_crowns, crown_symbol);
    }

    format!("1{}", crown_symbol)
}

fn format_base_display(anim_base_id: u32, standard_base_id: i32) -> (String, String) {
    if anim_base_id != 0 {
        let calculated_enemy_id = anim_base_id.saturating_sub(2);
        return ("Anim Base".to_string(), format!("E-{:03}", calculated_enemy_id));
    }
    ("Base Img".to_string(), standard_base_id.to_string())
}


fn format_boolean_status(status: bool, true_str: &str, false_str: &str) -> String {
    if status { true_str.to_string() } else { false_str.to_string() }
}

fn format_global_respawn(min_spawn: u32, max_spawn: u32) -> String {
    if min_spawn == max_spawn {
        return format!("{}f", min_spawn);
    }
    format!("{}f ~ {}f", min_spawn, max_spawn)
}

fn format_boss_track(boss_track: i16, init_track: u32, bgm_change_percent: u32) -> String {
    if boss_track < 0 || boss_track as u32 == init_track || bgm_change_percent == 100 {
        return "-".to_string();
    }
    boss_track.to_string()
}

fn format_time_limit(time_limit: u32) -> String {
    if time_limit == 0 {
        return "-".to_string();
    }
    format!("{}m", time_limit)
}

fn format_category_prefix(category: &str) -> String {
    let upper = category.to_uppercase();
    if upper.starts_with('R') && upper.len() > 1 {
        return upper[1..].to_string();
    }
    upper
}

fn get_cpu_skip_status(
    category: &str,
    map_id: u32,
    lock_registry: &HashMap<u32, lockskipdata::LockSkipEntry>,
    cpu_setting: &scatcpusetting::ScatCpuSetting
) -> String {
    let global_map_id = map_name::get_global_map_id(category, map_id);

    if let Some(mid) = global_map_id
        && let Some(entry) = lock_registry.get(&mid)
        && entry.excluded_map_id == mid {
        return "N/A".to_string();
    }

    if cpu_setting.super_cpu_consume_amount > 0 {
        return format!("{} CPUs", cpu_setting.super_cpu_consume_amount);
    }
    "-".to_string()
}

fn get_map_image_filenames(map_id: u32, category: &str, lang_priority: &[String]) -> Vec<String> {
    let cat_lower = format_category_prefix(category).to_lowercase();
    let mut filenames = Vec::new();
    for lang in lang_priority {
        filenames.push(format!("mapname{:03}_{}_{}.png", map_id, cat_lower, lang));
    }
    filenames.push(format!("mapname{:03}_{}.png", map_id, cat_lower));
    filenames
}

fn get_stage_image_filenames(map_id: u32, stage_id: u32, category: &str, lang_priority: &[String]) -> Vec<String> {
    let cat_lower = format_category_prefix(category).to_lowercase();
    let mut filenames = Vec::new();
    for lang in lang_priority {
        filenames.push(format!("mapsn{:03}_{:02}_{}_{}.png", map_id, stage_id, cat_lower, lang));
    }
    filenames.push(format!("mapsn{:03}_{:02}_{}.png", map_id, stage_id, cat_lower));
    filenames
}

fn process_texture(image_file_path: &Path) -> Option<egui::ColorImage> {
    let Ok(loaded_raw_image_data) = image::open(image_file_path) else {
        return None;
    };

    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let image_dimensions = [autocropped_rgba_image.width() as usize, autocropped_rgba_image.height() as usize];

    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, autocropped_rgba_image.as_flat_samples().as_slice()))
}

fn center_header(ui: &mut egui::Ui, display_text: &str) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(display_text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

fn center_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

// --- MAIN UI DRAW LOOP ---

#[allow(clippy::too_many_arguments)]
pub fn draw(
    egui_context: &egui::Context,
    ui: &mut egui::Ui,
    stage_data: &Stage,
    map_name: &str,
    lang_priority: &[String],
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    lock_registry: &HashMap<u32, core::stage::data::lockskipdata::LockSkipEntry>,
    cpu_setting: &core::stage::data::scatcpusetting::ScatCpuSetting,
    selected_crown: &mut u8 // <--- Added Crown State reference
) {
    let cat_formatted = format_category_prefix(&stage_data.category);
    let map_dir = Path::new(paths::DIR_STAGES).join(&cat_formatted).join(format!("{:03}", stage_data.map_id));
    let stage_dir = map_dir.join(format!("{:02}", stage_data.stage_id));

    let map_img_key = format!("map_img_{}_{}", stage_data.category, stage_data.map_id);
    let stage_img_key = format!("stage_img_{}_{}_{}", stage_data.category, stage_data.map_id, stage_data.stage_id);

    if !texture_cache.contains_key(&map_img_key) {
        let possible_files = get_map_image_filenames(stage_data.map_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();
        if let Some(resolved_path) = resolver::get(&map_dir, &refs, lang_priority).first()
            && let Some(color_img) = process_texture(resolved_path) {
            texture_cache.insert(map_img_key.clone(), egui_context.load_texture(&map_img_key, color_img, egui::TextureOptions::LINEAR));
        }
    }

    if !texture_cache.contains_key(&stage_img_key) {
        let possible_files = get_stage_image_filenames(stage_data.map_id, stage_data.stage_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();
        if let Some(resolved_path) = resolver::get(&stage_dir, &refs, lang_priority).first()
            && let Some(color_img) = process_texture(resolved_path) {
            texture_cache.insert(stage_img_key.clone(), egui_context.load_texture(&stage_img_key, color_img, egui::TextureOptions::LINEAR));
        }
    }

    let mut map_width = 0.0;
    let mut stage_width = 0.0;
    let has_map = texture_cache.contains_key(&map_img_key);
    let has_stage = texture_cache.contains_key(&stage_img_key);

    if has_map {
        let size = texture_cache.get(&map_img_key).unwrap().size_vec2();
        map_width = size.x * (MAP_IMG_HEIGHT / size.y);
    }
    if has_stage {
        let size = texture_cache.get(&stage_img_key).unwrap().size_vec2();
        stage_width = size.x * (STAGE_IMG_HEIGHT / size.y);
    }

    let max_height = MAP_IMG_HEIGHT.max(STAGE_IMG_HEIGHT);

    ui.add_space(TOP_PADDING);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), max_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if has_map {
                let map_tex = texture_cache.get(&map_img_key).unwrap();
                ui.add(egui::Image::new(map_tex).fit_to_exact_size(egui::vec2(map_width, MAP_IMG_HEIGHT)));
            } else {
                ui.label(egui::RichText::new(map_name).strong().size(18.0));
            }
            ui.add_space(IMG_SPACING);
            if has_stage {
                let stage_tex = texture_cache.get(&stage_img_key).unwrap();
                ui.add(egui::Image::new(stage_tex).fit_to_exact_size(egui::vec2(stage_width, STAGE_IMG_HEIGHT)));
            } else {
                ui.label(egui::RichText::new(&stage_data.name).strong().size(18.0));
            }
        }
    );

    ui.add_space(BOTTOM_PADDING);
    ui.separator();
    ui.add_space(BOTTOM_PADDING);

    // <--- CROWNS DRAWN HERE --->
    super::crowns::draw(ui, stage_data, selected_crown);

    ui.strong("General Information");
    ui.separator();

    let energy_header = if stage_data.category == "B" { "Catamin" } else { "Energy" };
    let formatted_energy_value = format_energy_cost(&stage_data.category, stage_data.energy);
    let formatted_difficulty = format_difficulty_level(stage_data.difficulty);
    let formatted_crown = format_crown_display(stage_data.target_crowns, stage_data.max_crowns);
    let formatted_no_continues = format_boolean_status(stage_data.is_no_continues, "Yes", "No");
    let formatted_indestructible = format_boolean_status(stage_data.is_base_indestructible, "Active", "-");
    let (base_header, formatted_base_value) = format_base_display(stage_data.anim_base_id, stage_data.base_id);
    let formatted_global_respawn = format_global_respawn(stage_data.min_spawn, stage_data.max_spawn);
    let formatted_boss_track = format_boss_track(stage_data.boss_track as i16, stage_data.init_track, stage_data.bgm_change_percent);
    let formatted_time_limit = format_time_limit(stage_data.time_limit);
    let formatted_cpu_skip = get_cpu_skip_status(&stage_data.category, stage_data.map_id,lock_registry, cpu_setting);

    egui::Grid::new("stage_meta_grid")
        .striped(true)
        .spacing([15.0, 8.0])
        .show(ui, |grid| {
            center_header(grid, "Base HP");
            center_header(grid, energy_header);
            center_header(grid, "XP Base");
            center_header(grid, "Width");
            center_header(grid, "Max Enemy");
            center_header(grid, "Respawn");
            center_header(grid, "Time Limit");
            center_header(grid, "Difficulty");
            grid.end_row();

            center_text(grid, stage_data.base_hp.to_string());
            center_text(grid, formatted_energy_value);
            center_text(grid, stage_data.xp.to_string());
            center_text(grid, stage_data.width.to_string());
            center_text(grid, stage_data.max_enemies.to_string());
            center_text(grid, formatted_global_respawn);
            center_text(grid, formatted_time_limit);
            center_text(grid, formatted_difficulty);
            grid.end_row();

            center_header(grid, "No Cont.");
            center_header(grid, "Boss Guard");
            center_header(grid, &base_header);
            center_header(grid, "BG ID");
            center_header(grid, "BGM");
            center_header(grid, "Boss BGM");
            center_header(grid, "Crowns");
            center_header(grid, "CPU Skip");
            grid.end_row();

            center_text(grid, formatted_no_continues);
            center_text(grid, formatted_indestructible);
            center_text(grid, formatted_base_value);
            center_text(grid, stage_data.background_id.to_string());
            center_text(grid, stage_data.init_track.to_string());
            center_text(grid, formatted_boss_track);
            center_text(grid, formatted_crown);
            center_text(grid, formatted_cpu_skip);
            grid.end_row();
        });
}