use iced::widget::{button, column, pick_list, row, scrollable, slider, space, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};

use core::common::region::Region;
use core::modules::mods::export::{apk, bcm, pack, ExportType};
use core::modules::mods::ModDataState;
use core::modules::settings::Settings;

const SPINNER_FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
const REGIONS: [Region; 4] = [Region::En, Region::Ja, Region::Ko, Region::Tw];

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Tick,
    TabSelected(ExportType),
    TitleChanged(String),
    PackageChanged(String),
    RegionSelected(Region),
    SelectAppFile,
    CompressionChanged(f32),
    PackNameChanged(String),
    StartExport,
}

pub struct State {
    pub is_open: bool,
    compression: f32,
    busy_frame: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { is_open: false, compression: bcm::BCM_COMPRESSION_DEFAULT as f32, busy_frame: 0 }
    }
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut ModDataState, settings: &Settings) -> Task<Message> {
        match message {
            Message::Open => {
                self.is_open = true;
                Task::none()
            }
            Message::Close => {
                self.is_open = false;
                Task::none()
            }
            Message::Tick => {
                if data.export.is_busy {
                    self.busy_frame = (self.busy_frame + 1) % SPINNER_FRAMES.len();
                }
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
            Message::StartExport => {
                if data.export.is_busy || data.selected_mod.is_none() {
                    return Task::none();
                }

                match data.export.tab {
                    ExportType::Apk => apk::start_export(data, settings),
                    ExportType::Bcm => bcm::start_bcm_export(data, self.compression as i64),
                    ExportType::Pack => {
                        if data.export.pack_name.is_empty() {
                            data.export.pack_name = "DownloadLocal".to_string();
                        }
                        pack::start_pack_export(data);
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = data.export.is_busy;
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

        let raw_log = data.export.log_content.trim_end();
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

        let log_display = scrollable(text(data.export.log_content.clone()).size(12)).height(Length::Fixed(150.0));

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
                .style(primary_button_style)
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
                .style(primary_button_style)
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
                .style(primary_button_style)
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
        .style(move |theme: &Theme, _status| {
            let palette = theme.palette();
            button::Style {
                background: Some(Background::Color(if is_active { palette.primary } else { Color { a: 0.2, ..palette.text } })),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            }
        })
}

fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let bg = if status == button::Status::Hovered {
        Color { a: 0.8, ..palette.primary }
    } else {
        palette.primary
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border::default().rounded(4.0),
        ..Default::default()
    }
}
