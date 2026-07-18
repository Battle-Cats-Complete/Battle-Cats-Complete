use iced::{Element, Length};
use iced::widget::{container, text};

pub fn name_box<'a, Message: 'a>(name_text: &str) -> Element<'a, Message> {
    container(text(name_text.to_string()).size(14))
        .width(Length::Fixed(150.0))
        .height(Length::Fixed(15.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}