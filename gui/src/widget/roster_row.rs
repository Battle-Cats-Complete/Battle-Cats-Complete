use iced::widget::image::Handle;
use iced::widget::{container, image as iced_image, row, tooltip};
use iced::{Element, Length};

use crate::common::row_window::ROW_HEIGHT;
use crate::widget::list_row;

const TOOLTIP_PADDING: f32 = 8.0;

pub(crate) fn roster_row<'a, Message: Clone + 'a>(
    handle: Handle,
    is_selected: bool,
    on_press: Message,
    tooltip_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let image = iced_image(handle).height(Length::Fixed(ROW_HEIGHT));
    let image_button = list_row(row![image].width(Length::Fill), is_selected, false, Length::Fill, on_press);

    let tip = container(tooltip_content).padding(TOOLTIP_PADDING).style(container::bordered_box);

    tooltip(image_button, tip, tooltip::Position::Right).into()
}
