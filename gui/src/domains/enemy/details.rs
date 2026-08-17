use iced::widget::{column, scrollable, text};
use iced::{Element, Length};

use core::domains::enemy::scanner::EnemyEntry;

use crate::widget::smooth_scroll;

use super::Message;

const HEADING_SIZE: f32 = 20.0;
const SECTION_SPACING: f32 = 16.0;
const HEADING_BODY_GAP: f32 = 8.0;

pub(super) fn view<'a>(enemy: &'a EnemyEntry) -> Element<'a, Message> {
    let description = Some(enemy.description.join("\n"))
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| "No description available".to_string());

    let description_section = column![
        text("Description").size(HEADING_SIZE),
        text(description),
    ]
        .spacing(HEADING_BODY_GAP)
        .width(Length::Fill);

    let content = column![description_section]
        .spacing(SECTION_SPACING)
        .width(Length::Fill);

    smooth_scroll(scrollable(content)).into()
}
