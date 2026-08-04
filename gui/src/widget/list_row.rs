use iced::widget::button;
use iced::{Border, Color, Element, Length, Theme};

const SELECTED_BORDER_WIDTH: f32 = 2.0;
const HOVER_ALPHA: f32 = 0.15;

pub(crate) fn list_row<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    is_selected: bool,
    filled: bool,
    width: Length,
    on_press: Message,
) -> Element<'a, Message> {
    button(content)
        .on_press(on_press)
        .width(width)
        .padding(0)
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let ext = theme.extended_palette();
            let background = if is_selected {
                palette.primary
            } else if status == button::Status::Hovered {
                Color { a: HOVER_ALPHA, ..palette.text }
            } else if filled {
                ext.background.strong.color
            } else {
                Color::TRANSPARENT
            };

            button::Style {
                background: Some(background.into()),
                text_color: palette.text,
                border: Border::default()
                    .rounded(4.0)
                    .width(if is_selected { SELECTED_BORDER_WIDTH } else { 0.0 })
                    .color(palette.primary),
                ..Default::default()
            }
        })
        .into()
}
