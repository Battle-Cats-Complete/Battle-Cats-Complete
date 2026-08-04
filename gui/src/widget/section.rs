use iced::widget::{column, container, rule};
use iced::{Element, Length};

use crate::app::theme;

const TITLE_SIZE: f32 = 18.0;
const SUBTITLE_SIZE: f32 = 14.0;
const RULE_HEIGHT: f32 = 1.0;
const SPACING: f32 = 6.0;

pub(crate) fn section<'a, Message: 'a>(
    title: impl ToString,
    rule_width: Length,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        theme::bold_text(title).size(TITLE_SIZE),
        container(rule::horizontal(RULE_HEIGHT)).width(rule_width),
        content.into(),
    ]
        .spacing(SPACING)
        .into()
}

pub(crate) fn subsection<'a, Message: 'a>(title: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![theme::bold_text(title).size(SUBTITLE_SIZE), content.into()]
        .spacing(SPACING)
        .into()
}
