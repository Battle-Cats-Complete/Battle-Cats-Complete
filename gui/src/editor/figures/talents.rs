mod values;

use std::cell::RefCell;
use std::fmt;
use std::sync::LazyLock;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, scrollable, text, tooltip, Column, Row};
use iced::{Element, Length, Theme};
use nyanko::combat::get_talent;
use rustc_hash::FxHashMap;

use kore::common::gfx::autocrop;
use kore::domains::settings::EditorValues;
use kore::systems::combat::registry::get_display_def;
use kore::{Vault, Vfs};

use crate::app::theme;
use crate::widget::smooth_scroll;

use super::resolved::Rule;
use super::schema::{slot_of, TALENT_HEAD, TALENT_SLOTS, TALENT_STRIDE};
use super::{cards, Draft, Frame, Message, GLYPH_RATIO};

const ABILITY: usize = 0;
const MAX_LEVEL: usize = 1;
const VALUES: usize = 4;
const MIN_1: usize = 2;
const TEXT_ID: usize = 10;
const COST_ID: usize = 11;
const NAME_ID: usize = 12;
const LIMIT: usize = 13;

const TYPE_ID: usize = 1;

const TRAITS: [&str; 12] = [
    "Red", "Floating", "Dark", "Metal", "Angel", "Alien", "Zombie", "Relic", "Traitless", "Witch",
    "Eva", "Aku",
];

struct Heading {
    label: &'static str,
    content: f32,
}

const HEADINGS: [Heading; 6 + VALUES * 2] = [
    Heading { label: "Ability", content: 100.0 },
    Heading { label: "Image", content: 52.0 },
    Heading { label: "Text ID", content: 48.0 },
    Heading { label: "Cost", content: 48.0 },
    Heading { label: "Ultra", content: 48.0 },
    Heading { label: "Total Levels", content: 48.0 },
    Heading { label: "Min Val 1", content: 40.0 },
    Heading { label: "Max Val 1", content: 40.0 },
    Heading { label: "Min Val 2", content: 40.0 },
    Heading { label: "Max Val 2", content: 40.0 },
    Heading { label: "Min Val 3", content: 40.0 },
    Heading { label: "Max Val 3", content: 40.0 },
    Heading { label: "Min Val 4", content: 40.0 },
    Heading { label: "Max Val 4", content: 40.0 },
];

const ULTRA: i32 = 1;

const HEADER_LINES: usize = 3;

static WIDTHS: LazyLock<[f32; HEADINGS.len()]> = LazyLock::new(|| {
    HEADINGS.map(|heading| heading.content.max(wrapped(heading.label)))
});

static TABLE_WIDTH: LazyLock<f32> = LazyLock::new(|| {
    WIDTHS.iter().sum::<f32>() + COLUMN_GAP * (HEADINGS.len() - 1) as f32
});

fn wrapped(label: &str) -> f32 {
    let words: Vec<usize> = label.split_whitespace().map(|word| word.chars().count()).collect();

    let widest = (1..=words.len().min(HEADER_LINES))
        .map(|lines| split(&words, lines))
        .min()
        .unwrap_or_default();

    widest as f32 * LABEL_SIZE * GLYPH_RATIO
}

fn split(words: &[usize], lines: usize) -> usize {
    let per_line = words.len().div_ceil(lines);

    words
        .chunks(per_line)
        .map(|chunk| chunk.iter().sum::<usize>() + chunk.len().saturating_sub(1))
        .max()
        .unwrap_or_default()
}

const COLUMN_GAP: f32 = 6.0;
const ROW_GAP: f32 = 2.0;
const ROW_PADDING: f32 = 4.0;
#[cfg(test)]
const FRAME_BORDER: f32 = 6.0;

const LABEL_SIZE: f32 = 12.0;
const NOTICE_SIZE: f32 = 11.0;
const PREVIEW_HEIGHT: f32 = 15.0;

const TRAIT_WIDTH: f32 = 74.0;
const TRAIT_SIZE: f32 = 11.0;
const TRAIT_INSET: f32 = 3.0;
const HINT_PADDING: f32 = 6.0;
const TILE_WIDTH: f32 = 92.0;
const CHOOSER_COLUMNS: usize = 7;

const SEARCH_PLACEHOLDER: &str = "Search fields";
const IDLE_TYPE_ID: &str = "No Talents utilize Type ID";
const NO_IMAGE: &str = "None";
const IMAGE_HUNT: &str = "Search ID...";
const WORDING_HUNT: &str = "Search ID or Content...";
const TYPE_ID_TITLE: &str = "Type ID";

#[cfg(test)]
fn columns() -> f32 {
    *TABLE_WIDTH + FRAME_BORDER
}

pub(super) fn view<'a>(draft: &'a Draft, frame: Frame<'a>) -> Element<'a, Message> {
    let Frame { width, query, armed, names, vault, picker, hunt, .. } = frame;

    if draft.values() == EditorValues::Resolved {
        return resolved(draft, width, armed, names, vault, picker, hunt);
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
    width: f32,
    armed: bool,
    names: &Names,
    vault: &Vault,
    picker: Option<usize>,
    hunt: &str,
) -> Element<'a, Message> {
    let vfs = &vault.vfs;

    if let Some(index) = picker {
        let (listing, placeholder) = match offset(index) {
            TEXT_ID => (wording(vault, index, hunt), WORDING_HUNT),
            _ => (chooser(names, vfs, index, hunt), IMAGE_HUNT),
        };

        let top = column![back(), cards::hunt(hunt, width, placeholder)];

        return cards::shell(Some(top.into()), listing, cards::footer(vec![cards::sync(armed)]));
    }

    let filled = filled(draft);
    let mut rows = Column::new().spacing(ROW_GAP);

    for slot in 0..TALENT_SLOTS.min(filled + 1) {
        rows = rows.push(striped(talent_row(draft, slot, names, vfs), slot));
    }

    let body = column![headings(), rows].height(Length::Fill);

    cards::shell(None, body.into(), cards::footer(vec![traits(draft), cards::sync(armed)]))
}

fn scrolled<'a>(rows: Column<'a, Message>) -> Element<'a, Message> {
    let area = scrollable(rows).width(Length::Fill).height(Length::Fill);

    smooth_scroll(area).into()
}

fn chooser<'a>(names: &Names, vfs: &Vfs, index: usize, hunt: &str) -> Element<'a, Message> {
    let wanted = hunt.trim();

    let mut rows = Column::new().spacing(ROW_GAP);
    let mut line = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
    let mut used = 0;

    for choice in names.choices(vfs).into_iter().filter(|choice| choice.matches(wanted)) {
        if used == CHOOSER_COLUMNS {
            rows = rows.push(centred(line.into()));
            line = Row::new().spacing(ROW_GAP).align_y(Vertical::Center);
            used = 0;
        }

        line = line.push(names.tile(vfs, choice, index));
        used += 1;
    }

    scrolled(rows.push(centred(line.into())))
}

fn striped<'a>(content: Element<'a, Message>, index: usize) -> Element<'a, Message> {
    let framed = container(content)
        .padding(ROW_PADDING)
        .style(move |theme: &Theme| theme::zebra_table_row(theme, index));

    centred(framed.into())
}

fn headings<'a>() -> Element<'a, Message> {
    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (heading, width) in HEADINGS.iter().zip(*WIDTHS) {
        line = line.push(
            container(
                text(heading.label).size(LABEL_SIZE).align_x(Horizontal::Center).width(Length::Fill),
            )
            .width(Length::Fixed(width)),
        );
    }

    let framed = container(line.width(Length::Fixed(*TABLE_WIDTH)))
        .padding(ROW_PADDING)
        .style(theme::zebra_table_header);

    centred(framed.into())
}

fn centred<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).width(Length::Fill).center_x(Length::Fill).into()
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

    let mut cells = vec![
        skills,
        picture,
        drills(draft.reads_at(base + TEXT_ID).unwrap_or_default().to_string(), base + TEXT_ID),
        cards::number(draft, base + COST_ID),
        ultra(draft.reads_at(base + LIMIT).unwrap_or_default(), base + LIMIT),
        cards::number(draft, base + MAX_LEVEL),
    ];

    for pair in 0..VALUES {
        let low = base + MIN_1 + pair * 2;
        let meaning = values::value(ability, pair);

        cells.push(explained(cards::number(draft, low), meaning));
        cells.push(explained(cards::number(draft, low + 1), meaning));
    }

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (cell, width) in cells.into_iter().zip(*WIDTHS) {
        line = line.push(container(cell).width(Length::Fixed(width)));
    }

    line.width(Length::Fixed(*TABLE_WIDTH)).into()
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

fn explained<'a>(
    field: Element<'a, Message>,
    meaning: Option<values::Value>,
) -> Element<'a, Message> {
    let Some(value) = meaning else {
        return field;
    };

    let bubble = container(text(value.hint()).size(TRAIT_SIZE))
        .padding(HINT_PADDING)
        .style(container::bordered_box);

    tooltip(field, bubble, tooltip::Position::Top).into()
}

fn offset(index: usize) -> usize {
    index.saturating_sub(TALENT_HEAD) % TALENT_STRIDE
}

fn drills<'a>(label: String, index: usize) -> Element<'a, Message> {
    button(theme::centered_text(label).size(TRAIT_SIZE).width(Length::Fill).wrapping(text::Wrapping::None))
        .width(Length::Fill)
        .padding(TRAIT_INSET)
        .style(theme::neutral_button)
        .on_press(Message::Picker(Some(index)))
        .into()
}

fn wording<'a>(vault: &Vault, index: usize, hunt: &str) -> Element<'a, Message> {
    let texts = vault.vds.cats.descriptions(&vault.vfs);
    let wanted = hunt.trim().to_lowercase();
    let numeric = !wanted.is_empty() && wanted.chars().all(|glyph| glyph.is_ascii_digit());

    let mut rows = Column::new().spacing(ROW_GAP);

    for (id, body) in texts.iter().enumerate() {
        if body.trim().is_empty() {
            continue;
        }

        let kept = match (wanted.is_empty(), numeric) {
            (true, _) => true,
            (false, true) => id.to_string().contains(&wanted),
            (false, false) => body.to_lowercase().contains(&wanted),
        };

        if !kept {
            continue;
        }

        let label = text(format!("{id}  {}", body.replace('\n', " "))).size(TRAIT_SIZE);

        rows = rows.push(
            button(label)
                .width(Length::Fill)
                .padding(TRAIT_INSET)
                .style(theme::neutral_button)
                .on_press(Message::Picked(index, id as i32)),
        );
    }

    scrolled(rows)
}

fn back<'a>() -> Element<'a, Message> {
    container(
        button(theme::centered_text("Back").size(TRAIT_SIZE))
            .width(Length::Fixed(TRAIT_WIDTH))
            .padding(TRAIT_INSET)
            .style(theme::neutral_button)
            .on_press(Message::Picker(None)),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding(ROW_PADDING)
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
    name: Option<&'static str>,
}

impl Skill {
    fn of(id: i32) -> Skill {
        CATALOGUE.iter().copied().find(|skill| skill.id == id).unwrap_or(Skill { id, name: None })
    }
}

impl fmt::Display for Skill {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => formatter.write_str(name),
            None => write!(formatter, "{}", self.id),
        }
    }
}

static CATALOGUE: LazyLock<Vec<Skill>> = LazyLock::new(|| {
    let mut listed = vec![Skill { id: 0, name: Some("None") }];

    listed.extend((1..=u8::MAX).filter_map(|id| {
        let ability = get_talent(id)?;
        let name = get_display_def(ability.identity).name;

        Some(Skill { id: i32::from(id), name: (!name.is_empty()).then_some(name) })
    }));

    listed
});

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Named(i32);

impl Named {
    fn matches(self, wanted: &str) -> bool {
        wanted.is_empty() || self.to_string().to_lowercase().contains(&wanted.to_lowercase())
    }
}

impl fmt::Display for Named {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            return formatter.write_str(NO_IMAGE);
        }

        write!(formatter, "{}", self.0)
    }
}

#[derive(Default)]
pub(super) struct Names {
    listed: RefCell<Option<Vec<Named>>>,
    images: RefCell<FxHashMap<i32, Option<Handle>>>,
}

impl Names {
    pub(super) fn forget(&self) {
        self.listed.borrow_mut().take();
        self.images.borrow_mut().clear();
    }

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

pub(super) fn rule(index: usize, cells: &[i32]) -> Rule {
    let Some(slot) = slot_of(index) else {
        return Rule::Plain;
    };

    let position = offset(index);

    if position == NAME_ID {
        return Rule::Floor(-1);
    }

    let Some(pair) = position.checked_sub(MIN_1).map(|span| span / 2).filter(|pair| *pair < VALUES)
    else {
        return Rule::Plain;
    };

    let ability = cells.get(TALENT_HEAD + slot * TALENT_STRIDE).copied().unwrap_or_default();

    match values::value(ability, pair) {
        Some(meaning) if meaning.unit().percent() => Rule::Percent,
        _ => Rule::Plain,
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
            assert!(
                skill.name.is_some(),
                "talent ability {} resolves to no name, so the picker falls back to its number",
                skill.id,
            );
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
