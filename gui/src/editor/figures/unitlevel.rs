use iced::alignment::Horizontal;
use iced::widget::{container, text};
use iced::{Element, Length};

use super::{cards, Draft, Message};
use super::schema::BRACKET;

const NOTICE: &str =
    "Each bracket is the percentage of the base stat added per level within it, and the last one carries past its own range";

const NOTICE_SIZE: f32 = 11.0;

pub(super) fn view<'a>(draft: &'a Draft, width: f32, armed: bool, cap: Option<i32>) -> Element<'a, Message> {
    let shown: Vec<usize> = (0..draft.len()).collect();
    let reachable = cap.filter(|level| *level > 0).map(|level| (level as usize - 1) / BRACKET + 1);

    let notice = match reachable.filter(|first| *first < draft.len()) {
        Some(_) => match cap {
            Some(level) => format!("{NOTICE}\nDimmed brackets sit above this unit's level cap of {level}, set in \"unitbuy.csv\""),
            None => NOTICE.to_owned(),
        },
        None => NOTICE.to_owned(),
    };

    let banner = container(
        text(notice).size(NOTICE_SIZE).align_x(Horizontal::Center).style(text::secondary),
    )
    .width(Length::Fill)
    .center_x(Length::Fill);

    cards::shell(
        Some(cards::header(banner)),
        cards::grid(draft, width, &shown, reachable),
        cards::footer(vec![cards::sync(armed)]),
    )
}
