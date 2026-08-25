use std::path::PathBuf;
use std::thread;

use iced::alignment::Horizontal;
use iced::futures::channel::mpsc;
use iced::widget::{column, container, row, scrollable, text_input};
use iced::{task, Alignment, Element, Length, Task, Theme};
use tracing::{error, info};

use kore::common::job::{JobEvent, JobOutcome};
use kore::systems::addons::apkeditor;
use kore::systems::apk::clone::{self, CloneConfig};

use crate::app::theme;
use crate::common::dialog;
use crate::widget::ConsoleState;

use super::picker;

const PANEL_PADDING: f32 = 12.0;
const ROW_GAP: f32 = 8.0;

const APK_ONLY: &str = "Add APK";
const APK_SPLIT: &str = "Add APK/XAPK";
const CLONE_IDLE: &str = "Clone App";
const CLONE_BUSY: &str = "Cloning...";

#[derive(Debug, Clone)]
pub enum Message {
    PickApk,
    PickIcon,
    ApkPicked(Option<PathBuf>),
    IconPicked(Option<PathBuf>),
    ClearIcon,
    AppNameChanged(String),
    PackageChanged(String),
    Start,
    Job(JobEvent),
    Scrolled(scrollable::Viewport),
}

#[derive(Default)]
pub struct State {
    apk: Option<PathBuf>,
    icon: Option<PathBuf>,
    app_name: String,
    package: String,
    running: bool,
    job_handle: Option<task::Handle>,
    log: String,
    console: ConsoleState,
}

impl State {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickApk => Task::perform(dialog::file("Android App", apk_filters()), Message::ApkPicked),
            Message::PickIcon => Task::perform(dialog::file("PNG Image", &["png"]), Message::IconPicked),
            Message::ApkPicked(picked) => {
                if picked.is_some() {
                    self.apk = picked;
                }
                Task::none()
            }
            Message::IconPicked(picked) => {
                if picked.is_some() {
                    self.icon = picked;
                }
                Task::none()
            }
            Message::ClearIcon => {
                self.icon = None;
                Task::none()
            }
            Message::AppNameChanged(value) => {
                self.app_name = value;
                Task::none()
            }
            Message::PackageChanged(value) => {
                self.package = value;
                Task::none()
            }
            Message::Start => self.start(),
            Message::Job(event) => self.apply_job_event(event),
            Message::Scrolled(viewport) => {
                self.console.on_scroll(viewport);
                Task::none()
            }
        }
    }

    fn config(&self) -> Option<CloneConfig> {
        Some(CloneConfig {
            input: self.apk.clone()?,
            app_name: self.app_name.clone(),
            package: self.package.clone(),
            icon: self.icon.clone(),
        })
    }

    fn ready(&self) -> bool {
        !self.running && self.config().is_some_and(|config| config.is_actionable())
    }

    fn start(&mut self) -> Task<Message> {
        if self.running {
            return Task::none();
        }

        let Some(config) = self.config() else {
            return Task::none();
        };

        if !config.is_actionable() {
            return Task::none();
        }

        self.running = true;
        self.log.clear();

        let (tx, rx) = mpsc::unbounded();

        let closer = tx.clone();

        thread::spawn(move || {
            let outcome = clone::run(config, move |event: JobEvent| {
                let _ = tx.unbounded_send(event);
            });

            let finished = outcome.map_or_else(JobOutcome::Failed, |_| JobOutcome::Completed);
            let _ = closer.unbounded_send(JobEvent::Finished(finished));
        });

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.job_handle = Some(handle);

        stream_task.map(Message::Job)
    }

    fn apply_job_event(&mut self, event: JobEvent) -> Task<Message> {
        match event {
            JobEvent::Log(line) => {
                self.log.push_str(&format!("{}\n", line));
                self.console.snap_to_bottom()
            }
            JobEvent::Progress { .. } => Task::none(),
            JobEvent::Finished(outcome) => {
                self.running = false;
                self.job_handle = None;

                match outcome {
                    JobOutcome::Completed => {
                        info!("Clone job completed.");
                        Task::none()
                    }
                    JobOutcome::Aborted => {
                        info!("Clone job aborted.");
                        Task::none()
                    }
                    JobOutcome::Failed(message) => {
                        error!("Clone Error: {}", message);
                        self.log.push_str(&format!("!! ERROR: {}\n", message));
                        self.console.snap_to_bottom()
                    }
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![self.view_controls(), self.view_console()]
            .spacing(ROW_GAP)
            .height(Length::Fill)
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let files = row![
            picker::slot(apk_label(), self.apk.as_deref(), Message::PickApk),
            picker::slot("Add PNG", self.icon.as_deref(), Message::PickIcon),
            self.view_clear_icon(),
        ]
        .spacing(ROW_GAP);

        let fields = row![
            named_input("App name...", &self.app_name, Message::AppNameChanged),
            named_input("Pkg name...", &self.package, Message::PackageChanged),
            self.view_start(),
        ]
        .spacing(ROW_GAP)
        .align_y(Alignment::Center);

        let body = column![centered(files), centered(fields)].spacing(ROW_GAP);

        container(body).padding(PANEL_PADDING).into()
    }

    fn view_clear_icon(&self) -> Element<'_, Message> {
        let armed = self.icon.is_some();

        picker::action("Clear Icon", Message::ClearIcon)
            .on_press_maybe(armed.then_some(Message::ClearIcon))
            .style(move |t: &Theme, status| {
                if armed { theme::danger_button(t, status) } else { theme::neutral_button(t, status) }
            })
            .into()
    }

    fn view_start(&self) -> Element<'_, Message> {
        let armed = self.ready();
        let label = if self.running { CLONE_BUSY } else { CLONE_IDLE };

        picker::action(label, Message::Start)
            .on_press_maybe(armed.then_some(Message::Start))
            .style(move |t: &Theme, status| {
                if armed { theme::primary_button(t, status) } else { theme::neutral_button(t, status) }
            })
            .into()
    }

    fn view_console(&self) -> Element<'_, Message> {
        container(self.console.view(&self.log, Message::Scrolled))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn apk_filters() -> &'static [&'static str] {
    if apkeditor::is_installed() {
        &["apk", "xapk", "apkm", "apks"]
    } else {
        &["apk"]
    }
}

fn apk_label() -> &'static str {
    if apkeditor::is_installed() { APK_SPLIT } else { APK_ONLY }
}

fn named_input<'a>(
    hint: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(hint, value)
        .on_input(on_input)
        .width(Length::Fixed(picker::BUTTON_WIDTH))
        .padding(picker::COMBO_PADDING)
        .size(picker::TEXT_SIZE)
        .style(theme::rounded_input)
        .into()
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).width(Length::Fill).align_x(Horizontal::Center).into()
}
