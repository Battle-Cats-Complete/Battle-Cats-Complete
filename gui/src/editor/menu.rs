use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::alignment;
use iced::widget::{button, container, text as text_widget, tooltip, Column};
use iced::{Border, Element, Length, Padding, Pixels, Size, Theme};

use crate::app::theme;
use crate::common::feedback::CONFIRM_LABEL;

use super::{Item, Message};

const TEXT_SIZE: f32 = 13.0;
const FRAME_PADDING: f32 = 4.0;
const ITEM_PADDING_X: f32 = 10.0;
const ITEM_PADDING_Y: f32 = 5.0;
const BORDER_WIDTH: f32 = 1.0;
const SHAPING: text::Shaping = text::Shaping::Advanced;
const WRAPPING: text::Wrapping = text::Wrapping::None;
const SAFETY: f32 = 1.0;
const TOOLTIP_PADDING: f32 = 8.0;
const TOOLTIP_GAP: f32 = FRAME_PADDING + BORDER_WIDTH + 4.0;

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

pub(super) fn measure(renderer: &iced::Renderer, items: &[Item]) -> Size {
    let frame = FRAME_PADDING * 2.0 + BORDER_WIDTH * 2.0;

    let (width, height) = items.iter().fold((0.0_f32, 0.0_f32), |(width, height), item| {
        let bounds = label_bounds(renderer, &item.label);
        let armed = if item.confirm { label_bounds(renderer, CONFIRM_LABEL).width } else { 0.0 };

        (width.max(bounds.width).max(armed), height + bounds.height + ITEM_PADDING_Y * 2.0)
    });

    Size::new(width + ITEM_PADDING_X * 2.0 + frame + SAFETY, height + frame)
}

fn label_bounds(renderer: &iced::Renderer, label: &str) -> Size {
    Paragraph::with_text(text::Text {
        content: label,
        bounds: Size::INFINITE,
        size: Pixels(TEXT_SIZE),
        line_height: text::LineHeight::default(),
        font: renderer.default_font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: SHAPING,
        wrapping: WRAPPING,
    })
    .min_bounds()
}

pub(super) fn view<'a, M: Clone + 'a>(
    items: &'a [Item],
    armed: Option<usize>,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let mut column = Column::with_capacity(items.len()).width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        column = column.push(entry(index, item, armed == Some(index), to_message));
    }

    container(column).width(Length::Fill).padding(FRAME_PADDING).style(frame).into()
}

fn entry<'a, M: Clone + 'a>(
    index: usize,
    item: &'a Item,
    armed: bool,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let label = text_widget(if armed { CONFIRM_LABEL } else { item.label.as_str() })
        .size(TEXT_SIZE)
        .shaping(SHAPING)
        .wrapping(WRAPPING);

    let mut entry = button(label)
        .width(Length::Fill)
        .padding(Padding::new(ITEM_PADDING_Y).left(ITEM_PADDING_X).right(ITEM_PADDING_X))
        .style(if armed { armed_style } else { entry_style });

    if item.action.is_some() {
        entry = entry.on_press(to_message(Message::Invoked(index)));
    }

    let Some(hint) = item.hint.as_deref() else {
        return entry.into();
    };

    let bubble = container(text_widget(hint).size(TEXT_SIZE).shaping(SHAPING))
        .padding(TOOLTIP_PADDING)
        .style(container::bordered_box);

    tooltip(entry, bubble, tooltip::Position::Top).gap(TOOLTIP_GAP).into()
}

fn frame(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border {
            color: palette.background.strong.color,
            width: BORDER_WIDTH,
            radius: theme::RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

fn armed_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.danger.strong.color,
        button::Status::Active | button::Status::Disabled => palette.danger.base.color,
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.danger.base.text,
        border: Border::default().rounded(theme::RADIUS_SM),
        ..button::Style::default()
    }
}

fn entry_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (Some(palette.primary.base.color.into()), palette.primary.base.text)
        }
        button::Status::Active => (None, palette.background.base.text),
        button::Status::Disabled => (None, theme::weak_text_color(theme)),
    };

    button::Style {
        background,
        text_color,
        border: Border::default().rounded(theme::RADIUS_SM),
        ..button::Style::default()
    }
}
