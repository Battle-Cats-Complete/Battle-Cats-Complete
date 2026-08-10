use std::path::{Path, PathBuf};
use std::thread;

use iced::futures::channel::mpsc;
use iced::task;
use iced::widget::{column, container, pick_list, row, rule, scrollable, space, text, text_input};
use iced::{Element, Length, Padding, Size, Task, Theme};
use tracing::{info, warn};

use core::addons::adb::mods as adb_mods;
use core::addons::paths::{self, Presence};
use core::common::job::{JobEvent, JobOutcome};
use core::modules::mods::import::{self, ModImportTab, ModPackType};
use core::modules::mods::ModDataState;
use core::modules::settings::Settings;

use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::{popup, smooth_scroll, ConsoleState};

use super::{
    field_row, job_finished, picked_label, FIELD_ROW_SPACING, POPUP_PADDING, RULE_PADDING, RULE_THICKNESS,
    SCROLLBAR_GAP,
};

const POPUP_SIZE: Size = Size::new(500.0, 520.0);
const POPUP_TAB_WIDTH: f32 = 90.0;
const PACKAGE_FIELD_WIDTH: f32 = 70.0;

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Open,
    TabSelected(ModImportTab),
    PackageSuffixChanged(String),
    SelectArchive,
    FormatSelected(ModPackType),
    SelectSource,
    PackErrorExpired,
    StartImport,
    Job(JobEvent),
    ConsoleScrolled(scrollable::Viewport),
}

#[derive(Default)]
pub struct State {
    pub is_open: bool,
    popup: popup::State,
    selected_path: Option<PathBuf>,
    pack_error: Slot<String>,
    running: bool,
    log: String,
    console: ConsoleState,
    job_handle: Option<task::Handle>,
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut ModDataState, settings: &Settings) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    self.is_open = false;
                }
                Task::none()
            }
            Message::Open => {
                self.is_open = true;
                Task::none()
            }
            Message::PackErrorExpired => {
                self.pack_error.expire();
                Task::none()
            }
            Message::TabSelected(tab) => {
                data.import.tab = tab;
                Task::none()
            }
            Message::PackageSuffixChanged(value) => {
                data.import.package_suffix = value;
                Task::none()
            }
            Message::SelectArchive => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Archive", &["bcm", "zip"]).pick_file() {
                    self.selected_path = Some(path);
                }
                Task::none()
            }
            Message::FormatSelected(format) => {
                data.import.pack_type = format;
                self.selected_path = None;
                self.pack_error.clear();
                Task::none()
            }
            Message::SelectSource => self.handle_select_source(data),
            Message::StartImport => self.start_import(data, settings),
            Message::Job(event) => self.apply_job_event(event, data),
            Message::ConsoleScrolled(viewport) => {
                self.console.on_scroll(viewport);
                Task::none()
            }
        }
    }

    fn apply_job_event(&mut self, event: JobEvent, data: &mut ModDataState) -> Task<Message> {
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
                        info!("Import job completed.");
                        data.refresh_mods();
                        Task::none()
                    }
                    JobOutcome::Aborted => {
                        info!("Import job aborted.");
                        Task::none()
                    }
                    JobOutcome::Failed(message) => {
                        warn!("Import failed: {}", message);
                        self.log.push_str(&format!("ERROR: {}\n", message));
                        self.console.snap_to_bottom()
                    }
                }
            }
        }
    }

    fn begin(&mut self, status: String) {
        self.running = true;
        self.log.clear();
        self.log.push_str(&status);
        self.log.push('\n');
    }

    fn start_import(&mut self, data: &mut ModDataState, settings: &Settings) -> Task<Message> {
        if self.running {
            return Task::none();
        }

        let enforce_validation = settings.game_data.enforce_key_validation;
        let (tx, rx) = mpsc::unbounded();

        match data.import.tab {
            ModImportTab::Adb => {
                let suffix = data.import.package_suffix.clone();
                self.begin("Initializing Mod ADB Pull...".to_string());

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = adb_mods::run(suffix, enforce_validation, emit);
                    emit(job_finished(result));
                });
            }
            ModImportTab::Bcm => {
                let Some(path) = self.selected_path.clone() else {
                    return Task::none();
                };
                self.begin(format!("Extracting {:?}...", path.file_name().unwrap_or_default()));

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = import::run_bcm(path, enforce_validation, emit);
                    emit(job_finished(result));
                });
            }
            ModImportTab::Pack => {
                let Some(path) = self.selected_path.clone() else {
                    return Task::none();
                };
                let pack_type = data.import.pack_type;
                self.begin(format!("Processing {:?}...", path.file_name().unwrap_or_default()));

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = import::run_pack(path, pack_type, enforce_validation, emit);
                    emit(job_finished(result));
                });
            }
        }

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.job_handle = Some(handle);
        stream_task.map(Message::Job)
    }

    fn handle_select_source(&mut self, data: &ModDataState) -> Task<Message> {
        match data.import.pack_type {
            ModPackType::Apk => {
                if let Some(path) = rfd::FileDialog::new().add_filter("APK", &["apk", "xapk", "apkm"]).pick_file() {
                    self.selected_path = Some(path);
                }
            }
            ModPackType::Pack => {
                let Some(files) = rfd::FileDialog::new().add_filter("Pack/List", &["pack", "list"]).pick_files() else { return Task::none(); };
                let Some(first) = files.first() else { return Task::none(); };

                let parent = first.parent().unwrap_or(Path::new(""));
                let stem = first.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let pack_file = parent.join(format!("{}.pack", stem));
                let list_file = parent.join(format!("{}.list", stem));

                if !pack_file.exists() {
                    self.selected_path = None;
                    return self.pack_error.set("Missing .pack!".to_string(), Message::PackErrorExpired);
                } else if !list_file.exists() {
                    self.selected_path = None;
                    return self.pack_error.set("Missing .list!".to_string(), Message::PackErrorExpired);
                } else {
                    self.selected_path = Some(pack_file);
                    self.pack_error.clear();
                }
            }
        }

        Task::none()
    }

    pub fn view<'a>(&'a self, data: &'a ModDataState, window: Size) -> Element<'a, Message> {
        self.popup.view("Import Mod", POPUP_SIZE, window, Message::Popup, move || {
            let upper = smooth_scroll(
                scrollable(container(self.content_view(data)).width(Length::Fill).padding(POPUP_PADDING))
                    .spacing(SCROLLBAR_GAP)
                    .height(Length::Shrink)
            );

            column![upper, self.view_console_section()]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }, None)
    }

    fn content_view<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let tabs_row = row![
            tab_button("Android", data.import.tab == ModImportTab::Adb, Message::TabSelected(ModImportTab::Adb)),
            tab_button("BCM", data.import.tab == ModImportTab::Bcm, Message::TabSelected(ModImportTab::Bcm)),
            tab_button("Pack", data.import.tab == ModImportTab::Pack, Message::TabSelected(ModImportTab::Pack)),
        ].spacing(8);

        let content: Element<'a, Message> = match data.import.tab {
            ModImportTab::Adb => self.view_adb(data),
            ModImportTab::Bcm => self.view_bcm(data),
            ModImportTab::Pack => self.view_pack(data),
        };

        column![
            tabs_row,
            space().height(RULE_PADDING),
            rule::horizontal(RULE_THICKNESS),
            space().height(RULE_PADDING),
            content
        ]
            .spacing(0)
            .width(Length::Fill)
            .into()
    }

    fn view_console_section(&self) -> Element<'_, Message> {
        container(
            column![
                rule::horizontal(RULE_THICKNESS),
                space().height(RULE_PADDING),
                self.console.view(&self.log, Message::ConsoleScrolled)
            ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding { top: 0.0, right: POPUP_PADDING, bottom: POPUP_PADDING, left: POPUP_PADDING })
            .into()
    }

    fn view_adb<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let has_adb = paths::adb_status() == Presence::Installed;
        let is_busy = self.running;

        let info: Element<'a, Message> = if has_adb {
            text("Import mod package using Android/Emulator").into()
        } else {
            text("Android Bridge is required. Download it in Settings > Add-Ons")
                .style(text::warning)
                .into()
        };

        let package_row = field_row(
            "Package",
            text_input("en", &data.import.package_suffix)
                .on_input_maybe((!is_busy && has_adb).then_some(Message::PackageSuffixChanged))
                .width(Length::Fixed(PACKAGE_FIELD_WIDTH))
                .style(theme::rounded_input),
        );

        let btn_text = if has_adb { "Start Import" } else { "ADB Missing" };
        let start_btn = theme::sized_button(btn_text, theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
            .on_press_maybe((!is_busy && has_adb).then_some(Message::StartImport));

        column![info, package_row, start_btn].spacing(FIELD_ROW_SPACING).into()
    }

    fn view_bcm<'a>(&'a self, _data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = self.running;

        let (label, style) = pick_button_face(self.selected_path.as_deref(), "Select Archive");

        let select_btn = theme::sized_button(label, theme::MANAGE_BUTTON_WIDTH, style)
            .on_press_maybe((!is_busy).then_some(Message::SelectArchive));

        let start_btn = theme::sized_button("Start Import", theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
            .on_press_maybe((!is_busy && self.selected_path.is_some()).then_some(Message::StartImport));

        column![
            text("Import packaged .bcm or .zip mod archives"),
            select_btn,
            start_btn
        ].spacing(FIELD_ROW_SPACING).into()
    }

    fn view_pack<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = self.running;

        let pack_types = vec!["APK".to_string(), "Pack".to_string()];
        let selected_pack_type = match data.import.pack_type {
            ModPackType::Apk => "APK".to_string(),
            ModPackType::Pack => "Pack".to_string(),
        };

        let format_row = field_row(
            "Format",
            pick_list(
                pack_types,
                Some(selected_pack_type),
                |s| Message::FormatSelected(if s == "APK" { ModPackType::Apk } else { ModPackType::Pack })
            )
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
        );

        let default_label = if data.import.pack_type == ModPackType::Pack { "Select Pack/List" } else { "Select Source" };

        let (label, style) = match self.pack_error.get() {
            Some(message) => (message.clone(), theme::danger_button as theme::ButtonStyleFn),
            None => pick_button_face(self.selected_path.as_deref(), default_label),
        };

        let source_row = field_row(
            "Source",
            theme::sized_button(label, theme::MANAGE_BUTTON_WIDTH, style)
                .on_press_maybe((!is_busy).then_some(Message::SelectSource)),
        );

        let start_btn = theme::sized_button("Start Import", theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
            .on_press_maybe((!is_busy && self.selected_path.is_some()).then_some(Message::StartImport));

        column![
            text("Import modded files directly from game formats"),
            format_row,
            source_row,
            start_btn
        ].spacing(FIELD_ROW_SPACING).into()
    }

}

fn pick_button_face(selected: Option<&Path>, default_label: &str) -> (String, theme::ButtonStyleFn) {
    selected.map_or_else(
        || (default_label.to_string(), theme::neutral_button as theme::ButtonStyleFn),
        |path| (picked_label(path), theme::success_button as theme::ButtonStyleFn),
    )
}

fn tab_button<'a>(label: &'a str, is_active: bool, msg: Message) -> iced::widget::Button<'a, Message> {
    theme::sized_button(label, POPUP_TAB_WIDTH, move |t: &Theme, status| theme::toggle_button(t, status, is_active))
        .on_press(msg)
}
