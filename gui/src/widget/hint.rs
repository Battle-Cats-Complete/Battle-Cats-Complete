use iced::widget::{container, text, tooltip};
use iced::Element;

const HINT_PADDING: f32 = 6.0;

pub fn hover_hint<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    hint: &'a str,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(hint)).padding(HINT_PADDING).style(container::bordered_box),
        tooltip::Position::Top,
    )
    .into()
}
