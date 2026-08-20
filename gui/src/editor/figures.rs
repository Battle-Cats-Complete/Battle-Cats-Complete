mod cards;
mod combat;
mod schema;
mod unitbuy;
mod unitlevel;

pub(crate) use schema::{Subject, COUNT, FORMS, SUBJECTS};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use iced::{Element, Size, Task};
use nyanko::common::tools::file;
use tracing::warn;

use kore::common::preview::{self, Stamp};
use kore::domains::{mods, settings::EditorValues};

use crate::common::feedback::Slot;
use crate::widget::popup;

use cards::CARD_WIDTH;

const POPUP_SIZE: Size = Size::new(364.0, 376.0);

const CAT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::CatAttributes, POPUP_SIZE);
const ENEMY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::EnemyAttributes, POPUP_SIZE);
const BUY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::UnitBuy, POPUP_SIZE);
const CURVE_POPUP: popup::Spec = popup::Spec::new(popup::Kind::LevelCurve, POPUP_SIZE);

fn spec(subject: Subject) -> popup::Spec {
    match subject {
        Subject::Cat => CAT_POPUP,
        Subject::Enemy => ENEMY_POPUP,
        Subject::Buy => BUY_POPUP,
        Subject::Curve => CURVE_POPUP,
    }
}

const LABEL_SIZE: f32 = 12.0;
const CARD_PADDING: f32 = 6.0;
const COMMENT: &str = "//";
const LINE_RATIO: f32 = 1.3;
const GLYPH_RATIO: f32 = 0.55;

static LABEL_HEIGHTS: LazyLock<[f32; COUNT]> =
    LazyLock::new(|| SUBJECTS.map(|subject| label_height(schema::of(subject))));

fn label_height(schema: &schema::Schema) -> f32 {
    let inner = CARD_WIDTH - CARD_PADDING * 2.0;
    let per_line = ((inner / (LABEL_SIZE * GLYPH_RATIO)).floor() as usize).max(1);

    let lines = (0..schema.known())
        .map(|index| wrapped_lines(&schema.label(index), per_line))
        .max()
        .unwrap_or(1);

    lines as f32 * LABEL_SIZE * LINE_RATIO
}

fn split_row(line: &str, delimiter: char, schema: &schema::Schema) -> (Vec<i32>, String) {
    let (numeric, comment) = match line.find(COMMENT) {
        Some(at) => {
            let (head, tail) = line.split_at(at);

            (head.trim_end(), tail[COMMENT.len()..].trim().to_owned())
        }
        None => (line, String::new()),
    };

    let mut fields: Vec<&str> = numeric.split(delimiter).collect();

    while fields.last().is_some_and(|field| field.trim().is_empty()) {
        fields.pop();
    }

    let mut cells: Vec<i32> = fields.iter().map(|field| field.trim().parse::<i32>().unwrap_or(0)).collect();

    while cells.len() < schema.known() {
        cells.push(schema.fallback(cells.len()));
    }

    (cells, comment)
}

fn shown(schema: &schema::Schema, index: usize, raw: i32, values: EditorValues) -> String {
    if raw == schema.fallback(index) {
        return String::new();
    }

    schema.to_display(index, raw, values).to_string()
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
    values: EditorValues,
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
            && self.values == other.values
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
    comment: String,
    inputs: Vec<String>,
    failed: bool,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan, nudge: usize) {
        self.frame = popup::cascaded(nudge);
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

    pub(super) fn view(&self, window: Size, cap: Option<i32>) -> Option<Element<'_, Message>> {
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
            move || draft.body(width, query, armed, cap),
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

        let (cells, comment) = split_row(raw, delimiter, plan.schema);
        let inputs =
            (0..cells.len()).map(|index| shown(plan.schema, index, cells[index], plan.values)).collect();

        Some(Draft { plan, read_from, stamp, delimiter, lines, cells, comment, inputs, failed: false })
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

            let raw = self.plan.schema.to_raw(index, display, self.plan.values);
            *slot = shown(self.plan.schema, index, raw, self.plan.values);

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

        let (cells, comment) = split_row(raw, delimiter, self.plan.schema);
        self.cells = cells;
        self.comment = comment;

        self.inputs = (0..self.cells.len())
            .map(|index| shown(self.plan.schema, index, self.cells[index], self.plan.values))
            .collect();
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
            line.push(self.delimiter);
            line.push(' ');
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

    fn schema(&self) -> &'static schema::Schema {
        self.plan.schema
    }

    fn values(&self) -> EditorValues {
        self.plan.values
    }

    fn len(&self) -> usize {
        self.cells.len()
    }

    fn input(&self, index: usize) -> &str {
        self.inputs.get(index).map_or("", String::as_str)
    }

    fn comment(&self) -> &str {
        &self.comment
    }

    fn body<'a>(&'a self, width: f32, query: &'a str, armed: bool, cap: Option<i32>) -> Element<'a, Message> {
        match self.plan.subject() {
            Subject::Cat | Subject::Enemy => combat::view(self, width, query, armed),
            Subject::Buy => unitbuy::view(self, width, query, armed),
            Subject::Curve => unitlevel::view(self, width, armed, cap),
        }
    }
}

pub(super) fn plan(
    subject: Subject,
    row: usize,
    label: String,
    game: &Path,
    target_mod: Option<String>,
    values: EditorValues,
) -> Plan {
    Plan { row, label, game: game.to_path_buf(), target_mod, schema: schema::of(subject), values }
}
