use iced::alignment::Vertical;
use iced::widget::{row, text, text_input};
use iced::{Element, Length};

use crate::app::theme;

const LABEL_WIDTH: f32 = 110.0;
const INPUT_WIDTH: f32 = 55.0;
const PLACEHOLDER: &str = "Any";

pub(crate) fn range_row<'a, Message: Clone + 'a>(
    label: &str,
    min: &'a str,
    max: &'a str,
    on_min: impl Fn(String) -> Message + 'a,
    on_max: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    range_row_hinted(label, min, max, PLACEHOLDER, PLACEHOLDER, on_min, on_max)
}

pub(crate) fn range_row_hinted<'a, Message: Clone + 'a>(
    label: &str,
    min: &'a str,
    max: &'a str,
    min_hint: &str,
    max_hint: &str,
    on_min: impl Fn(String) -> Message + 'a,
    on_max: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(format!("{}:", label)).width(Length::Fixed(LABEL_WIDTH)),
        text_input(min_hint, min).on_input(on_min).width(Length::Fixed(INPUT_WIDTH)).style(theme::rounded_input),
        text("~"),
        text_input(max_hint, max).on_input(on_max).width(Length::Fixed(INPUT_WIDTH)).style(theme::rounded_input),
    ]
        .spacing(4)
        .align_y(Vertical::Center)
        .into()
}
