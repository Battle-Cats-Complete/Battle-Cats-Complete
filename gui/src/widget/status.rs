use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length};

const TITLE_SIZE: f32 = 18.0;
const DETAIL_SIZE: f32 = 12.0;
const DETAIL_GAP: f32 = 6.0;

pub(crate) fn status<'a, M: 'a>(title: impl text::IntoFragment<'a>, detail: Option<String>) -> Element<'a, M> {
    let mut body = column![text(title).size(TITLE_SIZE)]
        .spacing(DETAIL_GAP)
        .align_x(Alignment::Center);

    if let Some(detail) = detail {
        body = body.push(text(detail).size(DETAIL_SIZE).style(text::secondary));
    }

    container(body)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
