use iced::{Element, Length};
use iced::widget::{column, container, row, text};

use core::common::text::strip_markdown;

pub fn process_markdown<'a, Message: 'a>(raw_text: &str) -> Element<'a, Message> {
    let content = strip_markdown(raw_text);
    let mut root_column = column![].spacing(5);

    for line in content.lines() {
        let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            root_column = root_column.push(text("").height(Length::Fixed(10.0)));
            continue;
        }

        let mut line_row = row![].spacing(3);
        if leading_spaces > 0 {
            line_row = line_row.push(
                container(text(""))
                    .width(Length::Fixed(leading_spaces as f32 * 6.0))
            );
        }

        if trimmed.starts_with('•') || trimmed.starts_with('-') || trimmed.starts_with('*') {
            let clean_text = trimmed.trim_start_matches(['•', '-', '*']).trim();
            line_row = line_row.push(text("•")).push(text(clean_text.to_string()));
        } else if trimmed.starts_with('#') {
            let clean_text = trimmed.trim_start_matches('#').trim();
            line_row = line_row.push(text(clean_text.to_string()).size(20));
        } else {
            line_row = line_row.push(text(trimmed.to_string()));
        }

        root_column = root_column.push(line_row);
    }

    root_column.into()
}