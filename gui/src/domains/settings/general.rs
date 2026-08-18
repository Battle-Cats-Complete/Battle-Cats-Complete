use iced::alignment::Vertical;
use iced::border::Radius;
use iced::mouse::Interaction;
use iced::widget::{column, container, mouse_area, pick_list, row, text};
use iced::{Alignment, Border, Color, Element, Length, Point, Task, Theme};
use tracing::debug;

use core::domains::settings::{lang, nightly};
use core::domains::settings::{Settings as CoreSettings, UpdateMode};

use crate::app::theme;
use crate::app::UpdateStatus;
use crate::common::fonts;
use crate::widget::toggle_row;
#[cfg(target_os = "linux")]
use crate::common::feedback::Slot;

use super::{header_section, hover_hint, SECTION_SPACING};

const ROW_HEIGHT: f32 = 32.0;
const ROW_WIDTH: f32 = 140.0;
const DRAG_HIGHLIGHT_ALPHA: f32 = 0.25;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopFeedback {
    Created,
    Deleted,
    Failed,
}

#[derive(Default, Clone, Copy)]
enum Drag {
    #[default]
    Idle,
    Pressed {
        index: usize,
    },
    Moving {
        index: usize,
        last: Point,
        carry: f32,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleLogging(bool),
    ToggleNightly(bool),
    ToggleIgnoreConflicts(bool),
    ToggleIgnoreWatcherFailure(bool),
    UpdateModeSelected(UpdateMode),
    LanguageDragStart(usize),
    LanguageDragMove(Point),
    LanguageDragEnd,
    LanguageSetDefault,
    #[cfg(target_os = "linux")]
    ToggleDesktopData,
    #[cfg(target_os = "linux")]
    DesktopFeedbackExpired,
    ManualUpdateCheck,
    ShowUpdatePopup,
}

#[derive(Default)]
pub struct State {
    drag: Drag,
    baseline: Vec<String>,
    settled: bool,

    #[cfg(target_os = "linux")]
    desktop_feedback: Slot<DesktopFeedback>,
}

impl State {
    pub fn update(&mut self, message: Message, core_settings: &mut CoreSettings) -> Task<Message> {
        match message {
            Message::ToggleLogging(enabled) => {
                core_settings.general.enable_logging = enabled;
                Task::none()
            }
            Message::ToggleNightly(enabled) => {
                core_settings.general.enable_nightly = enabled;
                Task::none()
            }
            Message::ToggleIgnoreConflicts(enabled) => {
                core_settings.general.ignore_conflict_errors = enabled;
                Task::none()
            }
            Message::ToggleIgnoreWatcherFailure(enabled) => {
                core_settings.general.ignore_watcher_failure = enabled;
                Task::none()
            }
            Message::UpdateModeSelected(mode) => {
                core_settings.general.update_mode = mode;
                Task::none()
            }
            Message::LanguageDragStart(index) => {
                self.drag = Drag::Pressed { index };
                self.baseline = core_settings.general.language_priority.clone();
                Task::none()
            }
            Message::LanguageDragMove(point) => {
                match self.drag {
                    Drag::Idle => {}
                    Drag::Pressed { index } => {
                        self.drag = Drag::Moving { index, last: point, carry: 0.0 };
                    }
                    Drag::Moving { index, last, carry } => {
                        let priority = &mut core_settings.general.language_priority;
                        let max_index = priority.len().saturating_sub(1);

                        let mut new_index = index;
                        let mut new_carry = carry + (point.y - last.y);

                        while new_carry >= ROW_HEIGHT && new_index < max_index {
                            priority.swap(new_index, new_index + 1);
                            new_index += 1;
                            new_carry -= ROW_HEIGHT;
                        }
                        while new_carry <= -ROW_HEIGHT && new_index > 0 {
                            priority.swap(new_index, new_index - 1);
                            new_index -= 1;
                            new_carry += ROW_HEIGHT;
                        }

                        if (new_index == max_index && new_carry > 0.0) || (new_index == 0 && new_carry < 0.0) {
                            new_carry = 0.0;
                        }

                        self.drag = Drag::Moving { index: new_index, last: point, carry: new_carry };
                    }
                }
                Task::none()
            }
            Message::LanguageDragEnd => {
                self.drag = Drag::Idle;
                self.settled |= self.baseline != core_settings.general.language_priority;
                self.baseline.clear();
                Task::none()
            }
            Message::LanguageSetDefault => {
                let restored = lang::default_priority();
                self.settled |= core_settings.general.language_priority != restored;
                core_settings.general.language_priority = restored;
                Task::none()
            }
            #[cfg(target_os = "linux")]
            Message::ToggleDesktopData => {
                let is_installed = core::domains::settings::desktop::is_desktop_data_present();
                let (feedback, success) = if is_installed {
                    (DesktopFeedback::Deleted, core::domains::settings::desktop::delete_desktop_data().is_ok())
                } else {
                    (DesktopFeedback::Created, core::domains::settings::desktop::create_desktop_data().is_ok())
                };
                let kind = if success { feedback } else { DesktopFeedback::Failed };
                self.desktop_feedback.set(kind, Message::DesktopFeedbackExpired)
            }
            #[cfg(target_os = "linux")]
            Message::DesktopFeedbackExpired => {
                self.desktop_feedback.expire();
                Task::none()
            }
            Message::ManualUpdateCheck => {
                debug!("Manual update check requested, deferring to root app");
                Task::none()
            }
            Message::ShowUpdatePopup => {
                debug!("Updater popup re-open requested, deferring to root app");
                Task::none()
            }
        }
    }

    pub fn is_dragging(&self) -> bool {
        !matches!(self.drag, Drag::Idle)
    }

    pub(super) fn take_language_change(&mut self) -> bool {
        std::mem::take(&mut self.settled)
    }

    pub fn view<'a>(&'a self, core_settings: &'a CoreSettings, updater_status: &'a UpdateStatus) -> Element<'a, Message> {
        let update_modes = vec!["Prompt", "Ignore"];
        let current_update_mode = match core_settings.general.update_mode {
            UpdateMode::Prompt => "Prompt",
            UpdateMode::Ignore => "Ignore",
        };

        let mut system_content = column![].spacing(10);

        #[cfg(target_os = "linux")]
        {
            let is_installed = core::domains::settings::desktop::is_desktop_data_present();
            let (label, style): (&str, theme::ButtonStyleFn) = match self.desktop_feedback.get() {
                Some(DesktopFeedback::Created) => ("Desktop Data Created!", theme::success_button),
                Some(DesktopFeedback::Deleted) => ("Desktop Data Deleted!", theme::success_button),
                Some(DesktopFeedback::Failed) => ("Failed!", theme::danger_button),
                None if is_installed => ("Delete Desktop Data", theme::danger_button),
                None => ("Create Desktop Data", theme::primary_button),
            };
            system_content = system_content.push(
                theme::sized_button(label, theme::STATUS_BUTTON_WIDTH, style).on_press(Message::ToggleDesktopData)
            );
        }

        let (update_label, update_style, update_msg): (&str, theme::ButtonStyleFn, Option<Message>) = match updater_status {
            UpdateStatus::Checking => ("Checking for Updates...", theme::warning_button, None),
            UpdateStatus::UpToDate => ("Up to Date!", theme::success_button, None),
            UpdateStatus::UpdateFound(..) => ("Update Found!", theme::success_button, Some(Message::ShowUpdatePopup)),
            UpdateStatus::CheckFailed => ("Failed to Check!", theme::danger_button, None),
            UpdateStatus::Downloading(_) => ("Downloading Update...", theme::primary_button, Some(Message::ShowUpdatePopup)),
            UpdateStatus::RestartPending(_) => ("Restart Pending!", theme::warning_button, Some(Message::ShowUpdatePopup)),
            UpdateStatus::Idle => ("Check for Update Now", theme::primary_button, Some(Message::ManualUpdateCheck)),
        };
        system_content = system_content.push(
            theme::sized_button(update_label, theme::STATUS_BUTTON_WIDTH, update_style).on_press_maybe(update_msg)
        );

        let nightly_available = nightly::features_available();
        let weak = |theme: &Theme| iced::widget::text::Style { color: Some(theme::weak_text_color(theme)) };
        let nightly_name = text("Enable Nightly Features ");
        let nightly_moon = text(fonts::MOON_CLOSE).font(fonts::MISC_SYMBOLS);
        let nightly_label: Element<'a, Message> = if nightly_available {
            row![nightly_name, nightly_moon].align_y(Vertical::Center).into()
        } else {
            row![nightly_name.style(weak), nightly_moon.style(weak)].align_y(Vertical::Center).into()
        };
        let nightly_hint = if nightly_available {
            "Enables work-in-progress and unstable features"
        } else {
            "No Nightly features available in this version"
        };

        let behavior_content = column![
            row![
                text("Update Handling"),
                pick_list(
                    update_modes,
                    Some(current_update_mode),
                    |val| {
                        let mode = match val {
                            "Prompt" => UpdateMode::Prompt,
                            _ => UpdateMode::Ignore,
                        };
                        Message::UpdateModeSelected(mode)
                    }
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
            ].spacing(10).align_y(Alignment::Center),
            hover_hint(
                toggle_row(core_settings.general.enable_logging, text("Enable Logging"), Some(Message::ToggleLogging)),
                "Enables logs for easy debugging\nDisable to improve performance\nDevs may refuse to debug without logs",
            ),
            hover_hint(
                toggle_row(
                    core_settings.general.enable_nightly,
                    nightly_label,
                    nightly_available.then_some(Message::ToggleNightly),
                ),
                nightly_hint,
            ),
            toggle_row(core_settings.general.ignore_conflict_errors, text("Ignore Conflict Errors"), Some(Message::ToggleIgnoreConflicts)),
            toggle_row(core_settings.general.ignore_watcher_failure, text("Ignore Watcher Failure"), Some(Message::ToggleIgnoreWatcherFailure)),
        ].spacing(10);

        let language_content = column![
            theme::sized_button("Set to Default", theme::STATUS_BUTTON_WIDTH, theme::danger_button)
                .on_press(Message::LanguageSetDefault),
            self.language_list(core_settings),
        ].spacing(10);

        column![
            header_section(text("System").size(24), system_content),
            header_section(text("Behavior").size(24), behavior_content),
            header_section(text("Language Priority").size(24), language_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn language_list<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let priority = &core_settings.general.language_priority;
        let none_index = priority.iter().position(|code| code == "--");

        let mut list_col = column![].spacing(0);
        for (index, code) in priority.iter().enumerate() {
            let is_none = Some(index) == none_index;
            let is_weak = none_index.is_some_and(|none_index| index > none_index);
            let is_dragging =
                matches!(self.drag, Drag::Pressed { index: dragged } | Drag::Moving { index: dragged, .. } if dragged == index);

            list_col = list_col.push(language_row(index, code, is_none, is_weak, is_dragging));
        }

        container(list_col)
            .width(Length::Fixed(ROW_WIDTH))
            .padding(12)
            .style(theme::card_container_outlined)
            .into()
    }
}

fn language_row<'a>(index: usize, code: &'a str, is_none: bool, is_weak: bool, is_dragging: bool) -> Element<'a, Message> {
    let handle = mouse_area(text("☰").size(14))
        .interaction(Interaction::Grab)
        .on_press(Message::LanguageDragStart(index));

    let label = if is_none { theme::bold_text(lang::get_label_for_code(code)) } else { text(lang::get_label_for_code(code)) };

    let label: Element<'_, Message> = if is_weak {
        label.style(|theme: &Theme| iced::widget::text::Style { color: Some(theme::weak_text_color(theme)) }).into()
    } else {
        label.into()
    };

    let content = row![handle, label].spacing(8).align_y(Alignment::Center);

    container(content)
        .width(Length::Fixed(ROW_WIDTH))
        .height(Length::Fixed(ROW_HEIGHT))
        .align_y(Vertical::Center)
        .padding([0, 6])
        .style(move |theme: &Theme| {
            if is_dragging {
                container::Style {
                    background: Some(Color { a: DRAG_HIGHLIGHT_ALPHA, ..theme.palette().primary }.into()),
                    border: Border { radius: Radius::from(theme::RADIUS_SM), ..Border::default() },
                    ..container::Style::default()
                }
            } else {
                container::Style::default()
            }
        })
        .into()
}
