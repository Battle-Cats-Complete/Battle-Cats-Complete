use std::collections::HashMap;

use arboard::Clipboard;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, text};
use iced::{Background, Border, Color, Element, Length, Task, Theme};
use tracing::error;

use core::common::context::GlobalContext;
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::waiter::unitid;
use core::modules::settings::Settings;

use crate::common::feedback::Slot;
use crate::common::SpriteSheet;
use crate::modules::statblock::{builder, feedback_color, feedback_label, JobResult};

use super::statblock::build_cat_statblock;

const BUTTON_WIDTH: f32 = 100.0;
const BUTTON_HEIGHT: f32 = 24.0;

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
    pub cat: &'a CatEntry,
    pub form: usize,
    pub current_level: i32,
    pub level_input: &'a str,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub is_conjure_expanded: bool,
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

        let cat = ctx.cat;
        let dynamic_stats = unitid(cat.id as i32, &ctx.settings.general.language_priority);
        let Some(base_stats) = dynamic_stats.as_ref().and_then(|v| v.get(ctx.form)) else { return Task::none(); };

        let form_allows_talents = ctx.form >= 2;
        let talent_data = if form_allows_talents { cat.talent_data.as_ref() } else { None };
        let talent_levels = if form_allows_talents { ctx.talent_levels } else { None };
        let final_stats = get_final_stats(base_stats, cat.curve.as_ref(), ctx.current_level, talent_data, talent_levels);

        let cat_ctx = CatRenderContext {
            global: ctx.global,
            base_stats,
            final_stats: &final_stats,
            current_level: ctx.current_level,
            level_curve: cat.curve.as_ref(),
            talent_data,
            talent_levels,
            is_conjure_unit: false,
        };

        let data = build_cat_statblock(&cat_ctx, cat, ctx.form, ctx.level_input.to_string(), ctx.is_conjure_expanded, ctx.settings);

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
                        error!("Cat statblock save failed: {err}");
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
                    error!("Cat statblock copy failed: {err}");
                }
                self.copy_feedback.set(result.is_ok(), Message::CopyFeedbackExpired)
            }
            JobResult::Copy(Err(err)) => {
                error!("Cat statblock export failed: {err}");
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

        column![copy_btn, save_btn].spacing(6).align_x(Horizontal::Center).into()
    }
}
