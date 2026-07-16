use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use nyanko::chapter::Category;
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;

use core::common::resolver;
use core::common::utils::autocrop;
use core::modules::stage::paths;
use core::modules::stage::registry::{Map, Stage};

const MAP_IMG_HEIGHT: f32 = 50.0;
const STAGE_IMG_HEIGHT: f32 = 35.0;
const IMG_SPACING: f32 = 12.0;
const TOP_PADDING: f32 = 3.0;
const BOTTOM_PADDING: f32 = 5.0;
const FALLBACK_MAP_SIZE: f32 = 32.0;
const FALLBACK_STAGE_SIZE: f32 = 32.0;

fn format_diff(difficulty_level: u16) -> String {
    if difficulty_level == 0 {
        return "-".to_string();
    }
    format!("★{}", difficulty_level)
}

fn format_energy(category: &Category, raw_energy: u32) -> String {
    if *category != Category::CataminStages {
        return raw_energy.to_string();
    }

    if raw_energy < 1000 {
        return format!("{}A", raw_energy);
    }

    if raw_energy < 2000 {
        return format!("{}B", raw_energy % 1000);
    }

    format!("{}C", raw_energy % 1000)
}

fn format_base(anim_id: u32, standard_id: i32) -> (String, String) {
    if anim_id != 0 {
        let calculated_id = anim_id.saturating_sub(2);
        return ("Base Enemy".to_string(), format!("{:03}-E", calculated_id));
    }
    ("Base Img".to_string(), standard_id.to_string())
}

fn format_bool(status: bool, true_string: &str, false_string: &str) -> String {
    if status { true_string.to_string() } else { false_string.to_string() }
}

fn format_respawn(minimum_spawn: u32, maximum_spawn: u32) -> String {
    if minimum_spawn == maximum_spawn {
        return format!("{}f", minimum_spawn);
    }
    format!("{}f ~ {}f", minimum_spawn, maximum_spawn)
}

fn format_boss_bgm(boss_track: i16, init_track: u32, change_percent: u32) -> String {
    if boss_track < 0 || boss_track as u32 == init_track || change_percent == 100 {
        return "-".to_string();
    }
    boss_track.to_string()
}

fn format_time(time_limit: u32) -> String {
    if time_limit == 0 {
        return "-".to_string();
    }
    format!("{}m", time_limit)
}

fn get_skip_status(
    category: &Category,
    map_id: u32,
    lock_registry: &HashMap<u32, LockSkipDataEntry>,
    cpu_setting: &ScatCpuSetting
) -> String {
    if let Some(global_id) = category.global_map_id(map_id)
        && let Some(entry) = lock_registry.get(&global_id)
            && entry.excluded_map_id == global_id {
                return "N/A".to_string();
            }

    if cpu_setting.super_cpu_consume_amount > 0 {
        return format!("{} CPUs", cpu_setting.super_cpu_consume_amount);
    }

    "-".to_string()
}

fn get_map_file(map_id: u32, image_prefix: &str) -> String {
    let prefix_string = if image_prefix.is_empty() { String::new() } else { format!("_{}", image_prefix) };
    format!("mapname{:03}{}.png", map_id, prefix_string)
}

fn get_stage_file(map_id: u32, stage_id: u32, image_prefix: &str) -> String {
    let prefix_string = if image_prefix.is_empty() { String::new() } else { format!("_{}", image_prefix) };
    format!("mapsn{:03}_{:02}{}.png", map_id, stage_id, prefix_string)
}

fn get_story_stage_info(category: &Category, map_id: u32, stage_id: u32) -> Option<(PathBuf, String)> {
    let (folder_name, file_code) = match category {
        Category::EmpireOfCats => ("EC", "ec"),
        Category::IntoTheFuture => ("W", "wc"),
        Category::CatsOfTheCosmos => ("Space", "sc"),
        Category::ZombieOutbreaks => match map_id {
            0..=2 => ("EC", "ec"),
            4..=6 => ("W", "wc"),
            7..=9 => ("Space", "sc"),
            _ => return None,
        },
        _ => return None,
    };

    let image_index = match stage_id {
        0..=45 => {
            let reversed_index = 45 - stage_id;
            tracing::trace!("Reversing story image index: {} -> {}", stage_id, reversed_index);
            reversed_index
        }
        49 | 50 => {
            tracing::trace!("Clamping story image index: {} -> 47", stage_id);
            47
        }
        id => id,
    };

    let directory = Path::new(paths::DIR_CATEGORIES)
        .join(folder_name)
        .join("image")
        .join(format!("{:02}", image_index));

    let file_name = format!("{}0{:02}_n.png", file_code, image_index);

    Some((directory, file_name))
}

fn find_texture(directories: &[PathBuf], files: &[String], languages: &[String]) -> Option<PathBuf> {
    let file_references: Vec<&str> = files.iter().map(|string_val| string_val.as_str()).collect();
    for directory in directories {
        if let Some(resolved_path) = resolver::get(directory, &file_references, languages).into_iter().next() {
            return Some(resolved_path);
        }
    }
    None
}

fn process_texture(image_path: &Path) -> Option<egui::ColorImage> {
    let raw_image = image::open(image_path).ok()?;

    if raw_image.width() <= 1 && raw_image.height() <= 1 {
        return None;
    }

    let cropped_image = autocrop(raw_image.to_rgba8());

    if cropped_image.width() <= 1 && cropped_image.height() <= 1 {
        return None;
    }

    let dimensions = [cropped_image.width() as usize, cropped_image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(dimensions, cropped_image.as_flat_samples().as_slice()))
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    context: &egui::Context,
    ui: &mut egui::Ui,
    stage: &Stage,
    map: &Map,
    languages: &[String],
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    lock_registry: &HashMap<u32, LockSkipDataEntry>,
    cpu_setting: &ScatCpuSetting,
    selected_crown: &mut u8
) {
    let image_prefix = stage.category.image_prefix();

    let map_key = format!("map_img_{:?}_{}", stage.category, stage.map_id);
    let stage_key = format!("stage_img_{:?}_{}_{}", stage.category, stage.map_id, stage.stage_id);

    let base_directory = Path::new(paths::DIR_CATEGORIES);
    let map_directory = paths::map_folder(base_directory, &stage.category, stage.map_id);

    if !texture_cache.contains_key(&map_key) {
        let map_file = get_map_file(stage.map_id, &image_prefix);
        if let Some(texture_path) = find_texture(&[map_directory.clone()], &[map_file], languages)
            && let Some(processed_image) = process_texture(&texture_path) {
                tracing::trace!("Caching map texture: {}", map_key);
                texture_cache.insert(map_key.clone(), context.load_texture(&map_key, processed_image, egui::TextureOptions::LINEAR));
            }
    }

    if !texture_cache.contains_key(&stage_key) {
        let (search_dirs, files) = if let Some((story_dir, story_file)) = get_story_stage_info(&stage.category, stage.map_id, stage.stage_id) {
            (vec![story_dir], vec![story_file])
        } else {
            let stage_directory = paths::stage_folder(base_directory, &stage.category, stage.map_id, stage.stage_id);
            let stage_file = get_stage_file(stage.map_id, stage.stage_id, &image_prefix);
            (vec![stage_directory], vec![stage_file])
        };

        if let Some(texture_path) = find_texture(&search_dirs, &files, languages)
            && let Some(processed_image) = process_texture(&texture_path) {
                tracing::trace!("Caching stage texture: {}", stage_key);
                texture_cache.insert(stage_key.clone(), context.load_texture(&stage_key, processed_image, egui::TextureOptions::LINEAR));
            }
    }

    let mut map_width = 0.0;
    let mut stage_width = 0.0;
    let has_map = texture_cache.contains_key(&map_key);
    let has_stage = texture_cache.contains_key(&stage_key);

    if has_map
        && let Some(cached_texture) = texture_cache.get(&map_key) {
            let texture_size = cached_texture.size_vec2();
            map_width = texture_size.x * (MAP_IMG_HEIGHT / texture_size.y);
        }

    if has_stage
        && let Some(cached_texture) = texture_cache.get(&stage_key) {
            let texture_size = cached_texture.size_vec2();
            stage_width = texture_size.x * (STAGE_IMG_HEIGHT / texture_size.y);
        }

    let max_image_height = MAP_IMG_HEIGHT.max(STAGE_IMG_HEIGHT);

    ui.add_space(TOP_PADDING);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), max_image_height),
        egui::Layout::left_to_right(egui::Align::BOTTOM),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            if has_map {
                if let Some(cached_texture) = texture_cache.get(&map_key) {
                    ui.add(egui::Image::new(cached_texture).fit_to_exact_size(egui::vec2(map_width, MAP_IMG_HEIGHT)));
                }
            } else {
                ui.label(egui::RichText::new(&map.name).strong().size(FALLBACK_MAP_SIZE));
            }

            ui.add_space(IMG_SPACING);

            if has_stage {
                if let Some(cached_texture) = texture_cache.get(&stage_key) {
                    ui.add(egui::Image::new(cached_texture).fit_to_exact_size(egui::vec2(stage_width, STAGE_IMG_HEIGHT)));
                }
            } else {
                ui.label(egui::RichText::new(&stage.name).strong().size(FALLBACK_STAGE_SIZE));
            }
        }
    );

    ui.add_space(BOTTOM_PADDING);
    ui.separator();
    ui.add_space(BOTTOM_PADDING);

    super::crowns::draw(ui, stage, selected_crown);

    ui.label(egui::RichText::new("Information").strong().heading());
    ui.separator();

    let crown_magnification = match *selected_crown {
        1 => map.crown_2_mag.unwrap_or(100),
        2 => map.crown_3_mag.unwrap_or(100),
        3 => map.crown_4_mag.unwrap_or(100),
        _ => map.crown_1_mag.unwrap_or(100),
    };

    let final_hp = if stage.anim_base_id != 0 {
        (stage.base_hp * crown_magnification) / 100
    } else {
        stage.base_hp
    };

    let is_dojo = stage.category == Category::DojoRankingEvents || stage.category == Category::DojoHallOfInitiates;

    let hp_header = if is_dojo { "Time Limit" } else { "Base HP" };
    let hp_value = if is_dojo { format_time(stage.time_limit) } else { final_hp.to_string() };

    let energy_header = if stage.category == Category::CataminStages { "Catamin" } else { "Energy" };
    let energy_value = format_energy(&stage.category, stage.energy);

    let difficulty_value = format_diff(stage.difficulty);
    let continue_value = format_bool(stage.is_no_continues, "No", "Yes");
    let indestructible_value = format_bool(stage.is_base_indestructible, "Active", "-");
    let (base_header, base_value) = format_base(stage.anim_base_id, stage.base_id);
    let respawn_value = format_respawn(stage.min_spawn, stage.max_spawn);
    let boss_bgm_value = format_boss_bgm(stage.boss_track, stage.init_track, stage.bgm_change_percent);
    let skip_value = get_skip_status(&stage.category, stage.map_id, lock_registry, cpu_setting);

    egui::Grid::new("stage_meta_grid")
        .striped(true)
        .spacing([15.0, 8.0])
        .show(ui, |grid| {
            center_header(grid, hp_header);
            center_header(grid, "Width");
            center_header(grid, energy_header);
            center_header(grid, "XP");
            center_header(grid, "Boss Guard");
            center_header(grid, "Max Enemy");
            center_header(grid, "Respawn");
            grid.end_row();

            center_text(grid, hp_value);
            center_text(grid, stage.width.to_string());
            center_text(grid, energy_value);
            center_text(grid, stage.xp.to_string());
            center_text(grid, indestructible_value);
            center_text(grid, stage.max_enemies.to_string());
            center_text(grid, respawn_value);
            grid.end_row();

            center_header(grid, &base_header);
            center_header(grid, "Background");
            center_header(grid, "Music");
            center_header(grid, "Boss Music");
            center_header(grid, "Difficulty");
            center_header(grid, "CPU Skip");
            center_header(grid, "Continues");
            grid.end_row();

            center_text(grid, base_value);
            center_text(grid, stage.background_id.to_string());
            center_text(grid, stage.init_track.to_string());
            center_text(grid, boss_bgm_value);
            center_text(grid, difficulty_value);
            center_text(grid, skip_value);
            center_text(grid, continue_value);
            grid.end_row();
        });
}