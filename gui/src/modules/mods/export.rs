use std::thread;

use iced::futures::channel::mpsc;
use iced::task;
use iced::widget::{column, container, pick_list, row, rule, scrollable, slider, space, text, text_input};
use iced::{Element, Length, Padding, Size, Task, Theme};
use tracing::{error, info};

use core::common::job::{JobEvent, JobOutcome};
use core::common::region::Region;
use core::modules::mods::export::{apk, bcm, pack, ExportType};
use core::modules::mods::ModDataState;
use core::modules::settings::Settings;

use crate::app::theme;
use crate::widget::{popup, smooth_scroll, ConsoleState};

use super::{
    field_row, job_finished, picked_label, FIELD_ROW_SPACING, POPUP_PADDING, RULE_PADDING, RULE_THICKNESS,
    SCROLLBAR_GAP,
};

const REGIONS: [Region; 4] = [Region::En, Region::Ja, Region::Ko, Region::Tw];
const POPUP_TAB_WIDTH: f32 = 80.0;
const FIELD_WIDTH: f32 = 160.0;
const PACKAGE_FIELD_WIDTH: f32 = 70.0;

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Open,
    TabSelected(ExportType),
    TitleChanged(String),
    PackageChanged(String),
    RegionSelected(Region),
    SelectAppFile,
    CompressionChanged(f32),
    PackNameChanged(String),
    StartExport,
    Job(JobEvent),
    ConsoleScrolled(scrollable::Viewport),
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    compression: f32,
    running: bool,
    log: String,
    console: ConsoleState,
    job_handle: Option<task::Handle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            popup: popup::State::default(),
            compression: bcm::BCM_COMPRESSION_DEFAULT as f32,
            running: false,
            log: String::new(),
            console: ConsoleState::default(),
            job_handle: None,
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut ModDataState, settings: &Settings) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, popup::Kind::ModExport) {
                    self.is_open = false;
                }
                Task::none()
            }
            Message::Open => {
                self.is_open = true;
                Task::none()
            }
            Message::TabSelected(tab) => {
                data.export.tab = tab;
                Task::none()
            }
            Message::TitleChanged(value) => {
                data.export.app_title = value;
                Task::none()
            }
            Message::PackageChanged(value) => {
                data.export.package_suffix = value;
                Task::none()
            }
            Message::RegionSelected(region) => {
                data.export.target_region = region;
                Task::none()
            }
            Message::SelectAppFile => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Android App", &["apk", "xapk", "apkm", "apks"]).pick_file() {
                    data.export.selected_apk = Some(path);
                }
                Task::none()
            }
            Message::CompressionChanged(value) => {
                self.compression = value;
                Task::none()
            }
            Message::PackNameChanged(value) => {
                data.export.pack_name = value;
                Task::none()
            }
            Message::StartExport => self.start_export(data, settings),
            Message::Job(event) => self.apply_job_event(event),
            Message::ConsoleScrolled(viewport) => {
                self.console.on_scroll(viewport);
                Task::none()
            }
        }
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
                        info!("Export job completed.");
                        Task::none()
                    }
                    JobOutcome::Aborted => {
                        info!("Export job aborted.");
                        Task::none()
                    }
                    JobOutcome::Failed(message) => {
                        error!("Export Error: {}", message);
                        self.log.push_str(&format!("!! ERROR: {}\n", message));
                        self.console.snap_to_bottom()
                    }
                }
            }
        }
    }

    fn start_export(&mut self, data: &mut ModDataState, settings: &Settings) -> Task<Message> {
        if self.running {
            return Task::none();
        }

        let Some(mod_folder) = data.selected_mod.clone() else {
            return Task::none();
        };

        if data.export.tab == ExportType::Pack && data.export.pack_name.is_empty() {
            data.export.pack_name = "DownloadLocal".to_string();
        }

        self.running = true;
        self.log.clear();

        let (tx, rx) = mpsc::unbounded();

        match data.export.tab {
            ExportType::Apk => {
                let Some(input_apk) = data.export.selected_apk.clone() else {
                    self.running = false;
                    return Task::none();
                };
                let app_title = data.export.app_title.clone();
                let suffix = data.export.package_suffix.clone();
                let region = data.export.target_region;
                let behavior = settings.mods.export_behavior.clone();
                let enforce = settings.game_data.enforce_key_validation;

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = apk::run(mod_folder, input_apk, app_title, suffix, region, behavior, enforce, emit);
                    emit(job_finished(result));
                });
            }
            ExportType::Bcm => {
                let app_title = data.export.app_title.clone();
                let compression = self.compression as i64;

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = bcm::run(mod_folder, app_title, compression, emit);
                    emit(job_finished(result));
                });
            }
            ExportType::Pack => {
                let pack_name = data.export.pack_name.clone();
                let region = data.export.target_region;
                let enforce = settings.game_data.enforce_key_validation;

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = pack::run(mod_folder, pack_name, region, enforce, emit);
                    emit(job_finished(result));
                });
            }
        }

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.job_handle = Some(handle);
        stream_task.map(Message::Job)
    }

    pub fn view<'a>(&'a self, data: &'a ModDataState, window: Size) -> Element<'a, Message> {
        self.popup.view("Export Mod", popup::Kind::ModExport, window, Message::Popup, move || {
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
        let is_busy = self.running;
        let is_ready = data.selected_mod.is_some();

        let tabs_row = row![
            tab_button("APK", data.export.tab == ExportType::Apk, Message::TabSelected(ExportType::Apk)),
            tab_button("BCM", data.export.tab == ExportType::Bcm, Message::TabSelected(ExportType::Bcm)),
            tab_button("Pack", data.export.tab == ExportType::Pack, Message::TabSelected(ExportType::Pack)),
        ].spacing(8);

        let content: Element<'a, Message> = match data.export.tab {
            ExportType::Apk => self.view_apk(data, is_busy, is_ready),
            ExportType::Bcm => self.view_bcm(data, is_busy, is_ready),
            ExportType::Pack => self.view_pack(data, is_busy, is_ready),
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

    fn region_select<'a>(&self, current: Region) -> Element<'a, Message> {
        let names: Vec<String> = REGIONS.iter().map(|r| r.metadata().display_name.to_string()).collect();
        let selected = current.metadata().display_name.to_string();

        pick_list(names, Some(selected), |label| {
            let region = REGIONS.iter().copied().find(|r| r.metadata().display_name == label).unwrap_or(Region::En);
            Message::RegionSelected(region)
        })
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu)
            .into()
    }

    fn view_apk<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        let (file_label, file_style) = data.export.selected_apk.as_deref().map_or_else(
            || ("Select App File".to_string(), theme::neutral_button as theme::ButtonStyleFn),
            |path| (picked_label(path), theme::success_button as theme::ButtonStyleFn),
        );

        column![
            text("Patch and export modded APK"),
            field_row(
                "Title",
                text_input("", &data.export.app_title)
                    .on_input_maybe((!is_busy).then_some(Message::TitleChanged))
                    .width(Length::Fixed(FIELD_WIDTH))
                    .style(theme::rounded_input)
            ),
            field_row(
                "Package",
                text_input("", &data.export.package_suffix)
                    .on_input_maybe((!is_busy).then_some(Message::PackageChanged))
                    .width(Length::Fixed(PACKAGE_FIELD_WIDTH))
                    .style(theme::rounded_input)
            ),
            field_row("Region", self.region_select(data.export.target_region)),
            field_row(
                "App File",
                theme::sized_button(file_label, theme::MANAGE_BUTTON_WIDTH, file_style)
                    .on_press_maybe((!is_busy).then_some(Message::SelectAppFile))
            ),
            theme::sized_button("Apply Mod", theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
                .on_press_maybe((!is_busy && is_ready && data.export.selected_apk.is_some()).then_some(Message::StartExport))
        ].spacing(FIELD_ROW_SPACING).into()
    }

    fn view_bcm<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        column![
            text("Package mod into a standalone .bcm archive"),
            field_row(
                "Title",
                text_input("", &data.export.app_title)
                    .on_input_maybe((!is_busy).then_some(Message::TitleChanged))
                    .width(Length::Fixed(FIELD_WIDTH))
                    .style(theme::rounded_input)
            ),
            field_row(
                "Compression",
                slider(bcm::BCM_COMPRESSION_MIN as f32..=bcm::BCM_COMPRESSION_MAX as f32, self.compression, Message::CompressionChanged)
                    .width(Length::Fixed(FIELD_WIDTH))
            ),
            theme::sized_button("Create BCM Package", theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
                .on_press_maybe((!is_busy && is_ready).then_some(Message::StartExport))
        ].spacing(FIELD_ROW_SPACING).into()
    }

    fn view_pack<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        column![
            text("Compile mod files into raw .pack and .list files"),
            field_row(
                "Name",
                text_input("DownloadLocal", &data.export.pack_name)
                    .on_input_maybe((!is_busy).then_some(Message::PackNameChanged))
                    .width(Length::Fixed(FIELD_WIDTH))
                    .style(theme::rounded_input)
            ),
            field_row("Key", self.region_select(data.export.target_region)),
            theme::sized_button("Create Pack", theme::MANAGE_BUTTON_WIDTH, theme::primary_button)
                .on_press_maybe((!is_busy && is_ready).then_some(Message::StartExport))
        ].spacing(FIELD_ROW_SPACING).into()
    }
}

fn tab_button<'a>(label: &'a str, is_active: bool, msg: Message) -> iced::widget::Button<'a, Message> {
    theme::sized_button(label, POPUP_TAB_WIDTH, move |t: &Theme, status| theme::toggle_button(t, status, is_active))
        .on_press(msg)
}
