use iced::alignment::Vertical;
use iced::{border, Element, Length, Theme};
use iced::widget::{container, row, text};

pub fn fallback_icon<'a, Message: 'a>(icon_text: &str) -> Element<'a, Message> {
    container(text(icon_text.to_string()).size(10))
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
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
    if !raw_text.contains('^') {
        return text(raw_text.to_string()).into();
    }

    let mut result_row = row![].align_y(Vertical::Bottom);
    let mut parts = raw_text.split('^');

    if let Some(first) = parts.next() {
        if !first.is_empty() {
            result_row = result_row.push(text(first.to_string()));
        }
    }

    for part in parts {
        if let Some(break_idx) = part.find([' ', '\n']) {
            let super_str = &part[..break_idx];
            let normal_str = &part[break_idx..];

            if !super_str.is_empty() {
                result_row = result_row.push(text(super_str.to_string()).size(10));
            }
            if !normal_str.is_empty() {
                result_row = result_row.push(text(normal_str.to_string()));
            }
        } else {
            if !part.is_empty() {
                result_row = result_row.push(text(part.to_string()).size(10));
            }
        }
    }

    result_row.into()
}