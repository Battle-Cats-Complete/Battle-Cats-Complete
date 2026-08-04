use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use iced::widget::{column, row, text};
use iced::Element;
use image::{imageops, RgbaImage};

use core::common::gfx;
use core::modules::cat::filter::{evaluation, CatFilterState};
use core::modules::cat::scanner::CatEntry;

use crate::common::udi_loader::Composite;
use crate::widget::roster_list::{self, Roster};

pub(super) type State = roster_list::State<CatRoster>;
pub(super) type Message = roster_list::Message;

pub(super) struct CatRoster;

impl Roster for CatRoster {
    type Entry = CatEntry;
    type Filter = CatFilterState;

    const SCROLLABLE_ID: &'static str = "cat-banner-list";
    const LABEL: &'static str = "cats";
    const COMPOSITE: Composite = composite_banner;

    fn id(entry: &CatEntry) -> u32 {
        entry.id
    }

    fn image_path(entry: &CatEntry) -> Option<PathBuf> {
        entry.image_path.clone()
    }

    fn passes_filter(entry: &CatEntry, filter: &CatFilterState) -> bool {
        evaluation::entity_passes_filter(entry, filter)
    }

    fn matches_query(entry: &CatEntry, query: &str) -> bool {
        entry.base_id_str().contains(query)
            || entry.names.iter().flatten().any(|name| name.to_lowercase().contains(query))
    }

    fn tooltip<'a>(entry: &CatEntry) -> Element<'a, Message> {
        let mut content = column![
            row![text("[ID]").size(11), text(entry.base_id_str())].spacing(4)
        ].spacing(2);

        let labels = ["Normal", "Evolved", "True", "Ultra"];
        for (i, label) in labels.iter().enumerate() {
            if !entry.forms[i] { continue; }
            content = content.push(row![text(format!("[{}]", label)).size(11), text(entry.display_name(i))].spacing(4));
        }

        content.into()
    }
}

fn composite_banner(path: &PathBuf, background: &RgbaImage) -> Option<(u32, u32, Vec<u8>)> {
    for _ in 0..3 {
        if !path.exists() {
            return None;
        }

        let Ok(opened) = image::open(path) else {
            thread::sleep(Duration::from_millis(50));
            continue;
        };

        let mut unit_img = opened.to_rgba8();
        let mut final_image = background.clone();
        let bg_w = final_image.width() as i64;
        let bg_h = final_image.height() as i64;
        let (w, h) = unit_img.dimensions();
        let is_transparent_unit = w > 311 && h > 2 && unit_img.get_pixel(311, 2)[3] == 0;

        let (x, y) = if is_transparent_unit {
            (-3, 9)
        } else {
            unit_img = gfx::autocrop(unit_img);
            let unit_w = unit_img.width() as i64;
            let unit_h = unit_img.height() as i64;
            ((bg_w - unit_w) / 2, (bg_h - unit_h) / 2)
        };

        imageops::overlay(&mut final_image, &unit_img, x, y);

        const TARGET_H: u32 = 100;
        let ratio = TARGET_H as f32 / final_image.height() as f32;
        let target_w = (final_image.width() as f32 * ratio) as u32;
        let resized = imageops::resize(&final_image, target_w, TARGET_H, imageops::FilterType::Lanczos3);

        return Some((resized.width(), resized.height(), resized.into_raw()));
    }
    None
}
