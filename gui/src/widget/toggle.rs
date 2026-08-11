use iced::alignment::Vertical;
use iced::mouse::Interaction;
use iced::widget::{mouse_area, row, toggler};
use iced::Element;

use crate::app::theme;

const LABEL_SPACING: f32 = 10.0;

pub fn toggle_label<'a, Message: Clone + 'a>(
    label: impl Into<Element<'a, Message>>,
    value: bool,
    on_toggle: Option<impl Fn(bool) -> Message + 'a>,
) -> Element<'a, Message> {
    let Some(on_toggle) = on_toggle else {
        return label.into();
    };

    mouse_area(label.into())
        .on_press(on_toggle(!value))
        .interaction(Interaction::Pointer)
        .into()
}

pub fn toggle_row<'a, Message: Clone + 'a>(
    value: bool,
    label: impl Into<Element<'a, Message>>,
    on_toggle: Option<impl Fn(bool) -> Message + 'a>,
) -> Element<'a, Message> {
    let Some(on_toggle) = on_toggle else {
        return row![toggler(value).style(theme::ios_toggle), label.into()]
            .spacing(LABEL_SPACING)
            .align_y(Vertical::Center)
            .into();
    };

    let flipped = on_toggle(!value);

    row![
        toggler(value).on_toggle(on_toggle).style(theme::ios_toggle),
        mouse_area(label.into()).on_press(flipped).interaction(Interaction::Pointer),
    ]
    .spacing(LABEL_SPACING)
    .align_y(Vertical::Center)
    .into()
}
