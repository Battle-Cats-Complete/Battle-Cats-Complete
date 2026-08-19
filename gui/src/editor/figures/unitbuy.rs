use iced::Element;

use super::{cards, Draft, Message};

pub(super) fn view<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    let schema = draft.schema();
    let needle = query.trim().to_lowercase();

    let shown: Vec<usize> = (0..draft.len())
        .filter(|index| needle.is_empty() || schema.label(*index).to_lowercase().contains(&needle))
        .collect();

    cards::shell(
        Some(cards::search(query, width, "Search Field...")),
        cards::grid(draft, width, &shown, None),
        cards::footer(vec![cards::sync(armed)]),
    )
}
