use std::collections::HashMap;

use arboard::Clipboard;
use iced::alignment::Horizontal;
use iced::widget::{button, column};
use iced::{Background, Border, Color, Element, Length, Task, Theme};
use tracing::{error, warn};

use core::modules::settings::Settings;
use core::Vfs;

use crate::app::theme;
use crate::common::SpriteSheet;
use crate::modules::statblock::builder::{self, StatblockData};
use crate::modules::statblock::JobResult;

use crate::common::feedback::Slot;

const BUTTON_WIDTH: f32 = 100.0;
const BUTTON_HEIGHT: f32 = 24.0;
const BUTTON_SPACING: f32 = 6.0;
const BUTTON_TEXT_SIZE: f32 = 12.0;
const BUTTON_RADIUS: f32 = 4.0;

pub(crate) const ACTIONS_WIDTH: f32 = BUTTON_WIDTH;
pub(crate) const ACTIONS_HEIGHT: f32 = BUTTON_HEIGHT * 2.0 + BUTTON_SPACING;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportAction {
    Copy,
    Save,
}

#[derive(Clone)]
pub enum Message {
    Clicked(ExportAction),
    Finished(JobResult),
    CopyFeedbackExpired,
    SaveFeedbackExpired,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clicked(action) => write!(f, "Clicked({:?})", action),
            Self::Finished(_) => write!(f, "Finished"),
            Self::CopyFeedbackExpired => write!(f, "CopyFeedbackExpired"),
            Self::SaveFeedbackExpired => write!(f, "SaveFeedbackExpired"),
        }
    }
}

pub(crate) struct Request<'a> {
    pub(crate) data: StatblockData,
    pub(crate) sheets: &'a [SpriteSheet],
    pub(crate) settings: &'a Settings,
    pub(crate) vfs: &'a Vfs,
}

pub(crate) struct State {
    kind: &'static str,
    pending: Option<ExportAction>,
    clipboard: Option<Clipboard>,
    copy_feedback: Slot<bool>,
    save_feedback: Slot<bool>,
}

impl State {
    pub(crate) fn new(kind: &'static str) -> Self {
        Self {
            kind,
            pending: None,
            clipboard: None,
            copy_feedback: Slot::default(),
            save_feedback: Slot::default(),
        }
    }

    pub(crate) fn update<'a>(&mut self, message: Message, request: impl FnOnce() -> Option<Request<'a>>) -> Task<Message> {
        match message {
            Message::Clicked(action) => request().map_or_else(Task::none, |request| self.start(action, request)),
            Message::Finished(job) => {
                self.pending = None;
                self.finish(job)
            }
            Message::CopyFeedbackExpired => {
                self.copy_feedback.expire();
                Task::none()
            }
            Message::SaveFeedbackExpired => {
                self.save_feedback.expire();
                Task::none()
            }
        }
    }

    fn start(&mut self, action: ExportAction, request: Request<'_>) -> Task<Message> {
        if self.pending.is_some() {
            return Task::none();
        }

        if request.sheets.iter().any(|sheet| !sheet.is_settled()) {
            warn!("{} statblock export blocked, sprite sheets are still loading", self.kind);
            return self.reject(action);
        }

        let data = request.data;
        let is_cat = data.is_cat;
        let id_str = data.id_str.clone();
        let top_value = data.top_value.clone();

        let mut cuts_map = HashMap::new();
        for sheet in request.sheets.iter().rev() {
            cuts_map.extend(sheet.core.cuts_map.clone());
        }
        let priority = request.settings.general.language_priority.clone();
        let sheet_path = request.vfs.find("img015.png");
        let kind = self.kind;

        self.pending = Some(action);

        Task::perform(async move {
            let build_result = builder::build_statblock_image(&priority, sheet_path.as_deref(), data, cuts_map);

            match action {
                ExportAction::Copy => JobResult::Copy(build_result),
                ExportAction::Save => {
                    let result = build_result.and_then(|image| builder::save_to_disk(&image, is_cat, &id_str, &top_value).map(|_| ()));
                    if let Err(err) = &result {
                        error!("{kind} statblock save failed: {err}");
                    }
                    JobResult::Save(result)
                }
            }
        }, Message::Finished)
    }

    fn reject(&mut self, action: ExportAction) -> Task<Message> {
        match action {
            ExportAction::Copy => self.copy_feedback.set(false, Message::CopyFeedbackExpired),
            ExportAction::Save => self.save_feedback.set(false, Message::SaveFeedbackExpired),
        }
    }

    fn finish(&mut self, job: JobResult) -> Task<Message> {
        let kind = self.kind;

        match job {
            JobResult::Copy(Ok(image)) => {
                let result = self
                    .ensure_clipboard()
                    .map_or_else(|| Err("Clipboard unavailable".to_string()), |clipboard| builder::copy_to_clipboard(clipboard, &image));
                if let Err(err) = &result {
                    error!("{kind} statblock copy failed: {err}");
                }
                self.copy_feedback.set(result.is_ok(), Message::CopyFeedbackExpired)
            }
            JobResult::Copy(Err(err)) => {
                error!("{kind} statblock export failed: {err}");
                self.copy_feedback.set(false, Message::CopyFeedbackExpired)
            }
            JobResult::Save(result) => {
                self.save_feedback.set(result.is_ok(), Message::SaveFeedbackExpired)
            }
        }
    }

    fn ensure_clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.clipboard.is_none() {
            match Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(err) => error!("Failed to open system clipboard: {err}"),
            }
        }
        self.clipboard.as_mut()
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let copy_busy = self.pending == Some(ExportAction::Copy);
        let copy_feedback = self.copy_feedback.get().copied();
        let copy_label = feedback_label(copy_busy, copy_feedback, "Copy Image", "Copying...", "Copied!");

        let save_busy = self.pending == Some(ExportAction::Save);
        let save_feedback = self.save_feedback.get().copied();
        let save_label = feedback_label(save_busy, save_feedback, "Export Image", "Exporting...", "Exported!");

        column![
            self.action_button(copy_label, copy_busy, copy_feedback, ExportAction::Copy),
            self.action_button(save_label, save_busy, save_feedback, ExportAction::Save),
        ]
            .spacing(BUTTON_SPACING)
            .align_x(Horizontal::Center)
            .into()
    }

    fn action_button(&self, label: String, busy: bool, feedback: Option<bool>, action: ExportAction) -> Element<'_, Message> {
        button(
            theme::centered_text(label)
                .size(BUTTON_TEXT_SIZE)
                .width(Length::Fill)
                .height(Length::Fill)
        )
            .width(Length::Fixed(BUTTON_WIDTH))
            .height(Length::Fixed(BUTTON_HEIGHT))
            .padding(0)
            .on_press_maybe(self.pending.is_none().then_some(Message::Clicked(action)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, busy, feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(BUTTON_RADIUS),
                ..Default::default()
            })
            .into()
    }
}

fn feedback_label(busy: bool, feedback: Option<bool>, idle: &str, busy_label: &str, success: &str) -> String {
    if busy {
        return busy_label.to_string();
    }

    feedback.map_or_else(|| idle.to_string(), |ok| if ok { success.to_string() } else { "Failed!".to_string() })
}

fn feedback_color(theme: &Theme, busy: bool, feedback: Option<bool>) -> Color {
    let palette = theme.palette();

    if busy {
        return palette.warning;
    }

    feedback.map_or_else(|| palette.primary, |ok| if ok { palette.success } else { palette.danger })
}
