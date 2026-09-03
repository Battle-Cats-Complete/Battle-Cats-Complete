use super::*;
use iced::widget::{button, column, container, row, rule, text, Column, Row, Space, Text};

pub(super) fn ignored(line: &str) -> bool {
    line.split_once(": ").is_some_and(|(label, _)| IGNORED_LABELS.contains(&label))
}

pub(super) fn header_text<'a>(content: impl ToString) -> Text<'a> {
    strong(content, HEADER_TEXT_SIZE).color(CHIP_TEXT)
}

pub(super) fn strong<'a>(content: impl ToString, size: f32) -> Text<'a> {
    theme::bold_text(content).size(size).wrapping(Wrapping::None)
}

pub(super) fn plain<'a>(content: impl ToString, size: f32) -> Text<'a> {
    text(content.to_string()).size(size).wrapping(Wrapping::None)
}

pub(super) fn relabel(line: &str) -> Cow<'_, str> {
    let Some((label, reading)) = line.split_once(": ") else {
        return Cow::Borrowed(line);
    };

    let (from, to) = DEPLOY_COOLDOWN;

    if label != from {
        return Cow::Borrowed(line);
    }

    Cow::Owned(format!("{}: {}", to, reading))
}

pub(super) fn value_row<'a>(line: &str) -> Element<'a, Message> {
    let content: Element<'a, Message> = match line.split_once(": ") {
        Some((label, reading)) => row![
            plain(label, VALUE_TEXT_SIZE).color(CHIP_TEXT),
            strong(reading, VALUE_TEXT_SIZE).color(CHIP_TEXT),
        ]
        .spacing(VALUE_LABEL_GAP)
        .align_y(Vertical::Center)
        .into(),
        None => strong(line, VALUE_TEXT_SIZE).color(CHIP_TEXT).into(),
    };

    dark_box(content, Length::Shrink)
}

pub(super) fn dark_box<'a>(content: impl Into<Element<'a, Message>>, height: Length) -> Element<'a, Message> {
    container(content)
        .padding(BOX_PADDING)
        .height(height)
        .align_y(Vertical::Center)
        .style(|_theme: &Theme| container::Style {
            background: Some(DARK_BOX_BG.into()),
            border: iced::border::rounded(BOX_RADIUS),
            ..Default::default()
        })
        .into()
}

pub(super) fn light_box<'a>(content: impl Into<Element<'a, Message>>, height: Length) -> Element<'a, Message> {
    container(content)
        .padding(BOX_PADDING)
        .height(height)
        .align_y(Vertical::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            border: iced::border::rounded(BOX_RADIUS),
            ..Default::default()
        })
        .into()
}

pub(super) fn pictured(name: &str) -> bool {
    name.rsplit('.').next().is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

pub(super) fn file_name<'a>(name: &str) -> Text<'a> {
    theme::bold_text(name).size(FILE_NAME_SIZE).wrapping(Wrapping::Glyph)
}

pub(super) fn name_cell<'a>(name: &str, cell: f32) -> Element<'a, Message> {
    open_file(light_box(file_name(name).width(Length::Fill), Length::Fixed(cell)), name)
}

pub(super) fn widest(widths: impl Iterator<Item = f32>) -> f32 {
    widths.fold(0.0, f32::max)
}

pub(super) fn band<T>(items: &[T], row: usize, columns: usize) -> &[T] {
    let start = (row * columns).min(items.len());

    &items[start..(start + columns).min(items.len())]
}

pub(super) fn banded<'a>(cells: Vec<Element<'a, Message>>, columns: usize) -> Element<'a, Message> {
    let filled = cells.len();
    let mut band = Row::with_capacity(columns).spacing(CARD_SPACING).width(Length::Fill);

    for cell in cells {
        band = band.push(container(cell).width(Length::FillPortion(1)));
    }

    for _ in filled..columns {
        band = band.push(Space::new().width(Length::FillPortion(1)));
    }

    band.into()
}

pub(super) fn open_file<'a>(content: impl Into<Element<'a, Message>>, name: &str) -> Element<'a, Message> {
    button(content.into())
        .padding(0)
        .width(Length::Fill)
        .style(button::text)
        .on_press(Message::OpenFile(name.to_string()))
        .into()
}

pub(super) fn tongues(languages: &[String]) -> String {
    languages
        .iter()
        .map(|code| if code.is_empty() { "BASE".to_string() } else { code.to_uppercase() })
        .collect::<Vec<String>>()
        .join(", ")
}

pub(super) fn notice<'a>(message: &'a str) -> Element<'a, Message> {
    plain(message, NOTICE_TEXT_SIZE)
        .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) })
        .into()
}

pub(super) fn cards_per_row(available_width: f32, min_width: f32) -> usize {
    let usable = (available_width - PAGE_PADDING * 2.0 - SCROLLBAR_RESERVE).max(min_width);
    let slot = min_width + CARD_SPACING;

    (((usable + CARD_SPACING) / slot).floor() as usize).max(1)
}

pub(super) fn ledger_rule<'a>() -> Element<'a, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color { a: LEDGER_RULE_ALPHA, ..CHIP_TEXT },
            ..rule::default(theme)
        })
        .into()
}

pub(super) fn labelled<'a>(title: &'static str, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![strong(title, HEADER_TEXT_SIZE).color(CHIP_TEXT), body.into()].spacing(4).width(Length::Fill).into()
}

pub(super) fn hinted<'a>(content: impl Into<Element<'a, Message>>, description: &str) -> Element<'a, Message> {
    if description.is_empty() {
        return content.into();
    }

    tooltip(
        content,
        container(tinted_superscript(description, VALUE_TEXT_SIZE, None))
            .padding(6)
            .style(container::bordered_box),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn wrapped<'a>(cards: Vec<Element<'a, Message>>, room: f32, card: f32, gap: f32) -> Element<'a, Message> {
    let per_row = fits(room, card, gap);

    let mut body = Column::new().spacing(gap).width(Length::Fill).align_x(Horizontal::Center);
    let mut held = cards.into_iter().peekable();

    while held.peek().is_some() {
        body = body.push(Row::with_children(held.by_ref().take(per_row)).spacing(gap));
    }

    body.into()
}

pub(super) fn fits(room: f32, card: f32, gap: f32) -> usize {
    (((room + gap) / (card + gap)).floor() as usize).max(1)
}

pub(super) fn packed<'a>(cards: Vec<Element<'a, Message>>, room: f32, card: f32, gap: f32) -> Element<'a, Message> {
    let lanes = fits(room, card, gap).min(cards.len().max(1));

    let mut buckets: Vec<Vec<Element<'a, Message>>> = (0..lanes).map(|_| Vec::new()).collect();

    for (index, held) in cards.into_iter().enumerate() {
        buckets[index % lanes].push(held);
    }

    let stacks = buckets
        .into_iter()
        .map(|held| Column::with_children(held).spacing(gap).align_x(Horizontal::Center).into());

    Row::with_children(stacks).spacing(gap).into()
}

pub(super) fn placard<'a>(title: &'a str, content: impl Into<Element<'a, Message>>, room: f32) -> Element<'a, Message> {
    column![
        strong(title, SECTION_TITLE_SIZE),
        rule::horizontal(1),
        content.into(),
    ]
    .spacing(SECTION_HEAD_GAP)
    .align_x(Horizontal::Center)
    .width(Length::Fixed(room))
    .into()
}

fn table_header<'a>(title: &'static str, width: f32) -> Element<'a, Message> {
    container(theme::table_cell_text(title, Length::Fixed(width)).size(META_TEXT_SIZE))
        .center_y(Length::Fixed(TABLE_ROW_HEIGHT))
        .style(theme::zebra_table_header)
        .into()
}

pub(super) fn stamp_table<'a>() -> Element<'a, Message> {
    let rows = [("Import", since(import::pack_stamp())), ("Snapshot", since(mining::snapped_at()))];

    let mut table = Column::new().push(table_header("Activity", TALLY_TABLE_WIDTH)).width(Length::Shrink);

    for (index, (label, when)) in rows.into_iter().enumerate() {
        table = table.push(row![
            tally_label(label, STAMP_LABEL_WIDTH, index),
            sized_cell(when, STAMP_VALUE_WIDTH, index),
        ]);
    }

    table.into()
}

pub(super) fn since(stamp: Option<SystemTime>) -> String {
    stamp
        .and_then(|held| SystemTime::now().duration_since(held).ok())
        .map_or_else(|| "N/A".to_string(), ago)
}

pub(super) fn tally_table<'a>(title: &'static str, rows: Vec<(&'static str, usize)>) -> Element<'a, Message> {
    let mut table =
        Column::new().push(table_header(title, TALLY_TABLE_WIDTH)).align_x(Horizontal::Center).width(Length::Shrink);

    for (index, (label, count)) in rows.into_iter().enumerate() {
        table = table.push(row![tally_label(label, TALLY_LABEL_WIDTH, index), sized_cell(count, TALLY_VALUE_WIDTH, index)]);
    }

    table.into()
}

pub(super) fn zebra_cell<'a>(body: impl ToString, index: usize) -> Element<'a, Message> {
    sized_cell(body, TABLE_CELL_WIDTH, index)
}

fn sized_cell<'a>(body: impl ToString, width: f32, index: usize) -> Element<'a, Message> {
    container(theme::table_cell_text(body.to_string(), Length::Fixed(width)).size(META_TEXT_SIZE))
        .center_y(Length::Fixed(TABLE_ROW_HEIGHT))
        .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
        .into()
}

fn tally_label<'a>(body: &'a str, width: f32, index: usize) -> Element<'a, Message> {
    container(plain(body, META_TEXT_SIZE).width(Length::Fixed(width)))
        .padding([0, TALLY_PADDING as u16])
        .center_y(Length::Fixed(TABLE_ROW_HEIGHT))
        .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Surge and friends report a spawn width that reads as noise on a diff card.
    #[test]
    fn the_spawn_width_line_is_dropped_but_its_neighbours_survive() {
        assert!(ignored("Width: 400"));
        assert!(!ignored("Range: 400~800"));
        assert!(!ignored("Chance: 0% (+30%) -> 30%"));
        assert!(!ignored("Widthless"));
    }

    #[test]
    fn a_narrow_window_still_fits_one_card_per_row() {
        assert_eq!(cards_per_row(200.0, UNIT_MIN_WIDTH), 1);
        assert_eq!(cards_per_row(0.0, UNIT_MIN_WIDTH), 1);
        assert!(cards_per_row(1600.0, UNIT_MIN_WIDTH) > 1, "a wide window must fit more than one unit card");
    }
}
