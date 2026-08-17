use std::path::PathBuf;

use iced::Element;
use image::{imageops, Pixel, RgbaImage};

use core::domains::enemy::filter::evaluation::entity_passes_filter;
use core::domains::enemy::filter::EnemyFilterState;
use core::domains::enemy::scanner::EnemyEntry;

use crate::common::udi_loader::Composite;
use crate::widget::roster_list::{self, Roster};

const ENEMY_ICON_SCALE_FACTOR: f32 = 2.6;
const ENEMY_ICON_OFFSET_X: i64 = 8;
const ENEMY_SHADOW_MARGIN: u32 = 8;

pub(super) type State = roster_list::State<EnemyRoster>;
pub(super) type Message = roster_list::Message;

pub(super) struct EnemyRoster;

impl Roster for EnemyRoster {
    type Entry = EnemyEntry;
    type Filter = EnemyFilterState;

    const SCROLLABLE_ID: &'static str = "enemy-banner-list";
    const LABEL: &'static str = "enemies";
    const NOUN: &'static str = "Enemies";
    const COMPOSITE: Composite = composite_icon;

    fn id(entry: &EnemyEntry) -> u32 {
        entry.id
    }

    fn image_path(entry: &EnemyEntry) -> Option<PathBuf> {
        entry.icon_path.clone()
    }

    fn passes_filter(entry: &EnemyEntry, filter: &EnemyFilterState) -> bool {
        entity_passes_filter(entry, filter)
    }

    fn matches_query(entry: &EnemyEntry, query: &str) -> bool {
        let is_id_search = query.chars().next().is_some_and(|c| c.is_ascii_digit());

        (is_id_search && entry.id_str().to_lowercase().contains(query))
            || entry.name.to_lowercase().contains(query)
    }

    fn tooltip<'a>(entry: &EnemyEntry) -> Element<'a, Message> {
        roster_list::tooltip_table([
            ("ID", entry.id_str()),
            ("Name", entry.display_name()),
        ])
    }
}

fn composite_icon(path: &PathBuf, background: &RgbaImage) -> Option<(u32, u32, Vec<u8>)> {
    if !path.exists() {
        return None;
    }

    let Ok(opened) = image::open(path) else { return None; };
    let mut unit_img = opened.to_rgba8();

    let scaled_w = (unit_img.width() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
    let scaled_h = (unit_img.height() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
    unit_img = imageops::resize(&unit_img, scaled_w, scaled_h, imageops::FilterType::Lanczos3);

    let mut final_image = background.clone();
    let bg_w = final_image.width() as i64;
    let bg_h = final_image.height() as i64;
    let h = unit_img.height();

    let shadow_cutoff = h.saturating_sub(ENEMY_SHADOW_MARGIN);
    let mut min_y = h;
    let mut max_y = 0;
    let mut found_solid = false;

    for (_x, y, pixel) in unit_img.enumerate_pixels() {
        if y < shadow_cutoff && pixel[3] > 150 {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            found_solid = true;
        }
    }

    let center_y = if found_solid { (min_y + max_y) as i64 / 2 } else { h as i64 / 2 };
    let offset_y = (bg_h / 2) - center_y;

    for (x, y, pixel) in unit_img.enumerate_pixels() {
        let dest_x = ENEMY_ICON_OFFSET_X + x as i64;
        let dest_y = offset_y + y as i64;

        if dest_x >= 0 && dest_x < bg_w && dest_y >= 0 && dest_y < bg_h {
            let bg_pixel = final_image.get_pixel_mut(dest_x as u32, dest_y as u32);
            let is_black_border = bg_pixel[0] < 25 && bg_pixel[1] < 25 && bg_pixel[2] < 25 && bg_pixel[3] > 200;
            if !is_black_border {
                bg_pixel.blend(pixel);
            }
        }
    }

    let target_h = 50;
    let ratio = target_h as f32 / final_image.height() as f32;
    let target_w = (final_image.width() as f32 * ratio) as u32;
    let resized = imageops::resize(&final_image, target_w, target_h, imageops::FilterType::Lanczos3);

    Some((resized.width(), resized.height(), resized.into_raw()))
}
