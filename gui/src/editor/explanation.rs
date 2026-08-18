use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, scrollable, text_input};
use iced::{Element, Length, Padding, Size, Task};
use nyanko::common::tools::file;
use tracing::warn;

use core::common::preview::{self, Stamp};
use core::domains::mods;

use crate::app::theme;
use crate::common::feedback::{Slot, CONFIRM_LABEL};
use crate::widget::{popup, smooth_scroll};

const FIELD_HEIGHT: f32 = INPUT_SIZE * 1.3 + PADDING * 2.0;
const SYNC_HEIGHT: f32 = INPUT_SIZE * 1.3 + (PADDING + 1.0) * 2.0;

const POPUP_HEIGHT: f32 = FIELD_HEIGHT * FIELDS as f32
    + GAP * (FIELDS - 1) as f32
    + SYNC_HEIGHT
    + GAP
    + BODY_PADDING * 3.0
    + popup::CHROME_HEIGHT;

const POPUP: popup::Spec = popup::Spec::new(popup::Kind::Explanation, Size::new(420.0, POPUP_HEIGHT));

const FIELDS: usize = 5;
const LABELS: [&str; FIELDS] = [
    "Name...",
    "Description Line 1...",
    "Description Line 2...",
    "Description Line 3...",
    "Comment...",
];
const INPUT_SIZE: f32 = 13.0;
const PADDING: f32 = 4.0;
const GAP: f32 = 8.0;
const BODY_PADDING: f32 = 12.0;
const SYNC_WIDTH: f32 = 172.0;

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Changed(usize, String),
    Sync,
    SyncExpired,
}

#[derive(Clone)]
pub(crate) struct Plan {
    row: usize,
    label: String,
    file: String,
    game: PathBuf,
    target_mod: Option<String>,
}

impl Plan {
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
pub(crate) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    confirm: Slot<()>,
}

struct Draft {
    plan: Plan,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: char,
    lines: Vec<String>,
    fields: [String; FIELDS],
    tail: Vec<String>,
    failed: bool,
}

impl State {
    pub(crate) fn begin(&mut self, plan: Plan) {
        self.frame = popup::State::default();
        self.confirm.expire();
        self.draft = Draft::load(plan);
    }

    pub(crate) fn sync(&mut self, plan: Option<Plan>) {
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

        if !current.plan.matches(&plan) || preview::stamp(&current.read_from) != Some(current.stamp) {
            self.draft = Draft::load(plan);
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                if self.draft.is_some() && self.frame.update(msg, POPUP) {
                    self.draft = None;
                    self.confirm.expire();
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

    pub(crate) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        let draft = self.draft.as_ref()?;
        let armed = self.confirm.is_set();

        Some(self.frame.view(&draft.plan.label, POPUP, window, Message::Popup, move || draft.body(armed), None))
    }
}

impl Draft {
    fn load(plan: Plan) -> Option<Draft> {
        let read_from = plan.source();

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Explanation editor could not read the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&read_from)?;
        let body = file::scrub(&bytes);
        let delimiter = separator(&body);
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();
        let (fields, tail) = row(lines.get(plan.row).map_or("", String::as_str), delimiter);

        Some(Draft { plan, read_from, stamp, delimiter, lines, fields, tail, failed: false })
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
            warn!(path = %self.plan.game.display(), "Explanation editor could not read the vanilla file");
            self.failed = true;

            return;
        };

        let body = file::scrub(&bytes);
        let delimiter = separator(&body);
        let vanilla: Vec<&str> = body.lines().collect();

        let (fields, tail) = row(vanilla.get(self.plan.row).copied().unwrap_or_default(), delimiter);
        self.fields = fields;
        self.tail = tail;
        self.commit();
    }

    fn destination(&self) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        let path = mods::ensure(name, &self.plan.game)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Explanation editor could not stage the file: {}", err))
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

        let joined = self
            .fields
            .iter()
            .chain(self.tail.iter())
            .cloned()
            .collect::<Vec<String>>()
            .join(&self.delimiter.to_string());
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
                warn!(path = %path.display(), "Explanation editor could not write the file: {}", err);
                self.failed = true;
            }
        }
    }

    fn body(&self, armed: bool) -> Element<'_, Message> {
        let mut rows = column![].spacing(GAP);

        for (index, label) in LABELS.iter().enumerate() {
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

fn row(line: &str, delimiter: char) -> ([String; FIELDS], Vec<String>) {
    let mut parts = line.split(delimiter);
    let fields = std::array::from_fn(|_| parts.next().unwrap_or_default().trim().to_owned());

    (fields, parts.map(str::to_owned).collect())
}

pub(crate) fn plan(row: usize, label: String, file: String, game: &Path, target_mod: Option<String>) -> Plan {
    Plan { row, label, file, game: game.to_path_buf(), target_mod }
}
