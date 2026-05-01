use std::path::Path;
use std::collections::HashMap;
use eframe::egui;
use crate::features::stage::registry::Stage;
use crate::features::stage::logic::info as info_logic;
use crate::global::resolver;
use crate::features::stage::paths;

const MAP_IMG_HEIGHT: f32 = 50.0;
const STAGE_IMG_HEIGHT: f32 = 35.0;
const IMG_SPACING: f32 = 12.0;
const TOP_PADDING: f32 = 3.0;
const BOTTOM_PADDING: f32 = 5.0;

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

pub fn draw(
    egui_context: &egui::Context,
    ui: &mut egui::Ui, 
    stage_data: &Stage,
    map_name: &str,
    lang_priority: &[String],
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    lock_registry: &HashMap<u32, crate::features::stage::data::lockskipdata::LockSkipEntry>,
    cpu_setting: &crate::features::stage::data::scatcpusetting::ScatCpuSetting
) {
    let cat_formatted = info_logic::format_category_prefix(&stage_data.category);
    let map_dir = Path::new(paths::DIR_STAGES).join(&cat_formatted).join(format!("{:03}", stage_data.map_id));
    let stage_dir = map_dir.join(format!("{:02}", stage_data.stage_id));

    let map_img_key = format!("map_img_{}_{}", stage_data.category, stage_data.map_id);
    let stage_img_key = format!("stage_img_{}_{}_{}", stage_data.category, stage_data.map_id, stage_data.stage_id);

    if !texture_cache.contains_key(&map_img_key) {
        let possible_files = info_logic::get_map_image_filenames(stage_data.map_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();
        if let Some(resolved_path) = resolver::get(&map_dir, &refs, lang_priority).first() {
            if let Some(color_img) = info_logic::process_texture(resolved_path) {
                texture_cache.insert(map_img_key.clone(), egui_context.load_texture(&map_img_key, color_img, egui::TextureOptions::LINEAR));
            }
        }
    }

    if !texture_cache.contains_key(&stage_img_key) {
        let possible_files = info_logic::get_stage_image_filenames(stage_data.map_id, stage_data.stage_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();
        if let Some(resolved_path) = resolver::get(&stage_dir, &refs, lang_priority).first() {
            if let Some(color_img) = info_logic::process_texture(resolved_path) {
                texture_cache.insert(stage_img_key.clone(), egui_context.load_texture(&stage_img_key, color_img, egui::TextureOptions::LINEAR));
            }
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

    ui.strong("General Information");
    ui.separator();

    let energy_header = if stage_data.category == "B" { "Catamin" } else { "Energy" };
    let formatted_energy_value = info_logic::format_energy_cost(&stage_data.category, stage_data.energy);
    let formatted_difficulty = info_logic::format_difficulty_level(stage_data.difficulty);
    let formatted_crown = info_logic::format_crown_display(stage_data.target_crowns, stage_data.max_crowns);
    let formatted_no_continues = info_logic::format_boolean_status(stage_data.is_no_continues, "Yes", "No");
    let formatted_indestructible = info_logic::format_boolean_status(stage_data.is_base_indestructible, "Active", "-");
    let (base_header, formatted_base_value) = info_logic::format_base_display(stage_data.anim_base_id, stage_data.base_id);
    let formatted_global_respawn = info_logic::format_global_respawn(stage_data.min_spawn, stage_data.max_spawn);
    let formatted_boss_track = info_logic::format_boss_track(stage_data.boss_track, stage_data.init_track, stage_data.bgm_change_percent);
    let formatted_time_limit = info_logic::format_time_limit(stage_data.time_limit);
    let formatted_cpu_skip = info_logic::get_cpu_skip_status(&stage_data.category, stage_data.map_id,lock_registry, cpu_setting);
    
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