use iced::alignment::Vertical;
use iced::{border, Element, Length, Theme};
use iced::widget::{container, row, text};

pub fn grid_cell<'a, Message: 'a>(text_content: &str, is_header: bool) -> Element<'a, Message> {
    container(text(text_content.to_string()))
        .padding(1.5)
        .width(Length::Fixed(60.0))
        .center_x(Length::Fill)
        .style(move |theme: &Theme| {
            let palette = theme.palette();
            container::Style {
                background: Some(if is_header { palette.background.into() } else { palette.primary.into() }),
                border: border::rounded(4).color(palette.text).width(1),
                ..Default::default()
            }
        })
        .into()
}

pub fn render_frames<'a, Message: 'a>(frames: i32, max_width: f32) -> Element<'a, Message> {
    let seconds = frames as f32 / 30.0;

    let base_text = format!("{:.2}s", seconds);
    let frame_text = format!(" {}f", frames);

    container(
        row![
            text(base_text),
            text(frame_text).size(10)
        ]
            .align_y(Vertical::Bottom)
    )
        .width(Length::Fixed(max_width))
        .into()
}