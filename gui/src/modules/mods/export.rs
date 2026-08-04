use std::thread;

use iced::futures::channel::mpsc;
use iced::task;
use iced::widget::{button, column, container, pick_list, row, scrollable, slider, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};
use tracing::{error, info};

use core::common::job::{JobEvent, JobOutcome};
use core::common::region::Region;
use core::modules::mods::export::{apk, bcm, pack, ExportType};
use core::modules::mods::ModDataState;
use core::modules::settings::Settings;

use crate::app::theme;
use crate::widget::popup;

use super::job_finished;

const SPINNER_FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
const REGIONS: [Region; 4] = [Region::En, Region::Ja, Region::Ko, Region::Tw];
const POPUP_SIZE: Size = Size::new(400.0, 328.0);

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
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    compression: f32,
    busy_frame: usize,
    running: bool,
    log: String,
    job_handle: Option<task::Handle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            popup: popup::State::default(),
            compression: bcm::BCM_COMPRESSION_DEFAULT as f32,
            busy_frame: 0,
            running: false,
            log: String::new(),
            job_handle: None,
        }
    }
}

impl State {
    pub(super) fn advance_spinner(&mut self) {
        if self.running {
            self.busy_frame = (self.busy_frame + 1) % SPINNER_FRAMES.len();
        }
    }

    pub(super) fn is_running(&self) -> bool {
        self.running
    }

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
            Message::Job(event) => {
                match event {
                    JobEvent::Log(line) => {
                        self.log.push_str(&format!("{}\n", line));
                    }
                    JobEvent::Progress { .. } => {}
                    JobEvent::Finished(outcome) => {
                        self.running = false;
                        self.job_handle = None;

                        match outcome {
                            JobOutcome::Completed => info!("Export job completed."),
                            JobOutcome::Aborted => info!("Export job aborted."),
                            JobOutcome::Failed(message) => {
                                error!("Export Error: {}", message);
                                self.log.push_str(&format!("!! ERROR: {}\n", message));
                            }
                        }
                    }
                }
                Task::none()
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
        self.popup.view("Export Mod", POPUP_SIZE, window, Message::Popup, move || {
            container(scrollable(self.content_view(data)))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
                .into()
        })
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

        let raw_log = self.log.trim_end();
        let display_status = raw_log.lines().last().unwrap_or("Ready").replace('\n', ", ");

        let is_error = display_status.contains("ERROR") || display_status.contains("Error") || display_status.contains("Failed");
        let is_success = display_status.contains("Successfully") || display_status.contains("Complete");

        let status_row: Element<'a, Message> = if is_busy {
            row![text(SPINNER_FRAMES[self.busy_frame]), text(display_status)].spacing(8).into()
        } else {
            let color = if is_error {
                Color::from_rgb(1.0, 0.6, 0.6)
            } else if is_success {
                Color::from_rgb(0.6, 1.0, 0.6)
            } else {
                Color::from_rgb(0.6, 0.8, 1.0)
            };
            text(display_status).color(color).into()
        };

        let log_display = scrollable(text(self.log.clone()).size(12)).height(Length::Fixed(150.0));

        column![tabs_row, space().height(10), content, space().height(10), status_row, log_display].spacing(8).into()
    }

    fn region_select<'a>(&self, current: Region) -> Element<'a, Message> {
        let names: Vec<String> = REGIONS.iter().map(|r| r.metadata().display_name.to_string()).collect();
        let selected = current.metadata().display_name.to_string();

        pick_list(names, Some(selected), |label| {
            let region = REGIONS.iter().copied().find(|r| r.metadata().display_name == label).unwrap_or(Region::En);
            Message::RegionSelected(region)
        }).into()
    }

    fn view_apk<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        let file_label = data.export.selected_apk.as_ref()
            .map(|p| truncate_file_name(p))
            .unwrap_or_else(|| "No file selected".to_string());

        column![
            text("Patch and export modded APK"),
            row![
                text("Title:"),
                text_input("", &data.export.app_title)
                    .on_input_maybe((!is_busy).then_some(Message::TitleChanged))
                    .width(Length::Fixed(150.0))
            ].align_y(Alignment::Center).spacing(4),
            row![
                text("Package:"),
                text_input("", &data.export.package_suffix)
                    .on_input_maybe((!is_busy).then_some(Message::PackageChanged))
                    .width(Length::Fixed(60.0))
            ].align_y(Alignment::Center).spacing(4),
            row![text("Region:"), self.region_select(data.export.target_region)].align_y(Alignment::Center).spacing(4),
            row![
                button("Select App File").on_press_maybe((!is_busy).then_some(Message::SelectAppFile)),
                text(file_label)
            ].align_y(Alignment::Center).spacing(8),
            button("Apply Mod")
                .on_press_maybe((!is_busy && is_ready && data.export.selected_apk.is_some()).then_some(Message::StartExport))
                .style(theme::primary_button)
        ].spacing(12).into()
    }

    fn view_bcm<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        column![
            text("Package mod into a standalone .bcm archive"),
            row![
                text("Title:"),
                text_input("", &data.export.app_title)
                    .on_input_maybe((!is_busy).then_some(Message::TitleChanged))
                    .width(Length::Fixed(150.0))
            ].align_y(Alignment::Center).spacing(4),
            row![
                text("Compression:"),
                slider(bcm::BCM_COMPRESSION_MIN as f32..=bcm::BCM_COMPRESSION_MAX as f32, self.compression, Message::CompressionChanged)
                    .width(Length::Fixed(150.0))
            ].align_y(Alignment::Center).spacing(4),
            button("Create BCM Package")
                .on_press_maybe((!is_busy && is_ready).then_some(Message::StartExport))
                .style(theme::primary_button)
        ].spacing(12).into()
    }

    fn view_pack<'a>(&'a self, data: &'a ModDataState, is_busy: bool, is_ready: bool) -> Element<'a, Message> {
        column![
            text("Compile mod files into raw .pack and .list files"),
            row![
                text("Name:"),
                text_input("DownloadLocal", &data.export.pack_name)
                    .on_input_maybe((!is_busy).then_some(Message::PackNameChanged))
                    .width(Length::Fixed(150.0))
            ].align_y(Alignment::Center).spacing(4),
            row![text("Key:"), self.region_select(data.export.target_region)].align_y(Alignment::Center).spacing(4),
            button("Create Pack")
                .on_press_maybe((!is_busy && is_ready).then_some(Message::StartExport))
                .style(theme::primary_button)
        ].spacing(12).into()
    }
}

fn truncate_file_name(path: &std::path::Path) -> String {
    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    if file_name.chars().count() <= 30 {
        return file_name;
    }

    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let stem_chars: Vec<char> = stem.chars().collect();

    if stem_chars.len() > 15 {
        let first: String = stem_chars[..12].iter().collect();
        let last: String = stem_chars[stem_chars.len() - 5..].iter().collect();
        format!("{}...{}.{}", first, last, ext)
    } else {
        file_name
    }
}

fn tab_button<'a>(label: &'a str, is_active: bool, msg: Message) -> iced::widget::Button<'a, Message> {
    button(text(label).align_x(Alignment::Center))
        .width(Length::Fixed(80.0))
        .on_press(msg)
        .style(move |t: &Theme, status| theme::toggle_button(t, status, is_active))
}
