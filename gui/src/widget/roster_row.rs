use iced::widget::image::Handle;
use iced::widget::{button, container, image as iced_image, row, tooltip};
use iced::{Border, Color, Element, Length, Theme};

use crate::common::row_window::ROW_HEIGHT;

const TOOLTIP_PADDING: f32 = 8.0;
const SELECTED_BORDER_WIDTH: f32 = 2.0;
const HOVER_ALPHA: f32 = 0.15;

pub(crate) fn roster_row<'a, Message: Clone + 'a>(
    handle: Handle,
    is_selected: bool,
    on_press: Message,
    tooltip_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let image = iced_image(handle).height(Length::Fixed(ROW_HEIGHT));

    let image_button = button(row![image].width(Length::Fill))
        .on_press(on_press)
        .width(Length::Fill)
        .padding(0)
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let background = if is_selected {
                palette.primary
            } else if status == button::Status::Hovered {
                Color { a: HOVER_ALPHA, ..palette.text }
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
        });

    let tip = container(tooltip_content).padding(TOOLTIP_PADDING).style(container::bordered_box);

    tooltip(image_button, tip, tooltip::Position::Right).into()
}
