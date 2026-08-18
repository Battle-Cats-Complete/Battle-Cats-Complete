use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, scrollable, text_input};
use iced::{Element, Length, Padding, Size, Task};
use nyanko::common::tools::file;
use tracing::warn;

use core::common::preview::{self, Stamp};
use core::domains::mods;

use crate::app::{theme, Page};
use crate::common::feedback::{Slot, CONFIRM_LABEL};
use crate::widget::{popup, smooth_scroll};

const FIELD_HEIGHT: f32 = INPUT_SIZE * 1.3 + PADDING * 2.0;
const SYNC_HEIGHT: f32 = INPUT_SIZE * 1.3 + (PADDING + 1.0) * 2.0;

const POPUP_WIDTH: f32 = 420.0;
const NARROW_WIDTH: f32 = POPUP_WIDTH * 0.6;

const EXPLANATION_LABELS: &[&str] = &[
    "Name...",
    "Description Line 1...",
    "Description Line 2...",
    "Description Line 3...",
    "Comment...",
];

const ENEMY_NAME_LABELS: &[&str] = &["Name..."];

const ENEMY_DESCRIPTION_LABELS: &[&str] = &[
    "Description Line 1...",
    "Description Line 2...",
    "Description Line 3...",
    "Description Line 4...",
];

const INPUT_SIZE: f32 = 13.0;
const PADDING: f32 = 4.0;
const GAP: f32 = 8.0;
const BODY_PADDING: f32 = 12.0;
const SYNC_WIDTH: f32 = 172.0;

pub(super) const COUNT: usize = 3;

pub(super) const SUBJECTS: [Subject; COUNT] =
    [Subject::Explanation, Subject::EnemyName, Subject::EnemyDescription];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    Explanation,
    EnemyName,
    EnemyDescription,
}

impl Subject {
    pub(super) fn slot(self) -> usize {
        self as usize
    }

    pub(super) fn page(self) -> Page {
        match self {
            Self::Explanation => Page::Cats,
            Self::EnemyName | Self::EnemyDescription => Page::Enemies,
        }
    }

    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Explanation => EXPLANATION_LABELS,
            Self::EnemyName => ENEMY_NAME_LABELS,
            Self::EnemyDescription => ENEMY_DESCRIPTION_LABELS,
        }
    }

    fn kind(self) -> popup::Kind {
        match self {
            Self::Explanation => popup::Kind::Explanation,
            Self::EnemyName => popup::Kind::EnemyName,
            Self::EnemyDescription => popup::Kind::EnemyDescription,
        }
    }

    fn delimited(self) -> bool {
        !matches!(self, Self::EnemyName)
    }

    fn skipped(self) -> usize {
        match self {
            Self::EnemyDescription => 1,
            Self::Explanation | Self::EnemyName => 0,
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::EnemyName => NARROW_WIDTH,
            Self::Explanation | Self::EnemyDescription => POPUP_WIDTH,
        }
    }
}

fn spec(subject: Subject) -> popup::Spec {
    let fields = subject.labels().len() as f32;
    let height = FIELD_HEIGHT * fields
        + GAP * (fields - 1.0)
        + SYNC_HEIGHT
        + GAP
        + BODY_PADDING * 3.0
        + popup::CHROME_HEIGHT;

    popup::Spec::new(subject.kind(), Size::new(subject.width(), height))
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Changed(usize, String),
    Sync,
    SyncExpired,
}

#[derive(Clone)]
pub(crate) struct Plan {
    subject: Subject,
    row: usize,
    label: String,
    file: String,
    game: PathBuf,
    target_mod: Option<String>,
}

impl Plan {
    pub(super) fn subject(&self) -> Subject {
        self.subject
    }

    pub(super) fn file(&self) -> &str {
        &self.file
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
            .and_then(|name| mods::find(name, Path::new(&self.file)))
            .unwrap_or_else(|| self.game.clone())
    }
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    confirm: Slot<()>,
}

struct Draft {
    plan: Plan,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: Option<char>,
    lines: Vec<String>,
    head: Vec<String>,
    fields: Vec<String>,
    tail: Vec<String>,
    failed: bool,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan) {
        self.frame = popup::State::default();
        self.confirm.expire();
        self.draft = Draft::load(plan);
    }

    pub(super) fn close(&mut self) {
        self.draft = None;
        self.confirm.expire();
    }

    pub(super) fn drafting(&self) -> bool {
        self.draft.is_some()
    }

    pub(super) fn sync(&mut self, plans: &[Plan]) {
        let Some(current) = self.draft.as_ref() else {
            return;
        };

        let Some(plan) = plans.iter().find(|plan| plan.file == current.plan.file).or_else(|| plans.first()) else {
            self.draft = None;

            return;
        };

        if current.plan.target_mod.is_none() != plan.target_mod.is_none() {
            self.draft = None;

            return;
        }

        if !current.plan.matches(plan) || preview::stamp(&current.read_from) != Some(current.stamp) {
            self.draft = Draft::load(plan.clone());
        }
    }

    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                let Some(subject) = self.draft.as_ref().map(|draft| draft.plan.subject) else {
                    return Task::none();
                };

                if self.frame.update(msg, spec(subject)) {
                    self.close();
                }
            }
            Message::Changed(index, value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(index, value);
                }
            }
            Message::SyncExpired => self.confirm.expire(),
            Message::Sync => {
                if !self.confirm.take(&()) {
                    return self.confirm.set((), Message::SyncExpired);
                }

                if let Some(draft) = self.draft.as_mut() {
                    draft.restore();
                }
            }
        }

        Task::none()
    }

    pub(super) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        let draft = self.draft.as_ref()?;
        let armed = self.confirm.is_set();

        Some(self.frame.view(
            &draft.plan.label,
            spec(draft.plan.subject),
            window,
            Message::Popup,
            move || draft.body(armed),
            None,
        ))
    }
}

impl Draft {
    fn load(plan: Plan) -> Option<Draft> {
        let read_from = plan.source();

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Prose editor could not read the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&read_from)?;
        let body = file::scrub(&bytes);
        let delimiter = plan.subject.delimited().then(|| separator(&body));
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();
        let (head, fields, tail) = parse(&body, plan.row, delimiter, plan.subject);

        Some(Draft { plan, read_from, stamp, delimiter, lines, head, fields, tail, failed: false })
    }

    fn edit(&mut self, index: usize, value: String) {
        let Some(slot) = self.fields.get_mut(index) else {
            return;
        };

        if *slot == value {
            return;
        }

        *slot = value;
        self.commit();
    }

    fn restore(&mut self) {
        let Ok(bytes) = fs::read(&self.plan.game) else {
            warn!(path = %self.plan.game.display(), "Prose editor could not read the vanilla file");
            self.failed = true;

            return;
        };

        let body = file::scrub(&bytes);
        let delimiter = self.plan.subject.delimited().then(|| separator(&body));
        let (head, fields, tail) = parse(&body, self.plan.row, delimiter, self.plan.subject);
        self.head = head;
        self.fields = fields;
        self.tail = tail;
        self.commit();
    }

    fn destination(&self) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        let path = mods::ensure_as(name, &self.plan.game, &self.plan.file)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Prose editor could not stage the file: {}", err))
            .ok()?;

        if path == self.read_from {
            return Some((path, self.stamp));
        }

        preview::stamp(&path).map(|stamp| (path, stamp))
    }

    fn commit(&mut self) {
        let Some((path, stamp)) = self.destination() else {
            self.failed = true;

            return;
        };

        while self.lines.len() <= self.plan.row {
            self.lines.push(String::new());
        }

        let joined = join(&self.head, &self.fields, &self.tail, self.delimiter);
        let Some(slot) = self.lines.get_mut(self.plan.row) else {
            self.failed = true;

            return;
        };

        *slot = joined;

        let mut body = self.lines.join("\n");
        body.push('\n');

        match preview::save(&path, body.as_bytes(), stamp) {
            Ok(stamp) => {
                self.read_from = path;
                self.stamp = stamp;
                self.failed = false;
            }
            Err(err) => {
                warn!(path = %path.display(), "Prose editor could not write the file: {}", err);
                self.failed = true;
            }
        }
    }

    fn body(&self, armed: bool) -> Element<'_, Message> {
        let mut rows = column![].spacing(GAP);

        for (index, label) in self.plan.subject.labels().iter().enumerate() {
            let field = text_input(label, self.fields.get(index).map_or("", String::as_str))
                .on_input(move |value| Message::Changed(index, value))
                .size(INPUT_SIZE)
                .padding(PADDING)
                .width(Length::Fill)
                .style(theme::rounded_input);

            rows = rows.push(field);
        }

        let label = if armed { CONFIRM_LABEL } else { "Sync With \"game\"" };

        let sync = button(theme::centered_text(label).size(INPUT_SIZE).wrapping(iced::widget::text::Wrapping::None))
            .width(Length::Fixed(SYNC_WIDTH))
            .padding([PADDING + 1.0, 10.0])
            .style(theme::danger_button)
            .on_press(Message::Sync);

        column![
            smooth_scroll(
                scrollable(container(rows).padding(BODY_PADDING).width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ),
            container(sync)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::ZERO.top(GAP).left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING)),
        ]
        .height(Length::Fill)
        .into()
    }
}

fn separator(body: &str) -> char {
    if body.contains('|') { '|' } else { file::detect_separator(body) }
}

fn parse(body: &str, index: usize, delimiter: Option<char>, subject: Subject) -> (Vec<String>, Vec<String>, Vec<String>) {
    let skip = subject.skipped();
    let source = body.lines().nth(index).unwrap_or_default();
    let (head, fields, tail) = row(source, delimiter, skip, subject.labels().len());

    if skip == 0 || filled(&head) {
        return (head, fields, tail);
    }

    let borrowed = body
        .lines()
        .map(|line| row(line, delimiter, skip, 0).0)
        .find(|candidate| filled(candidate));

    (borrowed.unwrap_or(head), fields, tail)
}

fn filled(head: &[String]) -> bool {
    head.iter().any(|field| !field.trim().is_empty())
}

fn row(line: &str, delimiter: Option<char>, skip: usize, count: usize) -> (Vec<String>, Vec<String>, Vec<String>) {
    let Some(delimiter) = delimiter else {
        return (Vec::new(), vec![line.trim().to_owned()], Vec::new());
    };

    let mut parts = line.split(delimiter);
    let head = (0..skip).map(|_| parts.next().unwrap_or_default().to_owned()).collect();
    let fields = (0..count).map(|_| parts.next().unwrap_or_default().trim().to_owned()).collect();

    (head, fields, parts.map(str::to_owned).collect())
}

fn join(head: &[String], fields: &[String], tail: &[String], delimiter: Option<char>) -> String {
    let Some(delimiter) = delimiter else {
        return fields.first().cloned().unwrap_or_default();
    };

    let parts: Vec<&str> = head.iter().chain(fields).chain(tail).map(String::as_str).collect();

    parts.join(&delimiter.to_string())
}

pub(super) fn plan(
    subject: Subject,
    row: usize,
    label: String,
    file: String,
    game: &Path,
    target_mod: Option<String>,
) -> Plan {
    Plan { subject, row, label, file, game: game.to_path_buf(), target_mod }
}
