use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, text};
use iced::{border, Background, Color, Element, Length, Theme};

use crate::app::theme;

pub const ICON_SIZE: f32 = 40.0;

const LABEL_SIZE: f32 = 10.0;
const BORDER_WIDTH: f32 = 1.5;
const BORDER_SHADE: f32 = 0.35;

pub fn fallback_icon<'a, Message: 'a>(icon_text: &str) -> Element<'a, Message> {
    container(text(icon_text.to_string()).size(LABEL_SIZE))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(|theme: &Theme| {
            let palette = theme.palette();

            container::Style {
                background: Some(Background::Color(palette.danger)),
                text_color: Some(Color::WHITE),
                border: border::rounded(0)
                    .color(theme::darken_color(palette.danger, BORDER_SHADE))
                    .width(BORDER_WIDTH),
                ..Default::default()
            }
        })
        .into()
}
