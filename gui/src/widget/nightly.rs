use iced::alignment::Vertical;
use iced::widget::{container, row, text, Text};
use iced::{Element, Length};

use crate::common::fonts;

const MOON_GAP: f32 = 6.0;

pub(crate) fn nightly_label<'a, M: 'a>(label: &'a str, size: f32) -> Element<'a, M> {
    let moon = |glyph: &'a str| -> Text<'a> {
        text(glyph)
            .font(fonts::MISC_SYMBOLS)
            .size(size)
            .line_height(fonts::MISC_SYMBOLS_LINE_HEIGHT)
    };

    container(
        row![moon(fonts::MOON_OPEN), text(label).size(size), moon(fonts::MOON_CLOSE)]
            .spacing(MOON_GAP)
            .align_y(Vertical::Center),
    )
        .center_x(Length::Fill)
        .into()
}
