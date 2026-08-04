use iced::{border, Element, Length, Theme};
use iced::widget::{container, text};

pub const ICON_SIZE: f32 = 40.0;

pub fn fallback_icon<'a, Message: 'a>(icon_text: &str) -> Element<'a, Message> {
    container(text(icon_text.to_string()).size(10))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.palette();
            container::Style {
                border: border::rounded(0).color(palette.danger).width(1.5),
                ..Default::default()
            }
        })
        .into()
}
