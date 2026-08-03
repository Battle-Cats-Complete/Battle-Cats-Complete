use iced::alignment::Vertical;
use iced::{Color, Element, Length, Theme};
use iced::widget::{column, row, text, Row, Space};

const ABILITY_TEXT_SIZE: f32 = 13.0;
const ABILITY_SUPERSCRIPT_SIZE: f32 = ABILITY_TEXT_SIZE - 3.0;
const SUPERSCRIPT_ALPHA: f32 = 0.7;
const SUPERSCRIPT_SPACING: f32 = 1.25;

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

    let mut result_row = row![].align_y(Vertical::Top);
    let mut parts = line.split('^');
    let mut has_content = false;

    if let Some(first) = parts.next()
        && !first.is_empty() {
        result_row = result_row.push(text(first.to_string()).size(ABILITY_TEXT_SIZE));
        has_content = true;
    }

    for part in parts {
        if let Some(break_idx) = part.find(' ') {
            let super_str = &part[..break_idx];
            let normal_str = &part[break_idx..];

            if !super_str.is_empty() {
                result_row = push_superscript(result_row, has_content, super_str);
                has_content = true;
            }
            if !normal_str.is_empty() {
                result_row = result_row.push(text(normal_str.to_string()).size(ABILITY_TEXT_SIZE));
                has_content = true;
            }
        } else if !part.is_empty() {
            result_row = push_superscript(result_row, has_content, part);
            has_content = true;
        }
    }

    result_row.into()
}

fn push_superscript<'a, Message: 'a>(result_row: Row<'a, Message>, has_content: bool, super_str: &str) -> Row<'a, Message> {
    let result_row = if has_content {
        result_row.push(Space::new().width(Length::Fixed(SUPERSCRIPT_SPACING)))
    } else {
        result_row
    };

    result_row.push(weak_superscript_text(super_str))
}

fn weak_superscript_text<'a, Message: 'a>(super_str: &str) -> Element<'a, Message> {
    text(super_str.to_string())
        .size(ABILITY_SUPERSCRIPT_SIZE)
        .style(|theme: &Theme| text::Style {
            color: Some(Color { a: SUPERSCRIPT_ALPHA, ..theme.palette().text }),
        })
        .into()
}
