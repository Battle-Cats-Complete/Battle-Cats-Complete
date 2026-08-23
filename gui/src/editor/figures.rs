mod cards;
mod combat;
mod resolved;
mod schema;
mod talents;
mod unitbuy;
mod unitlevel;

pub(crate) use schema::{Subject, COUNT, FORMS, SUBJECTS};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use iced::widget::{operation, scrollable};
use iced::{Element, Size, Task};
use nyanko::common::tools::file;
use tracing::warn;

use kore::common::preview::{self, Stamp};
use kore::domains::{mods, settings::EditorValues};
use kore::Vfs;

use crate::common::feedback::Slot;
use crate::widget::popup;

use cards::CARD_WIDTH;
use resolved::{Face, Rule};

const POPUP_SIZE: Size = Size::new(364.0, 376.0);

const CAT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::CatAttributes, POPUP_SIZE);
const ENEMY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::EnemyAttributes, POPUP_SIZE);
const BUY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::UnitBuy, POPUP_SIZE);
const CURVE_POPUP: popup::Spec = popup::Spec::new(popup::Kind::LevelCurve, POPUP_SIZE);
const TALENT_SIZE: Size = Size::new(612.0, 420.0);
const TALENT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::Talents, TALENT_SIZE);

fn spec(subject: Subject) -> popup::Spec {
    match subject {
        Subject::Cat => CAT_POPUP,
        Subject::Enemy => ENEMY_POPUP,
        Subject::Buy => BUY_POPUP,
        Subject::Curve => CURVE_POPUP,
        Subject::Talents => TALENT_POPUP,
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

struct Row {
    cells: Vec<i32>,
    written: Vec<String>,
    stored: usize,
    comment: String,
}

fn split_row(line: &str, delimiter: char, schema: &schema::Schema) -> Row {
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

    let stored = fields.len();
    let mut written: Vec<String> = fields.iter().map(|field| (*field).to_owned()).collect();
    let mut cells: Vec<i32> = fields.iter().map(|field| field.trim().parse::<i32>().unwrap_or(0)).collect();

    while cells.len() < schema.known() {
        let fallback = schema.fallback(cells.len());
        written.push(fallback.to_string());
        cells.push(fallback);
    }

    Row { cells, written, stored, comment }
}

fn shown(schema: &schema::Schema, index: usize, raw: i32, values: EditorValues, rule: Rule) -> String {
    if raw == schema.fallback(index) {
        return String::new();
    }

    rule.to_display(schema.to_display(index, raw, values), values).to_string()
}

fn typable(value: &str, signed: bool) -> bool {
    let mut chars = value.chars();

    match chars.next() {
        None => true,
        Some('-') => signed && chars.all(|digit| digit.is_ascii_digit()),
        Some(first) if first.is_ascii_digit() => chars.all(|digit| digit.is_ascii_digit()),
        Some(_) => false,
    }
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
    Picked(usize, i32),
    Scrolled(f32),
    Picker(Option<usize>),
    CommentChanged(String),
    SearchChanged(String),
    Sync,
    SyncExpired,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Address {
    Line(usize),
    Keyed(u32),
}

impl Address {
    fn locate(self, lines: &[String], delimiter: char) -> Option<usize> {
        match self {
            Address::Line(row) => Some(row),
            Address::Keyed(id) => lines.iter().position(|line| leading(line, delimiter) == Some(id)),
        }
    }
}

fn leading(line: &str, delimiter: char) -> Option<u32> {
    line.split(delimiter).next()?.trim().parse().ok()
}

#[derive(Clone)]
pub(crate) struct Plan {
    address: Address,
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
        self.address == other.address
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

#[derive(Clone, Copy)]
pub(super) struct Frame<'a> {
    width: f32,
    query: &'a str,
    armed: bool,
    cap: Option<i32>,
    names: &'a talents::Names,
    vfs: &'a Vfs,
    picker: Option<usize>,
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    query: String,
    offset: f32,
    confirm: Slot<()>,
    names: talents::Names,
    picker: Option<usize>,
}

struct Draft {
    plan: Plan,
    row: usize,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: char,
    lines: Vec<String>,
    cells: Vec<i32>,
    written: Vec<String>,
    stored: usize,
    touched: usize,
    rules: Vec<Rule>,
    comment: String,
    inputs: Vec<String>,
    failed: bool,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan, nudge: usize) {
        self.frame = popup::cascaded(nudge);
        self.offset = 0.0;
        self.picker = None;
        self.draft = Draft::load(plan);
    }

    pub(super) fn restore_scroll<M: Send + 'static>(&self) -> Option<Task<M>> {
        let draft = self.draft.as_ref()?;
        let target = scrollable::AbsoluteOffset { x: 0.0, y: self.offset };

        Some(operation::scroll_to(cards::grid_id(draft.plan.subject()), target))
    }

    fn reload(&mut self, plan: Plan) {
        self.draft = Draft::load(plan);
    }

    pub(super) fn drafting(&self) -> bool {
        self.draft.is_some()
    }

    pub(super) fn raised(&self) -> u64 {
        self.frame.raised()
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
                    self.offset = 0.0;
                    self.picker = None;
                    self.confirm.expire();
                }
            }
            Message::Changed(index, value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(index, &value);
                }
            }
            Message::Picked(index, raw) => {
                self.picker = None;

                if let Some(draft) = self.draft.as_mut() {
                    draft.pick(index, raw);
                }
            }
            Message::Picker(index) => self.picker = index,
            Message::Scrolled(offset) => self.offset = offset,
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

    pub(super) fn view<'a>(
        &'a self,
        window: Size,
        cap: Option<i32>,
        vfs: &'a Vfs,
    ) -> Option<Element<'a, Message>> {
        let draft = self.draft.as_ref()?;
        let spec = spec(draft.plan.subject());
        let width = self.frame.body_width(spec, window);
        let armed = self.confirm.is_set();
        let query = self.query.as_str();
        let frame = Frame {
            width,
            query,
            armed,
            cap,
            names: &self.names,
            vfs,
            picker: self.picker,
        };

        Some(self.frame.view(
            &draft.plan.label,
            spec,
            window,
            Message::Popup,
            move || draft.body(frame),
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

        let Some(row) = plan.address.locate(&lines, delimiter) else {
            warn!(path = %read_from.display(), rows = lines.len(), "Stats editor found no row for this subject");

            return None;
        };

        let Some(raw) = lines.get(row) else {
            warn!(
                path = %read_from.display(),
                row,
                rows = lines.len(),
                "Stats editor found no row for this form"
            );

            return None;
        };

        let Row { cells, written, stored, comment } = split_row(raw, delimiter, plan.schema);
        let rules: Vec<Rule> = (0..cells.len())
            .map(|index| resolved::rule(plan.subject(), index, plan.schema.field(index)))
            .collect();
        let inputs = (0..cells.len())
            .map(|index| shown(plan.schema, index, cells[index], plan.values, rules[index]))
            .collect();

        Some(Draft {
            plan,
            row,
            read_from,
            stamp,
            delimiter,
            lines,
            cells,
            written,
            stored,
            touched: 0,
            rules,
            comment,
            inputs,
            failed: false,
        })
    }

    fn edit(&mut self, index: usize, value: &str) {
        if !typable(value, self.rule(index).signed(self.plan.values)) {
            return;
        }

        let rule = self.rule(index);
        let values = self.plan.values;

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

            let unshifted = rule.to_raw(rule.clamp(display, values), values);
            let raw = self.plan.schema.to_raw(index, unshifted, values);
            *slot = shown(self.plan.schema, index, raw, values, rule);

            raw
        };

        let Some(cell) = self.cells.get_mut(index) else {
            return;
        };

        if *cell == raw {
            return;
        }

        *cell = raw;
        self.record(index, raw);
        self.commit();
    }

    fn record(&mut self, index: usize, raw: i32) {
        if let Some(slot) = self.written.get_mut(index) {
            *slot = raw.to_string();
        }

        self.touched = self.touched.max(index + 1);
    }

    fn pick(&mut self, index: usize, raw: i32) {
        if self.cells.get(index) == Some(&raw) {
            return;
        }

        let Some(cell) = self.cells.get_mut(index) else {
            return;
        };

        *cell = raw;
        self.record(index, raw);

        let display = shown(self.plan.schema, index, raw, self.plan.values, self.rule(index));

        if let Some(slot) = self.inputs.get_mut(index) {
            *slot = display;
        }

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
        let vanilla: Vec<String> = body.lines().map(str::to_owned).collect();

        let Some(raw) = self.plan.address.locate(&vanilla, delimiter).and_then(|row| vanilla.get(row)) else {
            self.failed = true;

            return;
        };

        let Row { cells, written, stored, comment } = split_row(raw, delimiter, self.plan.schema);
        self.cells = cells;
        self.written = written;
        self.stored = stored;
        self.touched = 0;
        self.comment = comment;

        self.inputs = (0..self.cells.len())
            .map(|index| shown(self.plan.schema, index, self.cells[index], self.plan.values, self.rule(index)))
            .collect();
        self.commit();
    }

    fn commit(&mut self) {
        let Some((path, stamp)) = self.destination() else {
            self.failed = true;

            return;
        };

        let width = self.stored.max(self.touched);
        let Some(slot) = self.lines.get_mut(self.row) else {
            self.failed = true;

            return;
        };

        let mut line = self.written[..width].join(&self.delimiter.to_string());

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

    fn rule(&self, index: usize) -> Rule {
        self.rules.get(index).copied().unwrap_or(Rule::Opaque)
    }

    fn note(&self, index: usize) -> Option<&'static str> {
        resolved::note(self.plan.subject(), self.plan.schema.field(index))
    }

    fn face(&self, index: usize) -> Face {
        let rule = self.rule(index);
        let raw = self.cells.get(index).copied().unwrap_or_default();

        if let Some(gate) = rule.gate()
            && self.plan.values == EditorValues::Resolved
            && self.reads(gate.field) == Some(gate.blocked)
        {
            return Face::Disabled(gate.reason);
        }

        rule.face(raw, self.plan.values)
    }

    fn reads(&self, field: &str) -> Option<i32> {
        self.plan.schema.index_of(field).and_then(|index| self.cells.get(index).copied())
    }

    fn reads_at(&self, index: usize) -> Option<i32> {
        self.cells.get(index).copied()
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

    fn body<'a>(&'a self, frame: Frame<'a>) -> Element<'a, Message> {
        let Frame { width, query, armed, cap, names, vfs, picker } = frame;

        match self.plan.subject() {
            Subject::Cat | Subject::Enemy => combat::view(self, width, query, armed),
            Subject::Buy => unitbuy::view(self, width, query, armed),
            Subject::Curve => unitlevel::view(self, width, armed, cap),
            Subject::Talents => talents::view(self, width, query, armed, names, vfs, picker),
        }
    }
}

pub(super) fn plan(
    subject: Subject,
    address: Address,
    label: String,
    game: &Path,
    target_mod: Option<String>,
    values: EditorValues,
) -> Plan {
    Plan { address, label, game: game.to_path_buf(), target_mod, schema: schema::of(subject), values }
}

#[cfg(test)]
mod tests {
    use super::schema::{self, Subject};
    use super::{split_row, Row};

    const VANILLA: &str = "100,3,10,8,15,140,50,75,0,320,0,0,0,8,0,9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // \u{30cd}\u{30b3}";

    fn rebuild(row: &Row, touched: usize) -> String {
        let width = row.stored.max(touched);
        let mut line = row.written[..width].join(",");

        if !row.comment.is_empty() {
            line.push_str(", // ");
            line.push_str(&row.comment);
        }

        line
    }

    #[test]
    fn typing_accepts_only_digits_and_a_leading_minus() {
        for good in ["", "-", "0", "42", "-7"] {
            assert!(super::typable(good, true), "{good:?} should be typable");
        }

        for bad in ["+5", " 5", "5 ", "5a", "1.5", "--1", "1-"] {
            assert!(!super::typable(bad, true), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn an_unsigned_field_refuses_the_minus_key() {
        assert!(!super::typable("-", false), "a non-negative column must not accept a lone minus");
        assert!(!super::typable("-1", false), "a non-negative column must not accept a negative");
        assert!(super::typable("1", false), "digits stay typable");
    }

    #[test]
    fn resolved_leaves_a_value_it_cannot_represent_alone() {
        use super::resolved::{self, Face, Rule};
        use kore::domains::settings::EditorValues;

        let schema = schema::of(Subject::Cat);
        let Some(index) = schema.index_of("boss_wave_immune") else {
            panic!("nyanko no longer publishes boss_wave_immune");
        };

        let rule = resolved::rule(Subject::Cat, index, schema.field(index));
        assert_eq!(rule, Rule::Flag, "boss_wave_immune is a flag");

        assert_eq!(
            rule.face(-1, EditorValues::Resolved),
            Face::Danger,
            "-1 is not a flag state, so Resolved must fall back to the raw card",
        );

        assert_eq!(
            super::shown(schema, index, -1, EditorValues::Resolved, rule),
            "-1",
            "the unrepresentable value is shown literally, not blanked or coerced",
        );

        let mut cells = vec!["0"; schema.known()];
        cells[index] = "-1";
        let line = cells.join(",");

        let row = split_row(&line, ',', schema);
        assert_eq!(row.written[index], "-1", "the stored text is kept verbatim");
        assert_eq!(rebuild(&row, 0), line, "opening in Resolved must not rewrite the row");
    }

    #[test]
    fn an_untouched_row_is_written_back_byte_for_byte() {
        let row = split_row(VANILLA, ',', schema::of(Subject::Cat));

        assert_eq!(rebuild(&row, 0), VANILLA, "a no-op commit must not rewrite the row");
    }

    #[test]
    fn a_short_row_is_not_padded_to_the_full_column_table() {
        let row = split_row(VANILLA, ',', schema::of(Subject::Cat));

        assert_eq!(row.stored, 52, "vanilla cat 001 stores 52 columns");
        assert_eq!(row.cells.len(), schema::of(Subject::Cat).known(), "cells pad for display");
        assert_eq!(rebuild(&row, 0).matches(',').count(), VANILLA.matches(',').count());
    }

    #[test]
    fn editing_one_column_leaves_every_other_column_verbatim() {
        let mut row = split_row(VANILLA, ',', schema::of(Subject::Cat));
        row.written[3] = "999".to_owned();

        let before: Vec<&str> = VANILLA.split(" // ").next().unwrap_or_default().split(',').collect();
        let after = rebuild(&row, 4);
        let after: Vec<&str> = after.split(" // ").next().unwrap_or_default().split(',').collect();

        assert_eq!(before.len(), after.len(), "editing must not change the column count");

        for (index, (was, now)) in before.iter().zip(&after).enumerate() {
            if index == 3 {
                assert_eq!(*now, "999");
                continue;
            }

            assert_eq!(was, now, "column {index} was rewritten by an unrelated edit");
        }
    }

    #[test]
    fn editing_past_the_stored_width_pads_only_that_far() {
        let mut row = split_row(VANILLA, ',', schema::of(Subject::Cat));
        row.written[90] = "1".to_owned();

        let line = rebuild(&row, 91);
        let head = line.strip_suffix(", // \u{30cd}\u{30b3}").unwrap_or(&line);

        assert_eq!(head.split(',').count(), 91, "padding must stop at the edited column");
    }
}
