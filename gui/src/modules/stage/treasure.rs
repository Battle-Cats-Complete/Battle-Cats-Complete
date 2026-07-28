use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, text, tooltip};
use iced::{font, Alignment, Element, Length};
use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::stage::RewardStructure;

use core::common::formats::{GatyaItemBuy, GatyaItemName};
use core::modules::stage::treasure;
use core::modules::stage::Stage;

use super::icons;

const TREASURE_TABLE_WIDTH: f32 = 345.0;
const MAX_ICON_SIZE: f32 = 32.0;

fn bold_text<'a>(content: impl ToString) -> iced::widget::Text<'a> {
    text(content.to_string()).font(font::Font {
        weight: font::Weight::Bold,
        ..Default::default()
    })
}

fn format_drop_chance(raw_chance: u32, drop_rule: i32) -> String {
    if drop_rule == -3 || drop_rule == -4 {
        return "100%".to_string();
    }
    format!("{}%", raw_chance)
}

fn format_treasure_rule(drop_rule: i32) -> &'static str {
    match drop_rule {
        1 => "Once, Then Unlimited",
        0 => "Unlimited",
        -1 => "Unlimited (%)",
        -3 => "Guaranteed (Once)",
        -4 => "Guaranteed (Unlimited)",
        _ => "Unknown Rule",
    }
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<u32, Handle>>,
}

impl State {
    fn icon(&self, id: u32, path: &std::path::Path) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&id) {
            return Some(cached.clone());
        }

        let handle = icons::load_scaled(path, MAX_ICON_SIZE as u32)?;
        self.icon_cache.borrow_mut().insert(id, handle.clone());
        Some(handle)
    }

    fn item_row<'a>(
        &'a self,
        item_id: u32,
        left_label: String,
        amount_label: String,
        image_path: Option<&std::path::Path>,
        drop_name: String,
    ) -> Element<'a, super::Message> {
        let icon_element: Element<'a, super::Message> = match image_path.and_then(|path| self.icon(item_id, path)) {
            Some(handle) => tooltip(
                iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)),
                container(text(drop_name.clone())).padding(6).style(container::bordered_box),
                tooltip::Position::Top,
            ).into(),
            None => text(drop_name).into(),
        };

        row![
            container(text(left_label)).width(Length::FillPortion(1)).center_x(Length::Fill),
            container(icon_element).width(Length::FillPortion(1)).center_x(Length::Fill),
            container(text(amount_label)).width(Length::FillPortion(1)).center_x(Length::Fill),
        ]
            .align_y(Alignment::Center)
            .into()
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        item_buys: &'a HashMap<u32, GatyaItemBuy>,
        item_names: &'a HashMap<usize, GatyaItemName>,
        drop_charas: &'a HashMap<u32, u32>,
        unit_buys: &'a HashMap<u32, UnitBuy>,
        langs: &'a [String],
    ) -> Element<'a, super::Message> {
        match &stage.rewards {
            RewardStructure::Treasure { drop_rule, drops } => {
                let valid_drops: Vec<_> = drops.iter().filter(|drop| drop.chance > 0).collect();
                if valid_drops.is_empty() {
                    return iced::widget::space().into();
                }

                let header = bold_text(format!("Treasure | {}", format_treasure_rule(*drop_rule))).size(18);

                let mut grid = column![
                    row![
                        container(bold_text("Chance").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                        container(bold_text("Item").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                        container(bold_text("Amount").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                    ]
                ].spacing(4);

                for drop in valid_drops {
                    let resolved = treasure::resolve_drop(drop.item_id, drop.amount, item_buys, item_names, drop_charas, unit_buys, langs);
                    grid = grid.push(self.item_row(
                        drop.item_id,
                        format_drop_chance(drop.chance, *drop_rule),
                        resolved.amount_display.clone(),
                        resolved.image_path.as_deref(),
                        resolved.name.clone(),
                    ));
                }

                column![header, iced::widget::rule::horizontal(1), grid]
                    .spacing(6)
                    .width(Length::Fixed(TREASURE_TABLE_WIDTH))
                    .into()
            }
            RewardStructure::Timed(timed_scores) => {
                if timed_scores.is_empty() {
                    return iced::widget::space().into();
                }

                let header = bold_text("Timed Score Rewards").size(18);

                let mut grid = column![
                    row![
                        container(bold_text("Score").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                        container(bold_text("Item").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                        container(bold_text("Amount").size(13)).width(Length::FillPortion(1)).center_x(Length::Fill),
                    ]
                ].spacing(4);

                for score in timed_scores {
                    let resolved = treasure::resolve_drop(score.item_id, score.amount, item_buys, item_names, drop_charas, unit_buys, langs);
                    grid = grid.push(self.item_row(
                        score.item_id,
                        score.score.to_string(),
                        resolved.amount_display.clone(),
                        resolved.image_path.as_deref(),
                        resolved.name.clone(),
                    ));
                }

                column![header, iced::widget::rule::horizontal(1), grid]
                    .spacing(6)
                    .width(Length::Fixed(TREASURE_TABLE_WIDTH))
                    .into()
            }
            RewardStructure::None => iced::widget::space().into(),
        }
    }
}
