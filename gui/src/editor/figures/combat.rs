pub(super) mod cats;
pub(super) mod enemies;

use iced::Element;

use super::{cards, Draft, Message};

pub(super) fn view<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    let schema = draft.schema();
    let needle = query.trim().to_lowercase();

    let shown: Vec<usize> = (0..draft.len())
        .filter(|index| needle.is_empty() || schema.label(*index).to_lowercase().contains(&needle))
        .collect();

    let mut footer = Vec::with_capacity(2);

    if schema.comments() {
        footer.push(cards::comment(draft.comment()));
    }

    footer.push(cards::sync(armed));

    cards::shell(
        Some(cards::search(query, width, "Search Attribute...")),
        cards::grid(draft, width, &shown, None),
        cards::footer(footer),
    )
}
