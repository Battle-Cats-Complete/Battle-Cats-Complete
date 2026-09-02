use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, rule, scrollable, space, stack, text, text_input, Column, Space};
use iced::widget::responsive;
use iced::{widget, Element, Font, Length, Padding, Size, Task, Theme};
use tracing::warn;

use kore::common::architecture;
use kore::common::preview::{self, Stamp};
use kore::domains::cat::animation as cat_animation;
use kore::domains::cat::scanner::CatEntry;
use kore::domains::enemy::animation as enemy_animation;
use kore::domains::enemy::scanner::EnemyEntry;
use kore::domains::mods;
use kore::domains::settings::{AnimSettings, Settings};
use kore::systems::animation::authoring::{ease_label, kind_label, loop_label, Maanim};
use kore::systems::animation::ClipSet;
use nyanko::graphics::rig::{Model, ModelPart, Rig};
use kore::Vfs;

use crate::app::state::AnimState;
use crate::app::{theme, Page};
use crate::systems::animation as viewer;
use crate::common::row_window::{self, RowWindow};
use crate::widget::{list_row, popup, smooth_scroll};

const PANEL_WIDTH: f32 = 296.0;
const STRIP_HEIGHT: f32 = 178.0;
const PANEL_PADDING: f32 = 10.0;
const PANEL_TITLE_SIZE: f32 = 16.0;
const BODY_PADDING: f32 = 10.0;
const GAP: f32 = 8.0;
const ROW_GAP: f32 = 4.0;

const CLOSE_LABEL: &str = "\u{00d7}";
const CLOSE_TEXT_SIZE: f32 = 24.0;

const LABEL_SIZE: f32 = 12.0;
const CELL_SIZE: f32 = 12.0;
const CELL_PADDING: f32 = 4.0;
const INDEX_WIDTH: f32 = 26.0;
const ROW_HEIGHT: f32 = 21.0;
const ROW_SPACING: f32 = 0.0;
const ROW_PADDING: f32 = 6.0;
const INDENT: f32 = 11.0;

const TREE_TEXT_SIZE: f32 = 11.0;
const CHAR_WIDTH: f32 = TREE_TEXT_SIZE * 0.65;

const MARKER_SIZE: f32 = 18.0;
const MARKER_LINE_HEIGHT: f32 = ROW_HEIGHT / MARKER_SIZE;
const MARKER_WIDTH: f32 = 14.0;

const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const SCROLLBAR_ALLOWANCE: f32 = 14.0;
const SCROLL_TAIL: f32 = 14.0;
const KEY_ROW_HEIGHT: f32 = 26.0;

const FOLDER_OPEN: &str = "\u{25be}";
const FOLDER_SHUT: &str = "\u{25b8}";
const EASE_WIDTH: f32 = 66.0;
const DROP_WIDTH: f32 = 20.0;
const FACT_LABEL: f32 = 72.0;
const LOOP_WIDTH: f32 = 56.0;

const LOADING_NOTICE: &str = "Loading animation\u{2026}";
const NO_CLIP_NOTICE: &str = "Select an animation in the viewer to edit its curves.";
const UNREADABLE_NOTICE: &str = "This animation could not be read.";
const EMPTY_TRACK_NOTICE: &str = "This curve holds no keyframes.";
const WRITE_FAILED_NOTICE: &str = "The last change could not be saved.";
const NEVER_DRAWN_NOTICE: &str = "The engine never draws this part, so edits to it will not show.";
const NO_SPRITE_NOTICE: &str = "This part has no sprite, so the engine never draws it.";
const TRANSPARENT_NOTICE: &str = "Transparent at rest. It stays invisible until an Opacity curve raises it.";
const FLAT_NOTICE: &str = "Zero scale at rest. It stays invisible until a Scale curve grows it.";
const LOOSE_LABEL: &str = "Curves with no declared part";
const BARREN_LABEL: &str = "No children or curves";
const NOT_DRAWN_MARK: &str = "not drawn";
const SHADOWED_MARK: &str = "overridden";
const NO_PART_NOTICE: &str = "This curve drives a part the model does not declare.";

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Subject {
    Cat { id: u32, form: usize },
    Enemy { id: u32 },
}

impl Subject {
    fn page(self) -> Page {
        match self {
            Subject::Cat { .. } => Page::Cats,
            Subject::Enemy { .. } => Page::Enemies,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lens {
    Rig,
    Selected,
    Hierarchy,
}

impl Lens {
    const ALL: [Lens; 3] = [Lens::Rig, Lens::Selected, Lens::Hierarchy];

    fn label(self) -> &'static str {
        match self {
            Lens::Rig => "Rig",
            Lens::Selected => "Selected",
            Lens::Hierarchy => "Hierarchy",
        }
    }

    fn of(self, anim: &mut AnimSettings) -> &mut bool {
        match self {
            Lens::Rig => &mut anim.show_rig,
            Lens::Selected => &mut anim.show_selected,
            Lens::Hierarchy => &mut anim.show_hierarchy,
        }
    }

    fn on(self, anim: &AnimSettings) -> bool {
        match self {
            Lens::Rig => anim.show_rig,
            Lens::Selected => anim.show_selected,
            Lens::Hierarchy => anim.show_hierarchy,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Frame,
    Value,
    Ease,
    Power,
}

impl Field {
    const ALL: [Field; 4] = [Field::Frame, Field::Value, Field::Ease, Field::Power];

    fn label(self) -> &'static str {
        match self {
            Field::Frame => "Frame",
            Field::Value => "Value",
            Field::Ease => "Ease",
            Field::Power => "Power",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Clone)]
pub(super) struct Plan {
    subject: Subject,
    key: String,
    target_mod: Option<String>,
    clip: Option<String>,
}

pub(super) fn plan(
    subject: Subject,
    key: String,
    target_mod: Option<String>,
    clip: Option<String>,
) -> Plan {
    Plan { subject, key, target_mod, clip }
}

#[derive(Debug, Clone)]
pub enum Message {
    Close,
    Tick,
    Viewer(viewer::Message),
    Row(usize),
    Scrolled(f32),
    StripScrolled(f32),
    Changed(usize, Field, String),
    LoopChanged(String),
    AddKey,
    DropKey(usize),
    Overlay(Lens),
    Persisted(u64, PathBuf, Option<Stamp>),
}

pub(crate) struct Feed<'a> {
    pub settings: &'a mut Settings,
    pub anim: &'a mut AnimState,
    pub vfs: &'a Vfs,
    pub cats: &'a [CatEntry],
    pub enemies: &'a [EnemyEntry],
}

impl Feed<'_> {
    fn clips(&self, subject: Subject) -> ClipSet {
        match subject {
            Subject::Cat { id, form } => self
                .cats
                .iter()
                .find(|cat| cat.id == id)
                .map_or_else(ClipSet::default, |cat| cat_animation::clips(cat, form, self.vfs)),
            Subject::Enemy { id } => self
                .enemies
                .iter()
                .find(|enemy| enemy.id == id)
                .map_or_else(ClipSet::default, |enemy| enemy_animation::clips(enemy, self.vfs)),
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    session: Option<Session>,
}

struct Session {
    plan: Plan,
    viewer: viewer::State,
    draft: Option<Draft>,
    opened: Option<PathBuf>,
    expanded: HashSet<usize>,
    rows: Vec<TreeRow>,
    widest: f32,
    listed_rig: bool,
    scroll: f32,
    scroll_id: widget::Id,
    strip_scroll: f32,
    strip_id: widget::Id,
    primed: bool,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan) {
        self.session = Some(Session {
            plan,
            viewer: viewer::State::with_popup(popup::Kind::Animator),
            draft: None,
            opened: None,
            expanded: HashSet::new(),
            rows: Vec::new(),
            widest: 0.0,
            listed_rig: false,
            scroll: 0.0,
            scroll_id: widget::Id::unique(),
            strip_scroll: 0.0,
            strip_id: widget::Id::unique(),
            primed: false,
        });
    }

    pub(super) fn active(&self) -> bool {
        self.session.is_some()
    }

    pub(super) fn page(&self) -> Option<Page> {
        self.session.as_ref().map(|session| session.plan.subject.page())
    }

    pub(super) fn flush_now(&mut self, vfs: &Vfs) {
        let Some(draft) = self.session.as_mut().and_then(|session| session.draft.as_mut()) else {
            return;
        };

        draft.persist_now(vfs);
    }

    pub(super) fn update(&mut self, message: Message, feed: Feed<'_>) -> Task<Message> {
        let Some(session) = self.session.as_mut() else {
            return Task::none();
        };

        match message {
            Message::Close => {
                if let Some(draft) = session.draft.as_mut() {
                    draft.persist_now(feed.vfs);
                }

                self.session = None;

                Task::none()
            }
            Message::Tick => {
                let priming = session.sync(&feed);
                session.viewer.tick();

                let flush =
                    session.draft.as_mut().map_or_else(Task::none, |draft| draft.persist_if_dirty(feed.vfs));

                Task::batch([priming, flush])
            }
            Message::Viewer(msg) => {
                session.viewer.update(msg, feed.settings, feed.anim).map(Message::Viewer)
            }
            Message::Row(index) => {
                let Some((part, track)) = session.rows.get(index).map(|row| (row.part, row.track)) else {
                    return Task::none();
                };

                if let Some(track) = track {
                    if let Some(draft) = session.draft.as_mut() {
                        draft.retrack(track);
                    }

                    session.aim();

                    return Task::none();
                }

                if let Some(part) = part {
                    if !session.expanded.remove(&part) {
                        session.expanded.insert(part);
                    }

                    session.relist();
                }

                Task::none()
            }
            Message::Scrolled(offset) => {
                session.scroll = offset;

                Task::none()
            }
            Message::StripScrolled(offset) => {
                session.strip_scroll = offset;

                Task::none()
            }
            Message::Overlay(lens) => {
                let flag = lens.of(&mut feed.settings.animation);
                *flag = !*flag;

                Task::none()
            }
            Message::LoopChanged(value) => {
                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.set_looping(&value);

                draft.persist_if_dirty(feed.vfs)
            }
            Message::AddKey => {
                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.add_key();

                draft.persist_if_dirty(feed.vfs)
            }
            Message::DropKey(at) => {
                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.drop_key(at);

                draft.persist_if_dirty(feed.vfs)
            }
            Message::Changed(at, field, value) => {
                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.edit(at, field, &value);

                draft.persist_if_dirty(feed.vfs)
            }
            Message::Persisted(token, path, stamp) => {
                let Some(draft) = session.draft.as_mut().filter(|draft| draft.token == token) else {
                    return Task::none();
                };

                let settled = draft.settle(path, stamp);
                let updated = draft.doc.shared();

                match settled {
                    Settled::Failed => {}
                    Settled::Saved => {
                        if let Some(showing) = session.viewer.selected_anim().cloned() {
                            session.viewer.adopt_anim(&showing, updated);
                        }
                    }
                    Settled::Moved => {
                        session.opened = None;
                        session.viewer.invalidate_paths();
                    }
                }

                Task::none()
            }
        }
    }

    pub(super) fn view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Option<Element<'a, Message>> {
        self.session.as_ref().map(|session| session.view(settings, anim))
    }
}

impl Session {
    fn sync(&mut self, feed: &Feed<'_>) -> Task<Message> {
        let subject = self.plan.subject;
        let mut priming = Task::none();

        if std::mem::replace(&mut self.primed, true) {
            self.viewer.sync(&self.plan.key, || feed.clips(subject), feed.settings, feed.anim);
        } else {
            priming =
                self.viewer.preload(&self.plan.key, || feed.clips(subject), feed.anim).map(Message::Viewer);

            if let Some(label) = self.plan.clip.as_deref() {
                self.viewer.select_label(label);
            }
        }

        let selected = self.viewer.selected_anim().cloned();

        if selected == self.opened && !self.draft.as_ref().is_some_and(|draft| draft.drifted()) {
            if self.draft.is_some() && self.listed_rig != self.viewer.rig().is_some() {
                self.relist();
            }

            return priming;
        }

        self.opened = selected.clone();
        self.draft = selected
            .as_deref()
            .and_then(|anim| Draft::load(anim, self.plan.target_mod.as_deref(), feed.vfs));

        self.aim();
        self.relist();

        priming
    }

    fn relist(&mut self) {
        let listed = match self.draft.as_ref() {
            Some(draft) => listing(&draft.doc, self.viewer.rig().map(|rig| &rig.model), &self.expanded),
            None => Vec::new(),
        };

        self.listed_rig = self.viewer.rig().is_some();
        self.widest = listed.iter().map(TreeRow::span).fold(0.0, f32::max);
        self.rows = listed;
    }

    fn aim(&mut self) {
        let part = self.draft.as_ref().and_then(Draft::part).and_then(|part| usize::try_from(part).ok());

        self.viewer.set_highlight(part);
    }

    fn view<'a>(&'a self, settings: &'a Settings, anim: &'a AnimState) -> Element<'a, Message> {
        let showing: Element<'_, Message> = if self.viewer.resolved() {
            self.viewer.view(settings, anim).map(Message::Viewer)
        } else {
            container(text(LOADING_NOTICE).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let stage = container(showing).width(Length::Fill).height(Length::Fill);

        let body = row![self.side(settings), column![stage, self.strip()].spacing(GAP)].spacing(GAP);

        let content = container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..container::Style::default()
            });

        let mut layers = stack![content, close_button()];

        if let Some(expanded) = self.viewer.expanded_view(settings, anim) {
            layers = layers.push(expanded.map(Message::Viewer));
        }

        layers.into()
    }

    fn side<'a>(&'a self, settings: &'a Settings) -> Element<'a, Message> {
        let toggle = outlines_row(&settings.animation);

        let Some(draft) = self.draft.as_ref() else {
            let body =
                column![toggle, text(self.vacancy()).size(LABEL_SIZE)].spacing(GAP).height(Length::Fill);

            return panel_frame(body.into(), &self.plan.key);
        };

        let body = column![toggle, self.tree(), draft.inspector(self.viewer.rig())]
            .spacing(GAP)
            .height(Length::Fill);

        panel_frame(body.into(), &draft.file)
    }

    fn tree(&self) -> Element<'_, Message> {
        let picked = self.draft.as_ref().map(|draft| draft.track);
        let body = responsive(move |size: Size| self.view_rows(size, picked));

        container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::mock_console_container)
            .into()
    }

    fn view_rows(&self, size: Size, picked: Option<usize>) -> Element<'_, Message> {
        let tail = if self.widest > size.width { SCROLL_TAIL } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } = row_window::compute_with(
            self.rows.len(),
            size.height - tail,
            self.scroll,
            ROW_HEIGHT,
            ROW_SPACING,
        );

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let mut list = Column::with_capacity(range.len() + 3).spacing(ROW_SPACING);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for index in range {
            let Some(row) = self.rows.get(index) else {
                continue;
            };

            list = list.push(row.view(index, picked, width));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if tail > 0.0 {
            list = list.push(space().height(Length::Fixed(tail)));
        }

        smooth_scroll(
            scrollable(list)
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
    }

    fn strip(&self) -> Element<'_, Message> {
        let Some(draft) = self.draft.as_ref() else {
            return Space::new().height(Length::Fixed(0.0)).into();
        };

        let add = button(theme::button_label("Add Keyframe").size(LABEL_SIZE))
            .width(Length::Fill)
            .padding([3, 6])
            .on_press(Message::AddKey)
            .style(theme::primary_button);

        if draft.inputs.is_empty() {
            let body = column![text(EMPTY_TRACK_NOTICE).size(LABEL_SIZE), add].spacing(ROW_GAP);

            return container(body).width(Length::Fill).padding(PANEL_PADDING).into();
        }

        let table = column![keys_header(), responsive(move |size: Size| self.view_keys(draft, size))]
            .spacing(ROW_GAP)
            .height(Length::Fill);

        let notice: Element<'_, Message> = match draft.failed {
            true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
            false => Space::new().height(Length::Fixed(0.0)).into(),
        };

        container(column![notice, table, add].spacing(ROW_GAP))
            .width(Length::Fill)
            .height(Length::Fixed(STRIP_HEIGHT))
            .padding(PANEL_PADDING)
            .into()
    }

    fn view_keys<'a>(&'a self, draft: &'a Draft, size: Size) -> Element<'a, Message> {
        let RowWindow { range, pad_before, pad_after } = row_window::compute_with(
            draft.inputs.len(),
            size.height,
            self.strip_scroll,
            KEY_ROW_HEIGHT,
            ROW_GAP,
        );

        let mut list = Column::with_capacity(range.len() + 2).spacing(ROW_GAP);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for at in range {
            list = list.push(draft.key_row(at));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        smooth_scroll(
            scrollable(list)
                .id(self.strip_id.clone())
                .on_scroll(|viewport| Message::StripScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
    }

    fn vacancy(&self) -> &'static str {
        if self.opened.is_some() { UNREADABLE_NOTICE } else { NO_CLIP_NOTICE }
    }
}


struct TreeRow {
    label: String,
    depth: u16,
    mark: &'static str,
    part: Option<usize>,
    track: Option<usize>,
    warn: bool,
    inert: bool,
}

impl TreeRow {
    fn span(&self) -> f32 {
        ROW_PADDING * 2.0
            + MARKER_WIDTH
            + INDENT * f32::from(self.depth)
            + CHAR_WIDTH * self.label.chars().count() as f32
    }

    fn view(&self, index: usize, picked: Option<usize>, width: f32) -> Element<'_, Message> {
        let label = text(self.label.as_str())
            .font(Font::MONOSPACE)
            .size(TREE_TEXT_SIZE)
            .wrapping(text::Wrapping::None);

        let label = if self.warn { label.style(text::danger) } else { label };

        let body = row![
            text(self.mark)
                .font(Font::MONOSPACE)
                .size(MARKER_SIZE)
                .line_height(MARKER_LINE_HEIGHT)
                .width(Length::Fixed(MARKER_WIDTH)),
            label,
        ]
        .align_y(Vertical::Center);

        let content = container(body)
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding(
                Padding::default()
                    .left(ROW_PADDING + INDENT * f32::from(self.depth))
                    .right(ROW_PADDING),
            );

        if self.inert {
            return container(content).width(Length::Fixed(width)).into();
        }

        let selected = self.track.is_some() && self.track == picked;

        list_row(content, selected, false, Length::Fixed(width), Message::Row(index))
    }
}

fn keys_header<'a>() -> Element<'a, Message> {
    Field::ALL
        .iter()
        .fold(
            row![Space::new().width(Length::Fixed(INDEX_WIDTH))].spacing(ROW_GAP),
            |header, field| header.push(theme::centered_text(field.label()).size(LABEL_SIZE).width(Length::Fill)),
        )
        .push(theme::centered_text("Curve").size(LABEL_SIZE).width(Length::Fixed(EASE_WIDTH)))
        .push(Space::new().width(Length::Fixed(DROP_WIDTH)))
        .into()
}

fn both_ways() -> scrollable::Direction {
    let bar = || scrollable::Scrollbar::new().width(SCROLLBAR_WIDTH).margin(SCROLLBAR_MARGIN);

    scrollable::Direction::Both { vertical: bar(), horizontal: bar() }
}

fn curve_label(doc: &Maanim, at: usize) -> Option<(String, bool)> {
    let track = doc.track(at)?;

    let shadowed = doc
        .tracks()
        .iter()
        .skip(at + 1)
        .any(|later| later.part == track.part && later.kind == track.kind);

    let mut label = format!("{} \u{00b7} {} keys", kind_label(track.kind), track.keyframes.len());

    if track.loop_count != 1 {
        label.push_str(&format!(" \u{00b7} {}", loop_label(track.loop_count)));
    }

    let name = track.name.trim();

    if !name.is_empty() {
        label.push_str(&format!(" \u{00b7} {}", name));
    }

    if shadowed {
        label.push_str(&format!(" \u{00b7} {}", SHADOWED_MARK));
    }

    Some((label, shadowed))
}

fn part_label(model: &Model, at: usize) -> String {
    let Some(declared) = model.parts.get(at) else {
        return format!("Part {}", at);
    };

    let mut label = match declared.name.trim() {
        "" => format!("Part {}", at),
        name => format!("Part {} \u{00b7} {}", at, name),
    };

    if hidden(declared).is_some() {
        label.push_str(&format!(" \u{00b7} {}", NOT_DRAWN_MARK));
    }

    label
}

fn leaf(label: String, depth: u16, track: Option<usize>, warn: bool) -> TreeRow {
    TreeRow { label, depth, mark: "", part: None, track, warn, inert: false }
}

fn listing(doc: &Maanim, model: Option<&Model>, expanded: &HashSet<usize>) -> Vec<TreeRow> {
    let Some(model) = model else {
        return (0..doc.tracks().len())
            .filter_map(|at| curve_label(doc, at).map(|(label, warn)| leaf(label, 0, Some(at), warn)))
            .collect();
    };

    let mut listed = Vec::new();
    let count = model.parts.len();

    for (part, depth) in rows(model, expanded) {
        let curves: Vec<usize> = (0..doc.tracks().len())
            .filter(|at| doc.track(*at).is_some_and(|track| usize::try_from(track.part) == Ok(part)))
            .collect();

        let open = expanded.contains(&part);
        let depth = depth as u16;

        listed.push(TreeRow {
            label: part_label(model, part),
            depth,
            mark: if open { FOLDER_OPEN } else { FOLDER_SHUT },
            part: Some(part),
            track: None,
            warn: false,
            inert: false,
        });

        if !open {
            continue;
        }

        if curves.is_empty() && !bears(model, part) {
            listed.push(TreeRow {
                label: BARREN_LABEL.to_string(),
                depth: depth + 1,
                mark: "",
                part: None,
                track: None,
                warn: false,
                inert: true,
            });
        }

        for at in curves {
            if let Some((label, warn)) = curve_label(doc, at) {
                listed.push(leaf(label, depth + 1, Some(at), warn));
            }
        }
    }

    let loose: Vec<usize> = (0..doc.tracks().len())
        .filter(|at| doc.track(*at).is_none_or(|track| !usize::try_from(track.part).is_ok_and(|part| part < count)))
        .collect();

    if !loose.is_empty() {
        listed.push(leaf(LOOSE_LABEL.to_string(), 0, None, true));

        for at in loose {
            if let Some((label, warn)) = curve_label(doc, at) {
                listed.push(leaf(label, 1, Some(at), warn));
            }
        }
    }

    listed
}

fn rows(model: &Model, expanded: &HashSet<usize>) -> Vec<(usize, usize)> {
    let count = model.parts.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut roots = Vec::new();

    for (at, part) in model.parts.iter().enumerate() {
        let parent = usize::try_from(part.parent).ok().filter(|parent| *parent < count && *parent != at);

        match parent {
            Some(parent) => children[parent].push(at),
            None => roots.push(at),
        }
    }

    let mut listed = Vec::with_capacity(count);
    let mut seen = vec![false; count];

    for root in roots {
        descend(root, 0, &children, expanded, &mut seen, &mut listed, true);
    }

    for at in 0..count {
        descend(at, 0, &children, expanded, &mut seen, &mut listed, true);
    }

    listed
}

fn descend(
    at: usize,
    depth: usize,
    children: &[Vec<usize>],
    expanded: &HashSet<usize>,
    seen: &mut [bool],
    listed: &mut Vec<(usize, usize)>,
    visible: bool,
) {
    if seen.get(at).copied().unwrap_or(true) {
        return;
    }

    seen[at] = true;

    if visible {
        listed.push((at, depth));
    }

    let open = visible && expanded.contains(&at);

    for child in children.get(at).into_iter().flatten() {
        descend(*child, depth + 1, children, expanded, seen, listed, open);
    }
}

fn hidden(part: &ModelPart) -> Option<&'static str> {
    if part.id < 0 {
        return Some(NEVER_DRAWN_NOTICE);
    }

    if part.sprite < 0 {
        return Some(NO_SPRITE_NOTICE);
    }

    if part.opacity == 0 {
        return Some(TRANSPARENT_NOTICE);
    }

    if part.scale_x == 0 || part.scale_y == 0 {
        return Some(FLAT_NOTICE);
    }

    None
}

fn bears(model: &Model, part: usize) -> bool {
    let wanted = i32::try_from(part).ok();

    model.parts.iter().enumerate().any(|(at, other)| at != part && Some(other.parent) == wanted)
}

fn outlines_row(anim: &AnimSettings) -> Element<'_, Message> {
    Lens::ALL
        .iter()
        .fold(row![].spacing(ROW_GAP), |listed, lens| {
            let on = lens.on(anim);

            listed.push(
                button(theme::centered_text(lens.label()).size(LABEL_SIZE).width(Length::Fill))
                    .width(Length::Fill)
                    .padding([3, 4])
                    .on_press(Message::Overlay(*lens))
                    .style(move |theme: &Theme, status| theme::toggle_button(theme, status, on)),
            )
        })
        .into()
}

fn fact<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label).size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)),
        text(value).size(LABEL_SIZE),
    ]
    .spacing(ROW_GAP)
    .align_y(Vertical::Center)
    .into()
}

fn panel_frame<'a>(body: Element<'a, Message>, title: &str) -> Element<'a, Message> {
    let framed = column![theme::bold_text(title).size(PANEL_TITLE_SIZE), rule::horizontal(1), body]
        .spacing(ROW_GAP)
        .height(Length::Fill);

    container(framed)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Length::Fill)
        .padding(PANEL_PADDING)
        .into()
}

fn close_button<'a>() -> Element<'a, Message> {
    let close = button(
        theme::centered_text(CLOSE_LABEL)
            .size(CLOSE_TEXT_SIZE)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Center),
    )
    .width(Length::Fixed(theme::NAV_TOGGLE_SIZE))
    .height(Length::Fixed(theme::NAV_TOGGLE_SIZE))
    .padding(0)
    .on_press(Message::Close)
    .style(theme::danger_button);

    let slot = column![close].padding(Padding {
        top: theme::NAV_TOGGLE_TOP,
        right: theme::NAV_TOGGLE_RIGHT,
        bottom: 0.0,
        left: 0.0,
    });

    container(row![slot].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_right(Length::Fill)
        .into()
}

enum Settled {
    Saved,
    Moved,
    Failed,
}

struct Draft {
    file: String,
    game: PathBuf,
    target_mod: Option<String>,
    read_from: PathBuf,
    stamp: Stamp,
    doc: Maanim,
    track: usize,
    inputs: Vec<[String; 4]>,
    looping: String,
    dirty: bool,
    writing: bool,
    failed: bool,
    token: u64,
}

impl Draft {
    fn load(anim: &Path, target_mod: Option<&str>, vfs: &Vfs) -> Option<Draft> {
        let file = anim.file_name()?.to_str()?.to_owned();
        let game = vfs.rooted(architecture::GAME, &file).unwrap_or_else(|| anim.to_path_buf());
        let read_from = target_mod
            .and_then(|name| mods::find(vfs, name, &game))
            .unwrap_or_else(|| game.clone());

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Animation editor could not read the file: {}", err))
            .ok()?;

        let Some(stamp) = preview::stamp(&read_from) else {
            warn!(path = %read_from.display(), "Animation editor could not stamp the file");

            return None;
        };

        let doc = Maanim::parse(&bytes)
            .inspect_err(|err| warn!(path = %read_from.display(), "Animation editor could not parse the file: {}", err))
            .ok()?;

        let mut draft = Draft {
            file,
            game,
            target_mod: target_mod.map(str::to_owned),
            read_from,
            stamp,
            doc,
            track: 0,
            inputs: Vec::new(),
            looping: String::new(),
            dirty: false,
            writing: false,
            failed: false,
            token: next_token(),
        };

        draft.restate();

        Some(draft)
    }

    fn drifted(&self) -> bool {
        !self.dirty && !self.writing && preview::stamp(&self.read_from) != Some(self.stamp)
    }

    fn retrack(&mut self, at: usize) {
        if at >= self.doc.tracks().len() {
            return;
        }

        self.track = at;
        self.restate();
    }

    fn restate(&mut self) {
        let track = self.doc.track(self.track);

        self.looping = track.map_or_else(String::new, |track| track.loop_count.to_string());
        self.inputs = track
            .map(|track| {
                track
                    .keyframes
                    .iter()
                    .map(|key| {
                        [key.frame, key.value, key.ease, key.ease_power].map(|cell| cell.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    fn set_looping(&mut self, value: &str) {
        if !typable(value) {
            return;
        }

        self.looping = value.to_owned();

        let Ok(parsed) = value.parse::<i32>() else {
            return;
        };

        let Some(track) = self.doc.edit(self.track) else {
            return;
        };

        if track.loop_count == parsed {
            return;
        }

        track.loop_count = parsed;
        self.dirty = true;
    }

    fn add_key(&mut self) {
        if self.doc.add_key(self.track).is_none() {
            return;
        }

        self.restate();
        self.dirty = true;
    }

    fn drop_key(&mut self, at: usize) {
        if !self.doc.remove_key(self.track, at) {
            return;
        }

        self.restate();
        self.dirty = true;
    }

    fn part(&self) -> Option<i32> {
        self.doc.track(self.track).map(|track| track.part)
    }

    fn edit(&mut self, at: usize, field: Field, value: &str) {
        if !typable(value) {
            return;
        }

        let Some(slot) = self.inputs.get_mut(at).and_then(|row| row.get_mut(field.slot())) else {
            return;
        };

        *slot = value.to_owned();

        let Ok(parsed) = value.parse::<i32>() else {
            return;
        };

        let track = self.track;
        let Some(key) = self.doc.edit(track).and_then(|track| track.keyframes.get_mut(at)) else {
            return;
        };

        let cell = match field {
            Field::Frame => &mut key.frame,
            Field::Value => &mut key.value,
            Field::Ease => &mut key.ease,
            Field::Power => &mut key.ease_power,
        };

        if *cell == parsed {
            return;
        }

        *cell = parsed;
        self.dirty = true;

        if field == Field::Frame {
            self.doc.sort_keys(track);
            self.restate();
        }
    }

    fn destination(&self, vfs: &Vfs) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.target_mod.as_deref() else {
            return Some((self.game.clone(), self.stamp));
        };

        if self.read_from != self.game {
            return Some((self.read_from.clone(), self.stamp));
        }

        let path = mods::ensure_as(vfs, name, &self.game, &self.file)
            .inspect_err(|err| warn!(source = %self.game.display(), "Animation editor could not stage the file: {}", err))
            .ok()?;

        if let Err(err) = vfs.create((name, path.as_path())) {
            warn!(path = %path.display(), "Animation editor could not index the staged file: {}", err);
        }

        let stamp = preview::stamp(&path)?;

        Some((path, stamp))
    }

    fn prepare_write(&mut self, vfs: &Vfs) -> Option<(PathBuf, Maanim, Stamp, u64)> {
        let Some((path, stamp)) = self.destination(vfs) else {
            self.failed = true;

            return None;
        };

        self.dirty = false;
        self.writing = true;

        Some((path, self.doc.clone(), stamp, self.token))
    }

    fn persist_if_dirty(&mut self, vfs: &Vfs) -> Task<Message> {
        if !self.dirty || self.writing {
            return Task::none();
        }

        let Some((path, doc, stamp, token)) = self.prepare_write(vfs) else {
            return Task::none();
        };

        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    fn persist_now(&mut self, vfs: &Vfs) {
        if !self.dirty && !self.writing {
            return;
        }

        let Some((path, doc, stamp, _)) = self.prepare_write(vfs) else {
            return;
        };

        let written = write_now(&path, &doc.write(), stamp);

        self.settle(path, written);
    }

    fn settle(&mut self, path: PathBuf, stamp: Option<Stamp>) -> Settled {
        self.writing = false;

        let Some(stamp) = stamp else {
            self.failed = true;

            return Settled::Failed;
        };

        let moved = self.read_from != path;

        self.read_from = path;
        self.stamp = stamp;
        self.failed = false;

        if moved { Settled::Moved } else { Settled::Saved }
    }

    fn inspector(&self, rig: Option<&Rig>) -> Element<'_, Message> {
        let count = self.doc.track(self.track).map_or(0, |track| track.loop_count);
        let looping = row![
            text("Loop").size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)),
            text_input("", &self.looping)
                .on_input(Message::LoopChanged)
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .width(Length::Fixed(LOOP_WIDTH))
                .style(theme::rounded_input),
            text(loop_label(count)).size(LABEL_SIZE),
        ]
        .spacing(ROW_GAP)
        .align_y(Vertical::Center);

        let Some(index) = self.part() else {
            return looping.into();
        };

        let part = usize::try_from(index).ok().zip(rig).and_then(|(at, rig)| rig.model.parts.get(at));

        let Some(part) = part else {
            return column![looping, text(NO_PART_NOTICE).size(LABEL_SIZE)].spacing(ROW_GAP).into();
        };

        let named = match part.name.trim() {
            "" => index.to_string(),
            name => format!("{} · {}", index, name),
        };

        let mut facts = column![
            Element::from(looping),
            fact("Part", named),
            fact("Parent", part.parent.to_string()),
            fact("Sprite", part.sprite.to_string()),
            fact("Z Order", part.z.to_string()),
            fact("Offset", format!("{}, {}", part.x, part.y)),
            fact("Pivot", format!("{}, {}", part.pivot_x, part.pivot_y)),
            fact("Scale", format!("{}, {}", part.scale_x, part.scale_y)),
        ]
        .spacing(ROW_GAP);

        if let Some(reason) = hidden(part) {
            facts = facts.push(text(reason).size(LABEL_SIZE).style(text::danger));
        }

        facts.into()
    }

    fn key_row(&self, at: usize) -> Element<'_, Message> {
        let cells = self.inputs.get(at).map_or(&[][..], |row| row.as_slice());
        let ease = self.doc.track(self.track).and_then(|track| track.keyframes.get(at)).map_or(0, |key| key.ease);

        let mut line = row![theme::centered_text(at.to_string())
            .size(LABEL_SIZE)
            .width(Length::Fixed(INDEX_WIDTH))]
        .spacing(ROW_GAP)
        .height(Length::Fixed(KEY_ROW_HEIGHT))
        .align_y(Vertical::Center);

        for field in Field::ALL {
            let value = cells.get(field.slot()).map_or("", String::as_str);

            line = line.push(
                text_input("", value)
                    .on_input(move |typed| Message::Changed(at, field, typed))
                    .size(CELL_SIZE)
                    .padding(CELL_PADDING)
                    .width(Length::Fill)
                    .style(theme::rounded_input),
            );
        }

        let drop = button(theme::centered_text(CLOSE_LABEL).size(CELL_SIZE))
            .width(Length::Fixed(DROP_WIDTH))
            .padding(0)
            .on_press(Message::DropKey(at))
            .style(theme::danger_button);

        line.push(text(ease_label(ease)).size(LABEL_SIZE).width(Length::Fixed(EASE_WIDTH))).push(drop).into()
    }
}

fn typable(value: &str) -> bool {
    let mut chars = value.chars();

    match chars.next() {
        None => true,
        Some('-') => chars.all(|digit| digit.is_ascii_digit()),
        Some(first) if first.is_ascii_digit() => chars.all(|digit| digit.is_ascii_digit()),
        Some(_) => false,
    }
}

fn write_now(path: &Path, body: &[u8], stamp: Stamp) -> Option<Stamp> {
    preview::save(path, body, stamp)
        .inspect_err(|err| warn!(path = %path.display(), "Animation editor could not write the file: {}", err))
        .ok()
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    fn model(parents: &[i32]) -> Model {
        Model {
            parts: parents.iter().map(|parent| ModelPart { parent: *parent, ..ModelPart::default() }).collect(),
            ..Model::default()
        }
    }


    const SAMPLE: &str = "[modelanim:animation]\n1\n2\n0,11,-1,0,0,\n1\n0,0,0,0\n1,4,-1,0,0,\n1\n0,0,0,0\n";

    fn doc() -> Maanim {
        Maanim::parse(SAMPLE.as_bytes()).expect("the sample parses")
    }

    #[test]
    fn a_part_owns_its_curves_and_its_children_sit_beside_them() {
        // Part 0 is the root and part 1 hangs off it, each driven by one curve.
        let listed = listing(&doc(), Some(&model(&[-1, 0])), &HashSet::from([0, 1]));

        let shape: Vec<(u16, bool)> =
            listed.iter().map(|row| (row.depth, row.track.is_some())).collect();

        assert_eq!(shape, vec![(0, false), (1, true), (1, false), (2, true)]);
    }

    #[test]
    fn a_tree_starts_fully_collapsed() {
        let listed = listing(&doc(), Some(&model(&[-1, 0])), &HashSet::new());

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].mark, FOLDER_SHUT);
    }

    #[test]
    fn a_part_with_nothing_under_it_still_opens_onto_a_notice() {
        // Every part folds, so an empty one has to say why rather than doing nothing.
        let listed = listing(&doc(), Some(&model(&[-1, 0, 1])), &HashSet::from([0, 1, 2]));
        let barren = listed.iter().find(|row| row.label == BARREN_LABEL).expect("the notice is listed");

        assert!(barren.inert);
        assert!(barren.track.is_none() && barren.part.is_none());
    }

    #[test]
    fn a_curve_naming_a_part_the_model_lacks_still_gets_listed() {
        // The engine does not bound check the part index, so the curve has to stay reachable.
        let listed = listing(&doc(), Some(&model(&[-1])), &HashSet::from([0])); 

        assert!(listed.iter().any(|row| row.label == LOOSE_LABEL && row.warn));
        assert_eq!(listed.iter().filter(|row| row.track.is_some()).count(), 2);
    }

    #[test]
    fn without_a_model_every_curve_still_lists_flat() {
        let listed = listing(&doc(), None, &HashSet::new());

        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|row| row.depth == 0 && row.track.is_some()));
    }

    #[test]
    fn a_hierarchy_lists_parents_before_their_children() {
        // 0 is the root, 1 and 3 hang off it, 2 hangs off 1.
        let listed = rows(&model(&[-1, 0, 1, 0]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (2, 2), (3, 1)]);
    }

    #[test]
    fn a_folded_part_hides_its_descendants_but_not_its_siblings() {
        let listed = rows(&model(&[-1, 0, 1, 0]), &HashSet::from([0]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (3, 1)]);
    }

    #[test]
    fn a_parent_cycle_still_lists_every_part() {
        // The file is not bound checked, so 1 and 2 pointing at each other has to
        // stay visible rather than dropping out of the tree entirely.
        let listed = rows(&model(&[-1, 2, 1]), &HashSet::from([0, 1, 2]));

        assert_eq!(listed.len(), 3);
        assert!(listed.iter().any(|(part, _)| *part == 1));
        assert!(listed.iter().any(|(part, _)| *part == 2));
    }

    #[test]
    fn a_parent_past_the_end_is_treated_as_a_root() {
        let listed = rows(&model(&[-1, 9]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn a_part_parented_to_itself_does_not_recurse() {
        let listed = rows(&model(&[-1, 1]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }
}
