use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::{column, container, image as iced_image, row, text, tooltip};
use iced::{font, Alignment, Color, Element, Length, Theme};

use core::common::formats::{GatyaItemBuy, GatyaItemName};
use core::modules::stage::materials;
use core::modules::stage::{Map, Stage};

use super::icons;

const MAT_TABLE_WIDTH: f32 = 345.0;
const MAX_ICON_SIZE: f32 = 32.0;
const COL_SPACING: f32 = 8.0;

fn bold_text<'a>(content: impl ToString) -> iced::widget::Text<'a> {
    text(content.to_string()).font(font::Font {
        weight: font::Weight::Bold,
        ..Default::default()
    })
}

pub fn has_drops(_stage: &Stage, map: &Map) -> bool {
    map.drop_items.as_ref().is_some_and(|drops| drops.material_drops.iter().any(|&count| count > 0))
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<u32, iced::widget::image::Handle>>,
}

impl State {
    fn icon(&self, id: u32, path: &std::path::Path) -> Option<iced::widget::image::Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&id) {
            return Some(cached.clone());
        }

        let handle = icons::load_scaled(path, MAX_ICON_SIZE as u32)?;
        self.icon_cache.borrow_mut().insert(id, handle.clone());
        Some(handle)
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        selected_crown: u8,
        item_buys: &'a HashMap<u32, GatyaItemBuy>,
        item_names: &'a HashMap<usize, GatyaItemName>,
        langs: &'a [String],
    ) -> Element<'a, super::Message> {
        if !has_drops(stage, map) {
            return iced::widget::space().into();
        }

        let Some(drops) = &map.drop_items else {
            return iced::widget::space().into();
        };

        let stage_idx = stage.stage_id as usize;
        let base_amount = drops.stage_drops.get(stage_idx).copied().unwrap_or(0);
        let multiplier = drops.crown_multipliers.get(selected_crown as usize).copied().unwrap_or(1.0);
        let final_amount = (base_amount as f32 * multiplier).round() as u32;

        let header = bold_text(format!("Materials | Amount: {} ({}×{:.2})", final_amount, base_amount, multiplier)).size(18);

        let base_mats = &drops.material_drops[0..8];
        let z_mats = &drops.material_drops[8..16];

        let mut grid_col = column![].spacing(6);

        if base_mats.iter().any(|&count| count > 0) {
            grid_col = self.push_chunks(grid_col, base_mats, 0, item_buys, item_names, langs);
        }
        if z_mats.iter().any(|&count| count > 0) {
            grid_col = self.push_chunks(grid_col, z_mats, 8, item_buys, item_names, langs);
        }

        column![header, iced::widget::rule::horizontal(1), grid_col]
            .spacing(6)
            .width(Length::Fixed(MAT_TABLE_WIDTH))
            .into()
    }

    fn push_chunks<'a>(
        &'a self,
        mut col: iced::widget::Column<'a, super::Message>,
        chances: &'a [u32],
        offset: usize,
        item_buys: &'a HashMap<u32, GatyaItemBuy>,
        item_names: &'a HashMap<usize, GatyaItemName>,
        langs: &'a [String],
    ) -> iced::widget::Column<'a, super::Message> {
        let column_width = (MAT_TABLE_WIDTH - (COL_SPACING * 3.0)) / 4.0;

        for (chunk_idx, chunk) in chances.chunks(4).enumerate() {
            let chunk_offset = offset + (chunk_idx * 4);

            let resolved: Vec<_> = chunk.iter().enumerate()
                .map(|(i, &chance)| (materials::resolve(chunk_offset + i, chance, item_buys, item_names, langs), chance))
                .collect();

            let mut name_row = row![].spacing(COL_SPACING);
            let mut icon_row = row![].spacing(COL_SPACING);
            let mut pct_row = row![].spacing(COL_SPACING);

            for (idx, (drop, chance)) in resolved.iter().enumerate() {
                let material_id = 10000 + (chunk_offset + idx) as u32;

                name_row = name_row.push(
                    container(bold_text(drop.name.clone()).size(12))
                        .width(Length::Fixed(column_width))
                        .center_x(Length::Fill)
                );

                let icon_element: Element<'a, super::Message> = match &drop.image_path {
                    Some(path) => match self.icon(material_id, path) {
                        Some(handle) => tooltip(
                            iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)),
                            container(text(drop.name.clone())).padding(6).style(container::bordered_box),
                            tooltip::Position::Top,
                        ).into(),
                        None => text(drop.name.clone()).size(11).into(),
                    },
                    None => text(drop.name.clone()).size(11).into(),
                };

                icon_row = icon_row.push(container(icon_element).width(Length::Fixed(column_width)).center_x(Length::Fill));

                let pct_color = if *chance > 0 { None } else { Some(Color::from_rgb8(80, 80, 80)) };
                pct_row = pct_row.push(
                    container(text(format!("{}%", chance)).style(move |theme: &Theme| text::Style {
                        color: pct_color.or(Some(theme.palette().text)),
                    }))
                        .width(Length::Fixed(column_width))
                        .center_x(Length::Fill)
                );
            }

            col = col.push(column![name_row, icon_row, pct_row].spacing(4).align_x(Alignment::Center));
        }

        col
    }
}
