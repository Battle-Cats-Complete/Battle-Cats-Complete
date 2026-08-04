use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, rule};
use iced::{Alignment, Element, Length};
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;
use nyanko::chapter::Category;

use core::modules::stage::{paths, Map, Stage};

use crate::app::theme;
use crate::common::item_icon;
use crate::widget::{grid_header, grid_value};

use super::section::section;

const MAP_IMG_HEIGHT: f32 = 50.0;
const STAGE_IMG_HEIGHT: f32 = 35.0;
const BANNER_GAP: f32 = 12.0;
const RULE_HEIGHT: f32 = 1.0;
const GRID_SPACING: f32 = 4.0;
const FALLBACK_NAME_SIZE: f32 = 32.0;

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
    cpu_setting: &ScatCpuSetting,
) -> String {
    if let Some(global_id) = category.global_map_id(map_id)
        && let Some(entry) = lock_registry.get(&global_id)
        && entry.excluded_map_id == global_id
    {
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
        0..=45 => 45 - stage_id,
        49 | 50 => 47,
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
        if let Some(resolved_path) = core::common::resolver::get(directory, &file_references, languages).into_iter().next() {
            return Some(resolved_path);
        }
    }
    None
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<String, (Handle, u32, u32)>>,
}

impl State {
    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn texture(&self, key: &str, path: &Path) -> Option<(Handle, u32, u32)> {
        if let Some(cached) = self.icon_cache.borrow().get(key) {
            return Some(cached.clone());
        }

        let loaded = item_icon::load_cropped(path)?;
        self.icon_cache.borrow_mut().insert(key.to_string(), loaded.clone());
        Some(loaded)
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        langs: &[String],
        lock_registry: &HashMap<u32, LockSkipDataEntry>,
        cpu_setting: &ScatCpuSetting,
        selected_crown: u8,
    ) -> Element<'a, super::Message> {
        let image_prefix = stage.category.image_prefix();

        let map_key = format!("map_img_{:?}_{}", stage.category, stage.map_id);
        let stage_key = format!("stage_img_{:?}_{}_{}", stage.category, stage.map_id, stage.stage_id);

        let base_directory = Path::new(paths::DIR_CATEGORIES);
        let map_directory = paths::map_folder(base_directory, &stage.category, stage.map_id);

        let map_texture = find_texture(&[map_directory], &[get_map_file(stage.map_id, &image_prefix)], langs)
            .and_then(|path| self.texture(&map_key, &path));

        let (story_dirs, story_files) = if let Some((story_dir, story_file)) = get_story_stage_info(&stage.category, stage.map_id, stage.stage_id) {
            (vec![story_dir], vec![story_file])
        } else {
            let stage_directory = paths::stage_folder(base_directory, &stage.category, stage.map_id, stage.stage_id);
            (vec![stage_directory], vec![get_stage_file(stage.map_id, stage.stage_id, &image_prefix)])
        };

        let stage_texture = find_texture(&story_dirs, &story_files, langs)
            .and_then(|path| self.texture(&stage_key, &path));

        let banner_row = row![
            banner_element(map_texture, MAP_IMG_HEIGHT, &map.name),
            banner_element(stage_texture, STAGE_IMG_HEIGHT, &stage.name),
        ]
            .spacing(BANNER_GAP)
            .align_y(Alignment::End);

        let crown_row = super::crowns::view(stage, selected_crown);

        let crown_magnification = match selected_crown {
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

        let headers_row_1 = row![
            grid_header(hp_header),
            grid_header("Width"),
            grid_header(energy_header),
            grid_header("XP"),
            grid_header("Boss Guard"),
            grid_header("Max Enemy"),
            grid_header("Respawn"),
        ].spacing(GRID_SPACING);

        let width_value = stage.width.to_string();
        let xp_value = stage.xp.to_string();
        let max_enemies_value = stage.max_enemies.to_string();

        let values_row_1 = row![
            grid_value(hp_header, &hp_value),
            grid_value("Width", &width_value),
            grid_value(energy_header, &energy_value),
            grid_value("XP", &xp_value),
            grid_value("Boss Guard", &indestructible_value),
            grid_value("Max Enemy", &max_enemies_value),
            grid_value("Respawn", &respawn_value),
        ].spacing(GRID_SPACING);

        let headers_row_2 = row![
            grid_header(&base_header),
            grid_header("Background"),
            grid_header("Music"),
            grid_header("Boss Music"),
            grid_header("Difficulty"),
            grid_header("CPU Skip"),
            grid_header("Continues"),
        ].spacing(GRID_SPACING);

        let background_value = stage.background_id.to_string();
        let music_value = stage.init_track.to_string();

        let values_row_2 = row![
            grid_value(&base_header, &base_value),
            grid_value("Background", &background_value),
            grid_value("Music", &music_value),
            grid_value("Boss Music", &boss_bgm_value),
            grid_value("Difficulty", &difficulty_value),
            grid_value("CPU Skip", &skip_value),
            grid_value("Continues", &continue_value),
        ].spacing(GRID_SPACING);

        let grids = column![headers_row_1, values_row_1, headers_row_2, values_row_2].spacing(GRID_SPACING);

        column![
            banner_row,
            rule::horizontal(RULE_HEIGHT),
            crown_row,
            section("Information", Length::Fixed(super::CONTENT_WIDTH), grids),
        ]
            .spacing(8)
            .into()
    }
}

fn banner_element<'a, Message: 'a>(texture: Option<(Handle, u32, u32)>, target_height: f32, fallback_name: &str) -> Element<'a, Message> {
    texture.map_or_else(
        || container(theme::bold_text(fallback_name).size(FALLBACK_NAME_SIZE)).into(),
        |(handle, width, height)| {
            let display_width = width as f32 * (target_height / height as f32);
            iced_image(handle).width(Length::Fixed(display_width)).height(Length::Fixed(target_height)).into()
        },
    )
}
