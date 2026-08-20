use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, scrollable, text, text_input, Column, Row};
use iced::{Element, Length, Padding, Theme};

use crate::app::theme;
use crate::common::feedback::CONFIRM_LABEL;
use crate::widget::{hover_hint, smooth_scroll};

use super::{Draft, Message, LABEL_HEIGHTS};

pub(super) const CARD_WIDTH: f32 = 86.0;
pub(super) const CARD_GAP: f32 = 8.0;
pub(super) const BODY_PADDING: f32 = 12.0;

const LABEL_SIZE: f32 = 12.0;
const INPUT_SIZE: f32 = 13.0;
const CARD_PADDING: f32 = 6.0;
const SYNC_WIDTH: f32 = 172.0;
const FIELD_PADDING: f32 = 4.0;
const SEARCH_FRACTION: f32 = 2.0 / 3.0;

const OPAQUE_HINT: &str =
    "This value is not resolved and represents a raw data value\nEdit this attribute at your own risk";

pub(super) fn usable(width: f32) -> f32 {
    (width - BODY_PADDING * 2.0).max(CARD_WIDTH)
}

pub(super) fn grid<'a>(
    draft: &'a Draft,
    width: f32,
    shown: &[usize],
    dim_from: Option<usize>,
) -> Element<'a, Message> {
    let per_row = (((usable(width) + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize).max(1);

    let mut rows = Column::new().spacing(CARD_GAP);
    let mut line = Row::new().spacing(CARD_GAP);

    for (slot, index) in shown.iter().enumerate() {
        if slot > 0 && slot % per_row == 0 {
            rows = rows.push(line);
            line = Row::new().spacing(CARD_GAP);
        }

        line = line.push(card(draft, *index, dim_from.is_some_and(|first| *index >= first)));
    }

    let centered = container(rows.push(line))
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(Padding::ZERO.left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING));

    smooth_scroll(scrollable(centered).width(Length::Fill).height(Length::Fill)).into()
}

fn card(draft: &Draft, index: usize, dimmed: bool) -> Element<'_, Message> {
    let schema = draft.schema();
    let hint = schema.to_display(index, schema.fallback(index), draft.values()).to_string();

    let field = text_input(&hint, draft.input(index))
        .on_input(move |entry| Message::Changed(index, entry))
        .size(INPUT_SIZE)
        .padding(2)
        .align_x(Horizontal::Center)
        .style(theme::rounded_input);

    let opaque = draft.opaque(index);

    let caption = text(schema.label(index))
        .size(LABEL_SIZE)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .style(move |theme: &Theme| text::Style {
            color: dimmed.then(|| theme::weak_text_color(theme)),
        });

    let caption = if opaque { hover_hint(caption, OPAQUE_HINT) } else { caption.into() };

    let label = container(caption)
        .height(Length::Fixed(LABEL_HEIGHTS[schema.subject().slot()]))
        .align_y(Vertical::Center);

    let style = if opaque {
        theme::card_container_danger
    } else if dimmed {
        theme::card_container_muted
    } else {
        theme::card_container
    };

    container(column![label, field].spacing(2))
        .width(Length::Fixed(CARD_WIDTH))
        .padding(CARD_PADDING)
        .style(style)
        .into()
}

pub(super) fn search<'a>(query: &str, width: f32, placeholder: &'a str) -> Element<'a, Message> {
    header(search_field(query, width, placeholder))
}

fn search_field<'a>(query: &str, width: f32, placeholder: &'a str) -> Element<'a, Message> {
    let field = text_input(placeholder, query)
        .on_input(Message::SearchChanged)
        .size(INPUT_SIZE)
        .padding(FIELD_PADDING)
        .width(Length::Fixed((usable(width) * SEARCH_FRACTION).max(CARD_WIDTH)))
        .style(theme::rounded_input);

    container(field).width(Length::Fill).center_x(Length::Fill).into()
}

pub(super) fn header<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content.into())
        .padding(Padding::ZERO.top(BODY_PADDING).left(BODY_PADDING).right(BODY_PADDING).bottom(CARD_GAP))
        .into()
}

pub(super) fn comment<'a>(value: &str) -> Element<'a, Message> {
    text_input("Comment...", value)
        .on_input(Message::CommentChanged)
        .size(INPUT_SIZE)
        .padding(FIELD_PADDING)
        .width(Length::Fill)
        .style(theme::rounded_input)
        .into()
}

pub(super) fn footer<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut stack = Column::with_capacity(rows.len()).spacing(CARD_GAP).align_x(Horizontal::Center);

    for row in rows {
        stack = stack.push(row);
    }

    container(stack)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(Padding::ZERO.top(CARD_GAP).left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING))
        .into()
}

pub(super) fn sync<'a>(armed: bool) -> Element<'a, Message> {
    let label = if armed { CONFIRM_LABEL } else { "Sync With \"game\"" };

    let content = theme::centered_text(label).size(INPUT_SIZE).wrapping(text::Wrapping::None);

    button(content)
        .width(Length::Fixed(SYNC_WIDTH))
        .padding([FIELD_PADDING + 1.0, 10.0])
        .style(theme::danger_button)
        .on_press(Message::Sync)
        .into()
}

pub(super) fn shell<'a>(
    top: Option<Element<'a, Message>>,
    body: Element<'a, Message>,
    bottom: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut stack = Column::new().height(Length::Fill);

    if let Some(top) = top {
        stack = stack.push(top);
    }

    stack.push(body).push(bottom).into()
}
