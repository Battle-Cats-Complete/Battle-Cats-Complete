use iced::widget::{button, column, container, rule, scrollable, text, Space};
use iced::{Alignment, Element, Length, Size};

use std::path::{Path, PathBuf};

use kore::Conflict;

use crate::common::watcher::Lapse;
use crate::widget::{popup, smooth_scroll};

use super::{theme, Message};

const POPUP_TITLE: &str = "Initialization Errors Found";
const POPUP: popup::Spec = popup::Spec::new(popup::Kind::InitErrors, Size::new(560.0, 420.0));
const SCROLLBAR_GAP: f32 = 8.0;
const BODY_PADDING: f32 = 20.0;
const BODY_SPACING: f32 = 14.0;
const ENTRY_SPACING: f32 = 16.0;
const RULE_GAP: f32 = 6.0;
const PATH_SPACING: f32 = 2.0;
const BODY_SIZE: f32 = 14.0;
const KEY_SIZE: f32 = 15.0;
const PATH_SIZE: f32 = 13.0;
const ACKNOWLEDGE_SIZE: f32 = 16.0;

const CONFLICT_INTRO: &str = "Found conflicting file names in the following directories. \
The files listed below were not loaded into the Virtual File System and won't be read by the app this session.";

const WATCHER_CROWDED: &str = "Failed to start the File Watcher because this system has no free watch \
slots left. Live reload has been disabled for this session.";

const WATCHER_CROWDED_CAUSE: &str = "Close other watching applications or Battle Cats Complete instances and restart, or raise the system's watch limit.";

const WATCHER_BROKEN: &str = "Failed to start the File Watcher. Live reload has been disabled for this \
session, so edits made to game/ or mods/ outside the app will not be picked up until you restart it.";

const VOLATILE_INTRO: &str = "Battle Cats Complete is running from a temporary folder. This usually means \
the program was opened from inside its .zip instead of being extracted first.";

const VOLATILE_WARNING: &str = "Anything you import or edit will be stored here, and Windows deletes this \
folder without warning. Close the app, extract the .zip to a real folder such as Documents or Desktop, \
and run it from there.";

#[derive(Default)]
pub(super) struct State {
    popup: popup::State,
    conflicts: Vec<Conflict>,
    watcher_failed: Option<Lapse>,
    watcher_shown: bool,
    volatile: Option<PathBuf>,
}

impl State {
    pub(super) fn report_volatile(&mut self, home: &Path) {
        self.volatile = Some(home.to_path_buf());
    }

    pub(super) fn report_conflicts(&mut self, conflicts: Vec<Conflict>, ignored: bool) {
        self.conflicts = if ignored { Vec::new() } else { conflicts };
    }

    pub(super) fn report_watcher_failure(&mut self, lapse: Lapse, ignored: bool) {
        self.watcher_failed = Some(lapse);
        self.watcher_shown = !ignored;
    }

    pub(super) fn watcher_failed(&self) -> bool {
        self.watcher_failed.is_some()
    }

    pub(super) fn refresh_watcher(&mut self, ignored: bool) {
        self.watcher_shown = self.watcher_failed.is_some() && !ignored;
    }

    pub(super) fn is_open(&self) -> bool {
        self.volatile.is_some() || !self.conflicts.is_empty() || self.watcher_shown
    }

    pub(super) fn acknowledge(&mut self) {
        if self.volatile.take().is_some() {
            return;
        }

        if !self.conflicts.is_empty() {
            self.conflicts.clear();
            return;
        }

        self.watcher_shown = false;
    }

    pub(super) fn update(&mut self, message: popup::Message) {
        if self.popup.update(message, POPUP) {
            self.acknowledge();
        }
    }

    pub(super) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        if !self.is_open() {
            return None;
        }

        Some(self.popup.view(POPUP_TITLE, POPUP, window, Message::InitErrorPopup, move || {
            let body = match (self.volatile.as_deref(), self.conflicts.is_empty()) {
                (Some(home), _) => volatile_body(home),
                (None, true) => watcher_body(self.watcher_failed),
                (None, false) => self.conflict_body(),
            };

            column![
                body,
                Space::new().height(BODY_SPACING),
                button(text("Acknowledge").size(ACKNOWLEDGE_SIZE))
                    .style(button::primary)
                    .on_press(Message::AcknowledgeInitError),
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .into()
        }, None))
    }

    fn conflict_body(&self) -> Element<'_, Message> {
        let mut entries = column![].spacing(ENTRY_SPACING).width(Length::Fill);

        for conflict in &self.conflicts {
            let mut entry = column![theme::bold_text(conflict.key.to_string()).size(KEY_SIZE)].spacing(PATH_SPACING);

            for path in &conflict.paths {
                entry = entry.push(text(path.display().to_string()).size(PATH_SIZE));
            }

            entries = entries.push(entry);
        }

        let area = column![
            rule::horizontal(1),
            smooth_scroll(
                scrollable(entries)
                    .height(Length::Fill)
                    .spacing(SCROLLBAR_GAP)
            ),
        ]
        .spacing(RULE_GAP)
        .width(Length::Fill)
        .height(Length::Fill);

        column![text(CONFLICT_INTRO).size(BODY_SIZE), area]
            .spacing(BODY_SPACING)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn volatile_body(home: &Path) -> Element<'_, Message> {
    container(
        column![
            text(VOLATILE_INTRO).size(BODY_SIZE),
            theme::bold_text(home.display().to_string()).size(PATH_SIZE),
            text(VOLATILE_WARNING).size(BODY_SIZE),
        ]
        .spacing(BODY_SPACING),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}

fn watcher_body<'a>(lapse: Option<Lapse>) -> Element<'a, Message> {
    let body = match lapse {
        Some(Lapse::Crowded) => column![
            text(WATCHER_CROWDED).size(BODY_SIZE),
            text(WATCHER_CROWDED_CAUSE).size(BODY_SIZE),
        ]
        .spacing(BODY_SPACING)
        .into(),
        _ => Element::from(text(WATCHER_BROKEN).size(BODY_SIZE)),
    };

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .into()
}
