use std::collections::HashMap;
use std::path::Path;

use eframe::egui;
use nyanko::chapter::Category;
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;

use core::global::resolver;
use core::global::utils::autocrop;
use core::stage::paths;
use core::stage::registry::{Map, Stage};

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

fn format_energy_cost(category: &Category, raw_energy_cost: u32) -> String {
    if *category != Category::CataminStages {
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

fn get_image_prefix(category: &Category) -> String {
    match category {
        Category::StoriesOfLegend => "n".to_string(),
        Category::RegularEventStages => "s".to_string(),
        Category::CollabStages => "c".to_string(),
        Category::EmpireOfCats => "ec".to_string(),
        Category::IntoTheFuture => "w".to_string(),
        Category::CatsOfTheCosmos => "space".to_string(),
        Category::EventStages => "e".to_string(),
        Category::ContinuationStages => "ex".to_string(),
        Category::DojoHallOfInitiates => "t".to_string(),
        Category::TowersAndCitadels => "v".to_string(),
        Category::DojoRankingEvents => "r".to_string(),
        Category::ChallengeBattle => "m".to_string(),
        Category::UncannyLegends => "na".to_string(),
        Category::CataminStages => "b".to_string(),
        Category::LegendQuest => "d".to_string(),
        Category::ZombieOutbreaks => "z".to_string(),
        Category::GauntletStages => "a".to_string(),
        Category::EnigmaStages => "h".to_string(),
        Category::CollabGauntletStages => "ca".to_string(),
        Category::AkuRealms => "u".to_string(),
        Category::BehemothCulling => "q".to_string(),
        Category::Labyrinth => "l".to_string(),
        Category::ZeroLegends => "nd".to_string(),
        Category::OtherworldColosseum => "sr".to_string(),
        Category::CatclawChampionships => "g".to_string(),
        Category::Unknown(prefix) => {
            let upper = prefix.to_uppercase();
            if upper.starts_with('R') && upper.len() > 1 {
                upper[1..].to_lowercase()
            } else {
                upper.to_lowercase()
            }
        }
    }
}

fn get_cpu_skip_status(
    category: &Category,
    map_id: u32,
    lock_registry: &HashMap<u32, LockSkipDataEntry>,
    cpu_setting: &ScatCpuSetting
) -> String {
    let global_map_id = category.global_map_id(map_id);

    if let Some(map_id_val) = global_map_id {
        if let Some(entry) = lock_registry.get(&map_id_val) {
            if entry.excluded_map_id == map_id_val {
                return "N/A".to_string();
            }
        }
    }

    if cpu_setting.super_cpu_consume_amount > 0 {
        return format!("{} CPUs", cpu_setting.super_cpu_consume_amount);
    }

    "-".to_string()
}

fn get_map_image_filenames(map_id: u32, category: &Category, lang_priority: &[String]) -> Vec<String> {
    let category_lower = get_image_prefix(category);
    let mut filenames = Vec::new();

    for lang in lang_priority {
        filenames.push(format!("mapname{:03}_{}_{}.png", map_id, category_lower, lang));
    }
    filenames.push(format!("mapname{:03}_{}.png", map_id, category_lower));

    filenames
}

fn get_stage_image_filenames(map_id: u32, stage_id: u32, category: &Category, lang_priority: &[String]) -> Vec<String> {
    let category_lower = get_image_prefix(category);
    let mut filenames = Vec::new();

    for lang in lang_priority {
        filenames.push(format!("mapsn{:03}_{:02}_{}_{}.png", map_id, stage_id, category_lower, lang));
    }
    filenames.push(format!("mapsn{:03}_{:02}_{}.png", map_id, stage_id, category_lower));

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
    map_data: &Map,
    lang_priority: &[String],
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    lock_registry: &HashMap<u32, LockSkipDataEntry>,
    cpu_setting: &ScatCpuSetting,
    selected_crown: &mut u8
) {
    let category_formatted = get_image_prefix(&stage_data.category).to_uppercase();

    // Fallback for custom formatted folder names
    let folder_prefix = if category_formatted == "E" {
        "RE".to_string()
    } else if category_formatted == "T" && stage_data.category == Category::DojoHallOfInitiates {
        "RT".to_string()
    } else if category_formatted == "V" && stage_data.category == Category::TowersAndCitadels {
        "RV".to_string()
    } else if category_formatted == "R" && stage_data.category == Category::DojoRankingEvents {
        "RR".to_string()
    } else {
        category_formatted
    };

    let map_dir = Path::new(paths::DIR_STAGES).join(&folder_prefix).join(format!("{:03}", stage_data.map_id));
    let stage_dir = map_dir.join(format!("{:02}", stage_data.stage_id));

    let map_img_key = format!("map_img_{:?}_{}", stage_data.category, stage_data.map_id);
    let stage_img_key = format!("stage_img_{:?}_{}_{}", stage_data.category, stage_data.map_id, stage_data.stage_id);

    if !texture_cache.contains_key(&map_img_key) {
        let possible_files = get_map_image_filenames(stage_data.map_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();

        if let Some(resolved_path) = resolver::get(&map_dir, &refs, lang_priority).first() {
            if let Some(color_img) = process_texture(resolved_path) {
                texture_cache.insert(map_img_key.clone(), egui_context.load_texture(&map_img_key, color_img, egui::TextureOptions::LINEAR));
            }
        }
    }

    if !texture_cache.contains_key(&stage_img_key) {
        let possible_files = get_stage_image_filenames(stage_data.map_id, stage_data.stage_id, &stage_data.category, lang_priority);
        let refs: Vec<&str> = possible_files.iter().map(|s| s.as_str()).collect();

        if let Some(resolved_path) = resolver::get(&stage_dir, &refs, lang_priority).first() {
            if let Some(color_img) = process_texture(resolved_path) {
                texture_cache.insert(stage_img_key.clone(), egui_context.load_texture(&stage_img_key, color_img, egui::TextureOptions::LINEAR));
            }
        }
    }

    let mut map_width = 0.0;
    let mut stage_width = 0.0;

    let has_map = texture_cache.contains_key(&map_img_key);
    let has_stage = texture_cache.contains_key(&stage_img_key);

    if has_map {
        if let Some(map_tex) = texture_cache.get(&map_img_key) {
            let size = map_tex.size_vec2();
            map_width = size.x * (MAP_IMG_HEIGHT / size.y);
        }
    }

    if has_stage {
        if let Some(stage_tex) = texture_cache.get(&stage_img_key) {
            let size = stage_tex.size_vec2();
            stage_width = size.x * (STAGE_IMG_HEIGHT / size.y);
        }
    }

    let max_height = MAP_IMG_HEIGHT.max(STAGE_IMG_HEIGHT);

    ui.add_space(TOP_PADDING);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), max_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            if has_map {
                if let Some(map_tex) = texture_cache.get(&map_img_key) {
                    ui.add(egui::Image::new(map_tex).fit_to_exact_size(egui::vec2(map_width, MAP_IMG_HEIGHT)));
                }
            } else {
                ui.label(egui::RichText::new(&map_data.name).strong().size(18.0));
            }

            ui.add_space(IMG_SPACING);

            if has_stage {
                if let Some(stage_tex) = texture_cache.get(&stage_img_key) {
                    ui.add(egui::Image::new(stage_tex).fit_to_exact_size(egui::vec2(stage_width, STAGE_IMG_HEIGHT)));
                }
            } else {
                ui.label(egui::RichText::new(&stage_data.name).strong().size(18.0));
            }
        }
    );

    ui.add_space(BOTTOM_PADDING);
    ui.separator();
    ui.add_space(BOTTOM_PADDING);

    super::crowns::draw(ui, stage_data, selected_crown);

    ui.strong("General Information");
    ui.separator();

    let crown_mag = match *selected_crown {
        1 => map_data.crown_2_mag.unwrap_or(100),
        2 => map_data.crown_3_mag.unwrap_or(100),
        3 => map_data.crown_4_mag.unwrap_or(100),
        _ => map_data.crown_1_mag.unwrap_or(100),
    } as u32;

    let final_base_hp = if stage_data.anim_base_id != 0 {
        (stage_data.base_hp * crown_mag) / 100
    } else {
        stage_data.base_hp
    };

    let energy_header = if stage_data.category == Category::CataminStages { "Catamin" } else { "Energy" };
    let formatted_energy_value = format_energy_cost(&stage_data.category, stage_data.energy);
    let formatted_difficulty = format_difficulty_level(stage_data.difficulty);
    let formatted_crown = format_crown_display(stage_data.target_crowns, stage_data.max_crowns);
    let formatted_no_continues = format_boolean_status(stage_data.is_no_continues, "Yes", "No");
    let formatted_indestructible = format_boolean_status(stage_data.is_base_indestructible, "Active", "-");
    let (base_header, formatted_base_value) = format_base_display(stage_data.anim_base_id, stage_data.base_id);
    let formatted_global_respawn = format_global_respawn(stage_data.min_spawn, stage_data.max_spawn);
    let formatted_boss_track = format_boss_track(stage_data.boss_track, stage_data.init_track, stage_data.bgm_change_percent);
    let formatted_time_limit = format_time_limit(stage_data.time_limit);
    let formatted_cpu_skip = get_cpu_skip_status(&stage_data.category, stage_data.map_id, lock_registry, cpu_setting);

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

            center_text(grid, final_base_hp.to_string());
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