use iced::alignment::Horizontal;
use iced::widget::{container, text};
use iced::{Element, Length};

use super::{cards, Draft, Message};

const NOTICE: &str =
    "Each bracket is the percentage of the base stat added per level within it, and the last one carries past its own range";

const NOTICE_SIZE: f32 = 11.0;

pub(super) fn view<'a>(draft: &'a Draft, width: f32, armed: bool) -> Element<'a, Message> {
    let shown: Vec<usize> = (0..draft.len()).collect();

    let notice = container(
        text(NOTICE).size(NOTICE_SIZE).align_x(Horizontal::Center).style(text::secondary),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    cards::shell(
        Some(cards::header(notice)),
        cards::grid(draft, width, &shown),
        cards::footer(vec![cards::sync(armed)]),
    )
}
