use iced::alignment::Vertical;
use iced::{border, Element, Length, Theme};
use iced::widget::{column, container, row, text};

pub const ICON_SIZE: f32 = 40.0;

pub const ABILITY_TEXT_SIZE: f32 = 13.0;
const ABILITY_SUPERSCRIPT_SIZE: f32 = ABILITY_TEXT_SIZE - 3.0;

pub fn fallback_icon<'a, Message: 'a>(icon_text: &str) -> Element<'a, Message> {
    container(text(icon_text.to_string()).size(10))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.palette();
            container::Style {
                border: border::rounded(0).color(palette.danger).width(1.5),
                ..Default::default()
            }
        })
        .into()
}

pub fn text_with_superscript<'a, Message: 'a>(raw_text: &str) -> Element<'a, Message> {
    let mut lines_col = column![];

    for line in raw_text.split('\n') {
        lines_col = lines_col.push(superscript_line(line));
    }

    lines_col.into()
}

fn superscript_line<'a, Message: 'a>(line: &str) -> Element<'a, Message> {
    if !line.contains('^') {
        return text(line.to_string()).size(ABILITY_TEXT_SIZE).into();
    }

    let mut result_row = row![].align_y(Vertical::Bottom);
    let mut parts = line.split('^');

    if let Some(first) = parts.next() {
        if !first.is_empty() {
            result_row = result_row.push(text(first.to_string()).size(ABILITY_TEXT_SIZE));
        }
    }

    for part in parts {
        if let Some(break_idx) = part.find(' ') {
            let super_str = &part[..break_idx];
            let normal_str = &part[break_idx..];

            if !super_str.is_empty() {
                result_row = result_row.push(text(super_str.to_string()).size(ABILITY_SUPERSCRIPT_SIZE));
            }
            if !normal_str.is_empty() {
                result_row = result_row.push(text(normal_str.to_string()).size(ABILITY_TEXT_SIZE));
            }
        } else if !part.is_empty() {
            result_row = result_row.push(text(part.to_string()).size(ABILITY_SUPERSCRIPT_SIZE));
        }
    }

    result_row.into()
}