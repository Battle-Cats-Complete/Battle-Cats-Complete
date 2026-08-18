mod schema;

pub use schema::Subject;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, scrollable, text, text_input, Column, Row};
use iced::{Element, Length, Padding, Size, Task};
use nyanko::common::tools::file;
use tracing::warn;

use core::common::preview::{self, Stamp};
use core::domains::mods;

use crate::app::theme;
use crate::common::feedback::{Slot, CONFIRM_LABEL};
use crate::widget::{popup, smooth_scroll};

const CAT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::CatAttributes, Size::new(364.0, 376.0));
const ENEMY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::EnemyAttributes, Size::new(364.0, 376.0));

fn spec(subject: schema::Subject) -> popup::Spec {
    match subject {
        schema::Subject::Cat => CAT_POPUP,
        schema::Subject::Enemy => ENEMY_POPUP,
    }
}

const CARD_WIDTH: f32 = 86.0;
const CARD_GAP: f32 = 8.0;
const LABEL_SIZE: f32 = 12.0;
const INPUT_SIZE: f32 = 13.0;
const CARD_PADDING: f32 = 6.0;
const BODY_PADDING: f32 = 12.0;
const SYNC_WIDTH: f32 = 172.0;
const SEARCH_PADDING: f32 = 4.0;
const COMMENT: &str = "//";
const SEARCH_FRACTION: f32 = 2.0 / 3.0;
const LINE_RATIO: f32 = 1.3;
const GLYPH_RATIO: f32 = 0.55;

static LABEL_HEIGHT: LazyLock<f32> = LazyLock::new(|| {
    let inner = CARD_WIDTH - CARD_PADDING * 2.0;
    let per_line = ((inner / (LABEL_SIZE * GLYPH_RATIO)).floor() as usize).max(1);

    let lines = [&schema::CAT, &schema::ENEMY]
        .into_iter()
        .flat_map(|schema| (0..schema.known()).map(move |index| wrapped_lines(&schema.label(index), per_line)))
        .max()
        .unwrap_or(1);

    lines as f32 * LABEL_SIZE * LINE_RATIO
});

fn split_row(line: &str, delimiter: char, schema: &schema::Schema) -> (Vec<i32>, String, String) {
    let (numeric, gap, comment) = match line.find(COMMENT) {
        Some(at) => {
            let (head, tail) = line.split_at(at);
            let trimmed = head.trim_end();
            let gap = head[trimmed.len()..].to_owned();

            (trimmed, gap, tail[COMMENT.len()..].trim().to_owned())
        }
        None => (line, String::from(" "), String::new()),
    };

    let mut fields: Vec<&str> = numeric.split(delimiter).collect();

    while fields.last().is_some_and(|field| field.trim().is_empty()) {
        fields.pop();
    }

    let mut cells: Vec<i32> = fields.iter().map(|field| field.trim().parse::<i32>().unwrap_or(0)).collect();

    while cells.len() < schema.known() {
        cells.push(schema.fallback(cells.len()));
    }

    (cells, gap, comment)
}

fn shown(schema: &schema::Schema, index: usize, raw: i32) -> String {
    if raw == schema.fallback(index) {
        return String::new();
    }

    schema.to_display(index, raw).to_string()
}

fn search<'a>(query: &str, width: f32) -> Element<'a, Message> {
    let field = text_input("Search Attribute...", query)
        .on_input(Message::SearchChanged)
        .size(INPUT_SIZE)
        .padding(SEARCH_PADDING)
        .width(Length::Fixed((width * SEARCH_FRACTION).max(CARD_WIDTH)))
        .style(theme::rounded_input);

    container(field).width(Length::Fill).center_x(Length::Fill).into()
}

fn footer<'a>(schema: &schema::Schema, value: &str, armed: bool) -> Element<'a, Message> {
    let actions = column![sync(armed)].spacing(CARD_GAP).align_x(Horizontal::Center);

    if !schema.comments() {
        return actions.into();
    }

    column![comment(value), actions].spacing(CARD_GAP).align_x(Horizontal::Center).into()
}

fn comment<'a>(value: &str) -> Element<'a, Message> {
    text_input("Comment...", value)
        .on_input(Message::CommentChanged)
        .size(INPUT_SIZE)
        .padding(SEARCH_PADDING)
        .width(Length::Fill)
        .style(theme::rounded_input)
        .into()
}

fn sync<'a>(armed: bool) -> Element<'a, Message> {
    let label = if armed { CONFIRM_LABEL } else { "Sync With \"game\"" };

    let text = theme::centered_text(label)
        .size(INPUT_SIZE)
        .wrapping(iced::widget::text::Wrapping::None);

    button(text)
        .width(Length::Fixed(SYNC_WIDTH))
        .padding([SEARCH_PADDING + 1.0, 10.0])
        .style(theme::danger_button)
        .on_press(Message::Sync)
        .into()
}

fn wrapped_lines(label: &str, per_line: usize) -> usize {
    let mut lines = 1;
    let mut used = 0;

    for word in label.split_whitespace() {
        let length = word.chars().count();

        if used == 0 {
            used = length;
            continue;
        }

        if used + 1 + length <= per_line {
            used += 1 + length;
            continue;
        }

        lines += 1;
        used = length;
    }

    lines
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Changed(usize, String),
    CommentChanged(String),
    SearchChanged(String),
    Sync,
    SyncExpired,
}

#[derive(Clone)]
pub(crate) struct Plan {
    row: usize,
    label: String,
    game: PathBuf,
    target_mod: Option<String>,
    schema: &'static schema::Schema,
}

impl Plan {
    pub(super) fn subject(&self) -> schema::Subject {
        self.schema.subject()
    }

    fn matches(&self, other: &Plan) -> bool {
        self.row == other.row
            && self.game == other.game
            && self.target_mod == other.target_mod
            && self.label == other.label
    }

    fn source(&self) -> PathBuf {
        self.target_mod
            .as_deref()
            .and_then(|name| mods::find(name, &self.game))
            .unwrap_or_else(|| self.game.clone())
    }
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    query: String,
    confirm: Slot<()>,
}

struct Draft {
    plan: Plan,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: char,
    lines: Vec<String>,
    cells: Vec<i32>,
    gap: String,
    comment: String,
    inputs: Vec<String>,
    failed: bool,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan) {
        self.frame = popup::State::default();
        self.draft = Draft::load(plan);
    }

    fn reload(&mut self, plan: Plan) {
        self.draft = Draft::load(plan);
    }

    pub(super) fn drafting(&self) -> bool {
        self.draft.is_some()
    }

    pub(super) fn drifted(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| preview::stamp(&draft.read_from) != Some(draft.stamp))
    }

    pub(super) fn sync(&mut self, plan: Option<Plan>) {
        let Some(current) = self.draft.as_ref() else {
            return;
        };

        let Some(plan) = plan else {
            self.draft = None;

            return;
        };

        if current.plan.target_mod.is_none() != plan.target_mod.is_none() {
            self.draft = None;

            return;
        }

        if !current.plan.matches(&plan) {
            self.reload(plan);

            return;
        }

        if preview::stamp(&current.read_from) != Some(current.stamp) {
            self.reload(plan);
        }
    }

    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                let Some(spec) = self.draft.as_ref().map(|draft| spec(draft.plan.subject())) else {
                    return Task::none();
                };

                if self.frame.update(msg, spec) {
                    self.draft = None;
                    self.confirm.expire();
                }
            }
            Message::Changed(index, value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(index, &value);
                }
            }
            Message::SearchChanged(query) => self.query = query,
            Message::CommentChanged(value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.set_comment(value);
                }
            }
            Message::SyncExpired => self.confirm.expire(),
            Message::Sync => {
                if !self.confirm.take(&()) {
                    return self.confirm.set((), Message::SyncExpired);
                }

                if let Some(draft) = self.draft.as_mut() {
                    draft.sync();
                }
            }
        }

        Task::none()
    }

    pub(super) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        let draft = self.draft.as_ref()?;
        let spec = spec(draft.plan.subject());
        let width = self.frame.body_width(spec, window);
        let armed = self.confirm.is_set();
        let query = self.query.as_str();

        Some(self.frame.view(
            &draft.plan.label,
            spec,
            window,
            Message::Popup,
            move || draft.body(width, query, armed),
            None,
        ))
    }
}

impl Draft {
    fn load(plan: Plan) -> Option<Draft> {
        let read_from = plan.source();

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Stats editor could not read the file: {}", err))
            .ok()?;

        let Some(stamp) = preview::stamp(&read_from) else {
            warn!(path = %read_from.display(), "Stats editor could not stamp the file");

            return None;
        };

        let body = file::scrub(&bytes);
        let delimiter = file::detect_separator(&body);
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();

        let Some(raw) = lines.get(plan.row) else {
            warn!(
                path = %read_from.display(),
                row = plan.row,
                rows = lines.len(),
                "Stats editor found no row for this form"
            );

            return None;
        };

        let (cells, gap, comment) = split_row(raw, delimiter, plan.schema);
        let inputs = (0..cells.len()).map(|index| shown(plan.schema, index, cells[index])).collect();

        Some(Draft { plan, read_from, stamp, delimiter, lines, cells, gap, comment, inputs, failed: false })
    }

    fn edit(&mut self, index: usize, value: &str) {
        if !value.is_empty() && value != "-" && value.parse::<i32>().is_err() {
            return;
        }

        let Some(slot) = self.inputs.get_mut(index) else {
            return;
        };

        let raw = if value.is_empty() {
            *slot = String::new();
            self.plan.schema.fallback(index)
        } else {
            let Ok(display) = value.parse::<i32>() else {
                *slot = value.to_owned();

                return;
            };

            let raw = self.plan.schema.to_raw(index, display);
            *slot = shown(self.plan.schema, index, raw);

            raw
        };

        let Some(cell) = self.cells.get_mut(index) else {
            return;
        };

        if *cell == raw {
            return;
        }

        *cell = raw;
        self.commit();
    }

    fn set_comment(&mut self, value: String) {
        if self.comment == value {
            return;
        }

        self.comment = value;
        self.commit();
    }

    fn sync(&mut self) {
        let Ok(bytes) = fs::read(&self.plan.game) else {
            warn!(path = %self.plan.game.display(), "Stats editor could not read the vanilla file");
            self.failed = true;

            return;
        };

        let body = file::scrub(&bytes);
        let delimiter = file::detect_separator(&body);
        let vanilla: Vec<&str> = body.lines().collect();

        let Some(raw) = vanilla.get(self.plan.row) else {
            self.failed = true;

            return;
        };

        let (cells, gap, comment) = split_row(raw, delimiter, self.plan.schema);
        self.cells = cells;
        self.gap = gap;
        self.comment = comment;

        self.inputs = (0..self.cells.len()).map(|index| shown(self.plan.schema, index, self.cells[index])).collect();
        self.commit();
    }

    fn commit(&mut self) {
        let Some((path, stamp)) = self.destination() else {
            self.failed = true;

            return;
        };

        let rebuilt: Vec<String> = self.cells.iter().map(i32::to_string).collect();
        let Some(slot) = self.lines.get_mut(self.plan.row) else {
            self.failed = true;

            return;
        };

        let mut line = rebuilt.join(&self.delimiter.to_string());

        if !self.comment.is_empty() {
            line.push_str(&self.gap);
            line.push_str(COMMENT);
            line.push(' ');
            line.push_str(&self.comment);
        }

        *slot = line;

        let mut body = self.lines.join("\n");
        body.push('\n');

        match preview::save(&path, body.as_bytes(), stamp) {
            Ok(stamp) => {
                self.read_from = path;
                self.stamp = stamp;
                self.failed = false;
            }
            Err(err) => {
                warn!(path = %path.display(), "Stats editor could not write the file: {}", err);
                self.failed = true;
            }
        }
    }

    fn destination(&self) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        let path = mods::ensure(name, &self.plan.game)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Stats editor could not stage the file: {}", err))
            .ok()?;

        if path == self.read_from {
            return Some((path, self.stamp));
        }

        let stamp = preview::stamp(&path)?;

        Some((path, stamp))
    }

    fn body(&self, width: f32, query: &str, armed: bool) -> Element<'_, Message> {
        let usable = (width - BODY_PADDING * 2.0).max(CARD_WIDTH);
        let per_row = (((usable + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize).max(1);
        let needle = query.trim().to_lowercase();

        let matches: Vec<usize> = (0..self.cells.len())
            .filter(|index| needle.is_empty() || self.plan.schema.label(*index).to_lowercase().contains(&needle))
            .collect();

        let mut grid = Column::new().spacing(CARD_GAP);
        let mut line = Row::new().spacing(CARD_GAP);

        for (slot, index) in matches.into_iter().enumerate() {
            if slot > 0 && slot % per_row == 0 {
                grid = grid.push(line);
                line = Row::new().spacing(CARD_GAP);
            }

            line = line.push(self.card(index));
        }

        grid = grid.push(line);

        let centered = container(grid)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(Padding::ZERO.left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING));

        let list = smooth_scroll(scrollable(centered).width(Length::Fill).height(Length::Fill));

        column![
            container(search(query, usable))
                .padding(Padding::ZERO.top(BODY_PADDING).left(BODY_PADDING).right(BODY_PADDING).bottom(CARD_GAP)),
            list,
            container(footer(self.plan.schema, &self.comment, armed))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::ZERO.top(CARD_GAP).left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING)),
        ]
        .height(Length::Fill)
        .into()
    }

    fn card(&self, index: usize) -> Element<'_, Message> {
        let value = self.inputs.get(index).map_or("", String::as_str);
        let hint = self.plan.schema.to_display(index, self.plan.schema.fallback(index)).to_string();

        let field = text_input(&hint, value)
            .on_input(move |entry| Message::Changed(index, entry))
            .size(INPUT_SIZE)
            .padding(2)
            .align_x(Horizontal::Center)
            .style(theme::rounded_input);

        let label = container(
            text(self.plan.schema.label(index))
                .size(LABEL_SIZE)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        )
        .height(Length::Fixed(*LABEL_HEIGHT))
        .align_y(Vertical::Center);

        container(column![label, field].spacing(2))
            .width(Length::Fixed(CARD_WIDTH))
            .padding(CARD_PADDING)
            .style(theme::card_container)
            .into()
    }
}

pub(super) fn cat(row: usize, label: String, game: &Path, target_mod: Option<String>) -> Plan {
    Plan { row, label, game: game.to_path_buf(), target_mod, schema: &schema::CAT }
}

pub(super) fn enemy(row: usize, label: String, game: &Path, target_mod: Option<String>) -> Plan {
    Plan { row, label, game: game.to_path_buf(), target_mod, schema: &schema::ENEMY }
}
