use iced::widget::Space;
use iced::{Element, Length};

use super::ability_fallback::ICON_SIZE;

const SCROLLBAR_RESERVE: f32 = 24.0;

pub(crate) fn ability_spacer<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
    Space::new().height(Length::Fixed(height)).into()
}

pub(crate) fn icons_per_row(available_width: f32, spacing: f32) -> usize {
    let usable = (available_width - SCROLLBAR_RESERVE).max(ICON_SIZE);
    let slot = ICON_SIZE + spacing;
    (((usable + spacing) / slot).floor() as usize).max(1)
}
