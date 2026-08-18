mod menu;
mod registry;
mod target;
mod watch;

use std::fs;
use std::path::PathBuf;

use iced::{Point, Task};
use tracing::{info, trace, warn};

use core::domains::mods;

use crate::app::{BattleCatsApp, Page};
use crate::common::feedback::Slot;

pub(crate) use target::{suppress, target};
pub(crate) use watch::watch;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    FileRow(usize),
}

pub(crate) struct Context {
    enabled: bool,
    page: Page,
    file: Option<FileTarget>,
}

struct FileTarget {
    source: PathBuf,
    name: String,
    mount: String,
    folder: bool,
    unlocked: bool,
    active_mod: Option<String>,
    in_active_mod: bool,
}

#[derive(Clone, Debug)]
pub enum Message {
    Opened(Point, Option<Target>),
    Dismissed,
    Invoked(usize),
    ConfirmExpired,
}

struct Item {
    label: String,
    hint: Option<String>,
    action: Option<Action>,
    confirm: bool,
}

impl Item {
    fn new(label: impl Into<String>, action: Action) -> Self {
        Self { label: label.into(), hint: None, action: Some(action), confirm: false }
    }

    fn disabled(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { label: label.into(), hint: Some(hint.into()), action: None, confirm: false }
    }

    fn confirming(mut self) -> Self {
        self.confirm = true;
        self
    }
}

enum Action {
    AddFileToMod { source: PathBuf, target_mod: String },
    DeleteFile { source: PathBuf },
}

impl Action {
    fn run(&self) {
        match self {
            Self::AddFileToMod { source, target_mod } => match mods::adopt(target_mod, source) {
                Ok(path) => info!(path = %path.display(), "Added a file to a mod"),
                Err(err) => warn!(source = %source.display(), "Failed to add the file to the mod: {}", err),
            },
            Self::DeleteFile { source } => match fs::remove_file(source) {
                Ok(()) => info!(path = %source.display(), "Deleted a mod file"),
                Err(err) => warn!(path = %source.display(), "Failed to delete the file: {}", err),
            },
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    open: Option<Open>,
    confirm: Slot<usize>,
}

struct Open {
    at: Point,
    items: Vec<Item>,
}

impl State {
    pub(crate) fn open(&mut self, at: Point, context: &Context) {
        let items = registry::items(context);

        if items.is_empty() {
            trace!(
                page = ?context.page,
                targeted = context.file.is_some(),
                nightly = context.enabled,
                "Right click produced no context menu actions"
            );
        }

        self.confirm.expire();
        self.open = (!items.is_empty()).then_some(Open { at, items });
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Invoked(index) => self.invoke(index),
            Message::Dismissed => {
                self.open = None;
                self.confirm.expire();
                Task::none()
            }
            Message::ConfirmExpired => {
                self.confirm.expire();
                Task::none()
            }
            Message::Opened(..) => Task::none(),
        }
    }

    fn invoke(&mut self, index: usize) -> Task<Message> {
        let Some((actionable, confirms)) = self
            .open
            .as_ref()
            .and_then(|open| open.items.get(index))
            .map(|item| (item.action.is_some(), item.confirm))
        else {
            return Task::none();
        };

        if !actionable {
            return Task::none();
        }

        if confirms && !self.confirm.armed_for(&index) {
            return self.confirm.set(index, Message::ConfirmExpired);
        }

        self.confirm.expire();

        let Some(open) = self.open.take() else {
            return Task::none();
        };

        if let Some(action) = open.items.get(index).and_then(|item| item.action.as_ref()) {
            action.run();
        }

        Task::none()
    }
}

pub(crate) fn context(app: &BattleCatsApp, target: Option<Target>) -> Context {
    Context {
        enabled: app.settings.general.enable_nightly,
        page: app.current_page,
        file: file_target(app, target),
    }
}

fn file_target(app: &BattleCatsApp, target: Option<Target>) -> Option<FileTarget> {
    let Some(Target::FileRow(index)) = target else {
        return None;
    };

    let mount = app.files_state.mount()?;
    let (folder, relative) = app.files_state.entry_at(&app.vault.vfs, index)?;
    let source = app.vault.vfs.root(mount)?.join(&relative);
    let name = relative.file_name()?.to_string_lossy().into_owned();
    let active_mod = app.mods_state.active_mod();

    let in_active_mod = active_mod
        .as_deref()
        .is_some_and(|active| app.vault.vfs.locate_in(active, &name).is_some());

    Some(FileTarget {
        source,
        name,
        mount: mount.to_owned(),
        folder,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod,
        in_active_mod,
    })
}
