use iced::widget::{button, column, container, rule, scrollable, text, Space};
use iced::{Alignment, Element, Length, Size};

use core::Conflict;

use crate::widget::{popup, smooth_scroll};

use super::{theme, Message};

const POPUP_TITLE: &str = "Initialization Errors Found";
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

const WATCHER_INTRO: &str = "Failed to initialize file Watcher due to complex directory structure. \
File Watcher has been disabled for this session. Please simplify your directory structure.";

#[derive(Default)]
pub(super) struct State {
    popup: popup::State,
    conflicts: Vec<Conflict>,
    watcher_failed: bool,
}

impl State {
    pub(super) fn report_conflicts(&mut self, conflicts: Vec<Conflict>, ignored: bool) {
        if ignored || conflicts.is_empty() {
            return;
        }

        self.conflicts = conflicts;
    }

    pub(super) fn report_watcher_failure(&mut self, ignored: bool) {
        self.watcher_failed = !ignored;
    }

    pub(super) fn is_open(&self) -> bool {
        !self.conflicts.is_empty() || self.watcher_failed
    }

    pub(super) fn acknowledge(&mut self) {
        if !self.conflicts.is_empty() {
            self.conflicts.clear();
            return;
        }

        self.watcher_failed = false;
    }

    pub(super) fn update(&mut self, message: popup::Message) {
        if self.popup.update(message, popup::Kind::InitErrors) {
            self.acknowledge();
        }
    }

    pub(super) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        if !self.is_open() {
            return None;
        }

        Some(self.popup.view(POPUP_TITLE, popup::Kind::InitErrors, window, Message::InitErrorPopup, move || {
            let body = if self.conflicts.is_empty() { watcher_body() } else { self.conflict_body() };

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

fn watcher_body<'a>() -> Element<'a, Message> {
    container(text(WATCHER_INTRO).size(BODY_SIZE))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .into()
}
