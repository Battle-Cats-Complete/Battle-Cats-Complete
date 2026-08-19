use std::borrow::Borrow;
use std::fmt::Display;

use iced::alignment::Vertical;
use iced::widget::{container, pick_list, row, text, tooltip};
use iced::{Element, Length};

use crate::app::theme;

const LABEL_SPACING: f32 = 10.0;
const IDLE_PADDING: f32 = 5.0;
const HINT_PADDING: f32 = 6.0;

pub fn combo_row<'a, T, L, V, Message>(
    label: &'a str,
    hint: &'a str,
    options: L,
    selected: Option<V>,
    on_select: Option<impl Fn(T) -> Message + 'a>,
) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + Display + 'a,
    Message: Clone + 'a,
{
    let Some(on_select) = on_select else {
        let idle = container(
            text(selected.map_or_else(String::new, |value| value.to_string()))
                .style(|theme: &iced::Theme| text::Style { color: Some(theme::weak_text_color(theme)) }),
        )
        .padding(IDLE_PADDING)
        .style(theme::combo_box_idle);

        return explained(aligned(row![text(label), idle]), hint);
    };

    let control = pick_list(options, selected, on_select)
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu);

    aligned(row![explained(text(label), hint), control])
}

fn aligned<'a, Message: 'a>(content: iced::widget::Row<'a, Message>) -> Element<'a, Message> {
    content.spacing(LABEL_SPACING).align_y(Vertical::Center).width(Length::Shrink).into()
}

fn explained<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, hint: &'a str) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(hint)).padding(HINT_PADDING).style(container::bordered_box),
        tooltip::Position::Top,
    )
    .into()
}
