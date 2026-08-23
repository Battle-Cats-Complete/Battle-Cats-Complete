use std::cell::RefCell;
use std::fmt;
use std::sync::LazyLock;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, scrollable, text, Column, Row};
use iced::{Element, Length, Padding, Theme};
use nyanko::combat::get_talent;
use rustc_hash::FxHashMap;

use kore::common::gfx::autocrop;
use kore::domains::settings::EditorValues;
use kore::systems::combat::registry::get_display_def;
use kore::Vfs;

use crate::app::theme;
use crate::widget::smooth_scroll;

use super::resolved::Rule;
use super::schema::{slot_of, TALENT_HEAD, TALENT_SLOTS, TALENT_STRIDE};
use super::{cards, Draft, Message};

const ABILITY: usize = 0;
const MAX_LEVEL: usize = 1;
const MIN_1: usize = 2;
const MAX_1: usize = 3;
const NAME_ID: usize = 12;
const LIMIT: usize = 13;

const TYPE_ID: usize = 1;

const TRAITS: [&str; 12] = [
    "Red", "Floating", "Dark", "Metal", "Angel", "Alien", "Zombie", "Relic", "Traitless", "Witch",
    "Eva", "Aku",
];

const HEADINGS: [(&str, f32); 6] = [
    ("Ability", 132.0),
    ("Image", 74.0),
    ("Ultra", 58.0),
    ("Total Levels", 84.0),
    ("Minimum Value", 92.0),
    ("Maximum Value", 92.0),
];

const ULTRA: i32 = 1;

const COLUMN_GAP: f32 = 6.0;
const ROW_GAP: f32 = 2.0;
const ROW_PADDING: f32 = 4.0;
#[cfg(test)]
const SCROLLBAR: f32 = 16.0;

const LABEL_SIZE: f32 = 12.0;
const NOTICE_SIZE: f32 = 11.0;
const PREVIEW_HEIGHT: f32 = 15.0;

const TRAIT_WIDTH: f32 = 74.0;
const TRAIT_SIZE: f32 = 11.0;
const TRAIT_INSET: f32 = 3.0;
const TILE_WIDTH: f32 = 92.0;
const CHOOSER_COLUMNS: usize = 6;

const SEARCH_PLACEHOLDER: &str = "Search fields";
const IDLE_TYPE_ID: &str = "No Talents utilize Type ID";
const TYPE_ID_TITLE: &str = "Type ID";

#[cfg(test)]
fn columns() -> f32 {
    let widths: f32 = HEADINGS.iter().map(|(_, width)| width).sum();
    let gaps = COLUMN_GAP * (HEADINGS.len() - 1) as f32;

    widths + gaps + cards::BODY_PADDING * 2.0 + SCROLLBAR
}

pub(super) fn view<'a>(
    draft: &'a Draft,
    width: f32,
    query: &'a str,
    armed: bool,
    names: &Names,
    vfs: &Vfs,
    picker: Option<usize>,
) -> Element<'a, Message> {
    if draft.values() == EditorValues::Resolved {
        return resolved(draft, armed, names, vfs, picker);
    }

    raw(draft, width, query, armed)
}

fn raw<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    let filtered = query.trim().to_lowercase();

    let shown: Vec<usize> = (0..draft.len())
        .filter(|index| {
            filtered.is_empty() || draft.schema().label(*index).to_lowercase().contains(&filtered)
        })
        .collect();

    let spare = spare(draft);

    cards::shell(
        Some(cards::search(query, width, SEARCH_PLACEHOLDER)),
        cards::grid(draft, width, &shown, dim_from(spare, &shown)),
        cards::footer(vec![cards::sync(armed)]),
    )
}

fn resolved<'a>(
    draft: &'a Draft,
    armed: bool,
    names: &Names,
    vfs: &Vfs,
    picker: Option<usize>,
) -> Element<'a, Message> {
    if let Some(index) = picker {
        return cards::shell(None, chooser(names, vfs, index), cards::footer(vec![cards::sync(armed)]));
    }

    let filled = filled(draft);
    let mut rows = Column::new().spacing(ROW_GAP);

    for slot in 0..TALENT_SLOTS.min(filled + 1) {
        rows = rows.push(striped(talent_row(draft, slot, names, vfs), slot));
    }

    let body = column![headings(), scrolled(rows)].height(Length::Fill);

    cards::shell(None, body.into(), cards::footer(vec![traits(draft), cards::sync(armed)]))
}

fn scrolled<'a>(rows: Column<'a, Message>) -> Element<'a, Message> {
    let inset = Padding::ZERO.left(cards::BODY_PADDING).right(cards::BODY_PADDING);

    let area =
        scrollable(container(rows).padding(inset)).width(Length::Fill).height(Length::Fill);

    smooth_scroll(area).into()
}

fn chooser<'a>(names: &Names, vfs: &Vfs, index: usize) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(ROW_GAP);
    let mut line = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
    let mut used = 0;

    for choice in names.choices(vfs) {
        if used == CHOOSER_COLUMNS {
            rows = rows.push(line);
            line = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
            used = 0;
        }

        line = line.push(names.tile(vfs, choice, index));
        used += 1;
    }

    column![
        container(
            button(theme::centered_text("Back").size(TRAIT_SIZE))
                .width(Length::Fixed(TRAIT_WIDTH))
                .padding(TRAIT_INSET)
                .style(theme::neutral_button)
                .on_press(Message::Picker(None)),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(ROW_PADDING),
        scrolled(rows.push(line)),
    ]
    .height(Length::Fill)
    .into()
}

fn striped<'a>(content: Element<'a, Message>, index: usize) -> Element<'a, Message> {
    container(content)
        .padding(ROW_PADDING)
        .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
        .into()
}

fn headings<'a>() -> Element<'a, Message> {
    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (label, width) in HEADINGS {
        line = line.push(
            container(text(label).size(LABEL_SIZE).align_x(Horizontal::Center).width(Length::Fill))
                .width(Length::Fixed(width)),
        );
    }

    container(line)
        .padding(Padding::from(ROW_PADDING).left(cards::BODY_PADDING).right(cards::BODY_PADDING))
        .style(theme::zebra_table_header)
        .into()
}

fn talent_row<'a>(
    draft: &'a Draft,
    slot: usize,
    names: &Names,
    vfs: &Vfs,
) -> Element<'a, Message> {
    let base = TALENT_HEAD + slot * TALENT_STRIDE;
    let ability = draft.reads_at(base + ABILITY).unwrap_or_default();
    let named = draft.reads_at(base + NAME_ID).unwrap_or(-1);

    let skills = cards::options(CATALOGUE.clone(), Skill::of(ability), move |pick: Skill| {
        Message::Picked(base + ABILITY, pick.id)
    });

    let picture = names.control(vfs, image_id(named, ability), named, base + NAME_ID);

    let cells = [
        skills,
        picture,
        ultra(draft.reads_at(base + LIMIT).unwrap_or_default(), base + LIMIT),
        cards::number(draft, base + MAX_LEVEL),
        cards::number(draft, base + MIN_1),
        cards::number(draft, base + MAX_1),
    ];

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (cell, (_, width)) in cells.into_iter().zip(HEADINGS) {
        line = line.push(container(cell).width(Length::Fixed(width)));
    }

    line.into()
}

fn ultra<'a>(current: i32, index: usize) -> Element<'a, Message> {
    let set = current == ULTRA;

    let style: theme::ButtonStyleFn =
        if set { theme::success_button } else { theme::neutral_button };

    let label = theme::centered_text(if set { "Yes" } else { "No" })
        .size(TRAIT_SIZE)
        .width(Length::Fill)
        .wrapping(text::Wrapping::None);

    button(label)
        .width(Length::Fill)
        .padding(TRAIT_INSET)
        .style(style)
        .on_press(Message::Picked(index, i32::from(!set)))
        .into()
}

fn image_id(named: i32, ability: i32) -> i32 {
    if named > 0 { named } else { ability }
}

fn traits<'a>(draft: &'a Draft) -> Element<'a, Message> {
    let mask = draft.reads_at(TYPE_ID).unwrap_or_default();
    let live = arming(draft);

    let mut stack = Column::new().spacing(ROW_GAP).align_x(Horizontal::Center);
    let mut buttons = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
    let mut used = 0;

    for (bit, name) in TRAITS.into_iter().enumerate() {
        if used == TRAITS.len() / 2 {
            stack = stack.push(buttons);
            buttons = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
            used = 0;
        }

        buttons = buttons.push(trait_button(bit, name, mask, live));
        used += 1;
    }

    let title = if live {
        text(TYPE_ID_TITLE).size(NOTICE_SIZE)
    } else {
        text(IDLE_TYPE_ID).size(NOTICE_SIZE).style(text::secondary)
    };

    column![title, stack.push(buttons)].spacing(ROW_GAP).align_x(Horizontal::Center).into()
}

fn trait_button<'a>(bit: usize, name: &'a str, mask: i32, live: bool) -> Element<'a, Message> {
    let set = mask & (1 << bit) != 0;

    let style: theme::ButtonStyleFn = match (live, set) {
        (false, _) => theme::inert_button,
        (true, true) => theme::success_button,
        (true, false) => theme::neutral_button,
    };

    let label =
        theme::centered_text(name).size(TRAIT_SIZE).width(Length::Fill).wrapping(text::Wrapping::None);

    let control = button(label).width(Length::Fixed(TRAIT_WIDTH)).padding(TRAIT_INSET).style(style);

    if live {
        return control.on_press(Message::Picked(TYPE_ID, mask ^ (1 << bit))).into();
    }

    control.into()
}

fn arming(draft: &Draft) -> bool {
    (0..filled(draft)).any(|slot| {
        draft.reads_at(TALENT_HEAD + slot * TALENT_STRIDE + NAME_ID).is_some_and(|id| id != -1)
    })
}

fn spare(draft: &Draft) -> Option<usize> {
    let used = filled(draft);

    (used < TALENT_SLOTS).then(|| TALENT_HEAD + used * TALENT_STRIDE)
}

fn dim_from(spare: Option<usize>, shown: &[usize]) -> Option<usize> {
    let first = spare?;

    shown.contains(&first).then_some(first)
}

fn filled(draft: &Draft) -> usize {
    (0..TALENT_SLOTS)
        .take_while(|slot| draft.reads_at(TALENT_HEAD + slot * TALENT_STRIDE) != Some(0))
        .count()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Skill {
    id: i32,
    name: &'static str,
}

impl Skill {
    fn of(id: i32) -> Skill {
        CATALOGUE
            .iter()
            .copied()
            .find(|skill| skill.id == id)
            .unwrap_or(Skill { id, name: "Unknown" })
    }
}

impl fmt::Display for Skill {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name)
    }
}

static CATALOGUE: LazyLock<Vec<Skill>> = LazyLock::new(|| {
    let mut listed = vec![Skill { id: 0, name: "None" }];

    listed.extend((1..=u8::MAX).filter_map(|id| {
        let ability = get_talent(id)?;

        Some(Skill { id: i32::from(id), name: get_display_def(ability.identity).name })
    }));

    listed
});

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Named(i32);

impl fmt::Display for Named {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            return formatter.write_str("Default");
        }

        write!(formatter, "{:03}", self.0)
    }
}

#[derive(Default)]
pub(super) struct Names {
    listed: RefCell<Option<Vec<Named>>>,
    images: RefCell<FxHashMap<i32, Option<Handle>>>,
}

impl Names {
    fn choices(&self, vfs: &Vfs) -> Vec<Named> {
        if let Some(cached) = self.listed.borrow().as_ref() {
            return cached.clone();
        }

        let mut listed = vec![Named(-1)];
        listed.extend(available(vfs).into_iter().map(Named));

        *self.listed.borrow_mut() = Some(listed.clone());

        listed
    }

    fn control<'a>(&self, vfs: &Vfs, drawn: i32, current: i32, index: usize) -> Element<'a, Message> {
        button(self.face(vfs, drawn, Named(current)))
            .width(Length::Fill)
            .padding(TRAIT_INSET)
            .style(theme::neutral_button)
            .on_press(Message::Picker(Some(index)))
            .into()
    }

    fn tile<'a>(&self, vfs: &Vfs, choice: Named, index: usize) -> Element<'a, Message> {
        button(self.face(vfs, choice.0, choice))
            .width(Length::Fixed(TILE_WIDTH))
            .padding(TRAIT_INSET)
            .style(theme::neutral_button)
            .on_press(Message::Picked(index, choice.0))
            .into()
    }

    fn face<'a>(&self, vfs: &Vfs, drawn: i32, fallback: Named) -> Element<'a, Message> {
        let Some(handle) = self.handle(vfs, drawn) else {
            return theme::centered_text(fallback.to_string())
                .size(TRAIT_SIZE)
                .width(Length::Fill)
                .wrapping(text::Wrapping::None)
                .into();
        };

        container(iced_image(handle).height(Length::Fixed(PREVIEW_HEIGHT)))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }

    fn handle(&self, vfs: &Vfs, id: i32) -> Option<Handle> {
        if let Some(cached) = self.images.borrow().get(&id) {
            return cached.clone();
        }

        let loaded = load(vfs, id);
        self.images.borrow_mut().insert(id, loaded.clone());

        loaded
    }
}

fn load(vfs: &Vfs, id: i32) -> Option<Handle> {
    if id <= 0 {
        return None;
    }

    let path = vfs.find(&format!("Skill_name_{id:03}.png"))?;
    let decoded = image::open(&path).ok()?;
    let cropped = autocrop(decoded.to_rgba8());

    Some(Handle::from_rgba(cropped.width(), cropped.height(), cropped.into_raw()))
}

fn available(vfs: &Vfs) -> Vec<i32> {
    let mut found: Vec<i32> = vfs
        .glob("Skill_name_")
        .into_iter()
        .filter_map(|name| {
            let digits: String =
                name.strip_prefix("Skill_name_")?.chars().take_while(char::is_ascii_digit).collect();

            (digits.len() == 3).then(|| digits.parse().ok())?
        })
        .collect();

    found.sort_unstable();
    found.dedup();

    found
}

pub(super) fn rule(index: usize) -> Rule {
    match slot_of(index).map(|_| (index - TALENT_HEAD) % TALENT_STRIDE) {
        Some(NAME_ID) => Rule::Floor(-1),
        Some(_) | None => Rule::Plain,
    }
}

#[cfg(test)]
mod tests {
    use crate::editor::figures::schema::{self, Subject};
    use crate::editor::figures::TALENT_SIZE;

    #[test]
    fn talents_publish_no_columns() {
        assert!(
            schema::of(Subject::Talents).order().is_empty(),
            "talents: nyanko now publishes named columns, so the generated labels must become a table"
        );
    }

    #[test]
    fn every_talent_ability_resolves_to_a_name() {
        let listed = &*super::CATALOGUE;

        assert!(listed.len() > 1, "nyanko publishes no talent abilities, so the picker would be empty");

        for skill in &listed[1..] {
            assert!(!skill.name.is_empty(), "talent ability {} resolves to an empty name", skill.id);
        }
    }

    #[test]
    fn the_popup_minimum_fits_every_column() {
        assert!(
            super::columns() <= TALENT_SIZE.width,
            "the talents popup minimum is narrower than its own columns, so the table would clip"
        );
    }
}
