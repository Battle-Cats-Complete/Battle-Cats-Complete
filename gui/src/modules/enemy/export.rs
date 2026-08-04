use std::collections::HashMap;

use arboard::Clipboard;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, text};
use iced::{Background, Border, Color, Element, Length, Task, Theme};
use tracing::error;

use core::common::context::GlobalContext;
use core::modules::enemy::game::registry::Magnification;
use core::modules::enemy::game::EnemyRenderContext;
use core::modules::enemy::scanner::{self, EnemyEntry};
use core::modules::settings::Settings;

use crate::common::feedback::Slot;
use crate::common::SpriteSheet;
use crate::modules::statblock::{builder, feedback_color, feedback_label, JobResult};

use super::statblock::build_enemy_statblock;

const BUTTON_WIDTH: f32 = 100.0;
const BUTTON_HEIGHT: f32 = 24.0;
const BUTTON_SPACING: f32 = 6.0;

pub(super) const ACTIONS_WIDTH: f32 = BUTTON_WIDTH;
pub(super) const ACTIONS_HEIGHT: f32 = BUTTON_HEIGHT * 2.0 + BUTTON_SPACING;

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

pub struct Ctx<'a> {
    pub enemy: &'a EnemyEntry,
    pub magnification: Magnification,
    pub sheets: &'a [SpriteSheet],
    pub global: GlobalContext<'a>,
    pub settings: &'a Settings,
}

#[derive(Default)]
pub struct State {
    pending: Option<ExportAction>,
    clipboard: Option<Clipboard>,
    copy_feedback: Slot<bool>,
    save_feedback: Slot<bool>,
}

impl State {
    pub fn update(&mut self, message: Message, ctx: Option<Ctx<'_>>) -> Task<Message> {
        match message {
            Message::Clicked(action) => ctx.map_or_else(Task::none, |ctx| self.start(action, ctx)),
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

    fn start(&mut self, action: ExportAction, ctx: Ctx<'_>) -> Task<Message> {
        if self.pending.is_some() {
            return Task::none();
        }

        let enemy = ctx.enemy;
        let dynamic_entry = scanner::scan_single(enemy.id, &ctx.settings.scanner_config());
        let stats = dynamic_entry.as_ref().map_or(&enemy.stats, |entry| &entry.stats);

        let enemy_ctx = EnemyRenderContext {
            global: ctx.global,
            stats,
            magnification: ctx.magnification,
        };

        let data = build_enemy_statblock(&enemy_ctx, enemy);

        let is_cat = data.is_cat;
        let id_str = data.id_str.clone();
        let top_value = data.top_value.clone();

        let mut cuts_map = HashMap::new();
        for sheet in ctx.sheets.iter().rev() {
            cuts_map.extend(sheet.core.cuts_map.clone());
        }
        let priority = ctx.settings.general.language_priority.clone();

        self.pending = Some(action);

        Task::perform(async move {
            let build_result = builder::build_statblock_image(&priority, data, cuts_map);

            match action {
                ExportAction::Copy => JobResult::Copy(build_result),
                ExportAction::Save => {
                    let result = build_result.and_then(|image| builder::save_to_disk(&image, is_cat, &id_str, &top_value).map(|_| ()));
                    if let Err(err) = &result {
                        error!("Enemy statblock save failed: {err}");
                    }
                    JobResult::Save(result)
                }
            }
        }, Message::Finished)
    }

    fn finish(&mut self, job: JobResult) -> Task<Message> {
        match job {
            JobResult::Copy(Ok(image)) => {
                let result = self
                    .ensure_clipboard()
                    .map_or_else(|| Err("Clipboard unavailable".to_string()), |clipboard| builder::copy_to_clipboard(clipboard, &image));
                if let Err(err) = &result {
                    error!("Enemy statblock copy failed: {err}");
                }
                self.copy_feedback.set(result.is_ok(), Message::CopyFeedbackExpired)
            }
            JobResult::Copy(Err(err)) => {
                error!("Enemy statblock export failed: {err}");
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

    pub fn view(&self) -> Element<'_, Message> {
        let copy_busy = self.pending == Some(ExportAction::Copy);
        let copy_feedback = self.copy_feedback.get().copied();
        let copy_label = feedback_label(copy_busy, copy_feedback, "Copy Image", "Copying...", "Copied!", "Failed!");
        let copy_btn = button(text(copy_label).size(12).align_x(Horizontal::Center).align_y(Vertical::Center))
            .width(Length::Fixed(BUTTON_WIDTH))
            .height(Length::Fixed(BUTTON_HEIGHT))
            .on_press_maybe(self.pending.is_none().then_some(Message::Clicked(ExportAction::Copy)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, copy_busy, copy_feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            });

        let save_busy = self.pending == Some(ExportAction::Save);
        let save_feedback = self.save_feedback.get().copied();
        let save_label = feedback_label(save_busy, save_feedback, "Export Image", "Exporting...", "Exported!", "Failed!");
        let save_btn = button(text(save_label).size(12).align_x(Horizontal::Center).align_y(Vertical::Center))
            .width(Length::Fixed(BUTTON_WIDTH))
            .height(Length::Fixed(BUTTON_HEIGHT))
            .on_press_maybe(self.pending.is_none().then_some(Message::Clicked(ExportAction::Save)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, save_busy, save_feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            });

        column![copy_btn, save_btn].spacing(BUTTON_SPACING).align_x(Horizontal::Center).into()
    }
}
