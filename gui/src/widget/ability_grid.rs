use iced::widget::Space;
use iced::{Element, Length};

use core::common::game::ABILITY_X;

use super::ability_fallback::ICON_SIZE;

const SCROLLBAR_RESERVE: f32 = 24.0;

pub(crate) fn ability_spacer<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
    Space::new().height(Length::Fixed(height)).into()
}

pub(crate) fn icons_per_row(available_width: f32) -> usize {
    let usable = (available_width - SCROLLBAR_RESERVE).max(ICON_SIZE);
    let slot = ICON_SIZE + ABILITY_X;
    (((usable + ABILITY_X) / slot).floor() as usize).max(1)
}
