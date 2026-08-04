use iced::alignment::{Horizontal, Vertical};
use iced::theme::palette::{darken, lighten, mix};
use iced::{border, Color, Element, Length, Theme};
use iced::widget::{container, text, tooltip};

use crate::widget::text_with_superscript;

const CELL_WIDTH: f32 = 82.0;
const CELL_HEIGHT: f32 = 28.0;

const CELL_TEXT_SIZE: f32 = 13.0;
const TOOLTIP_TEXT_SIZE: f32 = 14.0;
const CELL_BORDER_WIDTH: f32 = 2.0;
const CELL_RADIUS: f32 = 4.0;
const CELL_CLIP_INSET: f32 = CELL_BORDER_WIDTH;
const HEADER_SHADE: f32 = 0.08;
const VALUE_SHADE: f32 = 0.1;

fn cell_colors(theme: &Theme) -> (Color, Color, Color) {
    let background = theme.palette().background;
    let header = darken(background, HEADER_SHADE);
    let value = lighten(background, VALUE_SHADE);
    let border_color = mix(header, value, 0.5);

    (header, value, border_color)
}

fn cell<'a, Message: 'a>(content: Element<'a, Message>, is_header: bool) -> Element<'a, Message> {
    let window = container(content)
        .width(Length::Fixed(CELL_WIDTH - CELL_CLIP_INSET * 2.0))
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .clip(true);

    container(window)
        .width(Length::Fixed(CELL_WIDTH))
        .height(Length::Fixed(CELL_HEIGHT))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |theme: &Theme| {
            let (header, value, border_color) = cell_colors(theme);

            container::Style {
                background: Some(if is_header { header } else { value }.into()),
                border: border::rounded(CELL_RADIUS).color(border_color).width(CELL_BORDER_WIDTH),
                ..Default::default()
            }
        })
        .into()
}

fn with_tooltip<'a, Message: 'a>(content: Element<'a, Message>, header: &str, value: &str) -> Element<'a, Message> {
    let tooltip_content = text_with_superscript(&format!("{header}: {value}"), TOOLTIP_TEXT_SIZE);

    tooltip(
        content,
        container(tooltip_content).padding(6).style(container::bordered_box),
        tooltip::Position::Top,
    ).into()
}

pub fn grid_header<'a, Message: 'a>(label: &str) -> Element<'a, Message> {
    cell(text(label.to_string()).size(CELL_TEXT_SIZE).into(), true)
}

pub fn grid_value<'a, Message: 'a>(header: &str, value: &str) -> Element<'a, Message> {
    let content = cell(text(value.to_string()).size(CELL_TEXT_SIZE).into(), false);

    with_tooltip(content, header, value)
}

pub fn grid_frames<'a, Message: 'a>(header: &str, frames: i32) -> Element<'a, Message> {
    let raw = frame_text(frames);
    let content = cell(text_with_superscript(&raw, CELL_TEXT_SIZE), false);

    with_tooltip(content, header, &raw)
}

fn frame_text(frames: i32) -> String {
    let seconds = frames as f32 / 30.0;

    format!("{seconds:.2}s^{frames}f")
}
