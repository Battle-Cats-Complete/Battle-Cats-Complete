use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::widget::{button, column, container, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Task, Theme};

use core::modules::addons::paths::{self, Presence};
use core::modules::mods::import::{self, ModImportTab, ModPackType};
use core::modules::mods::ModDataState;

use crate::common::popup;

const FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const SPINNER_FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
const POPUP_SIZE: Size = Size::new(500.0, 428.0);

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Open,
    Tick,
    TabSelected(ModImportTab),
    PackageSuffixChanged(String),
    SelectArchive,
    FormatSelected(ModPackType),
    SelectSource,
    StartImport,
}

#[derive(Default)]
pub struct State {
    pub is_open: bool,
    popup: popup::State,
    selected_path: Option<PathBuf>,
    pack_error: Option<(String, Instant)>,
    busy_frame: usize,
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut ModDataState) -> Task<Message> {
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
            Message::Tick => {
                if data.import.is_busy {
                    self.busy_frame = (self.busy_frame + 1) % SPINNER_FRAMES.len();
                }
                if self.pack_error.as_ref().is_some_and(|(_, at)| at.elapsed() > FEEDBACK_DURATION) {
                    self.pack_error = None;
                }
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
                self.pack_error = None;
                Task::none()
            }
            Message::SelectSource => {
                self.handle_select_source(data);
                Task::none()
            }
            Message::StartImport => {
                if data.import.is_busy {
                    return Task::none();
                }

                match data.import.tab {
                    ModImportTab::Adb => import::start_adb_import(data),
                    ModImportTab::Bcm => {
                        if let Some(path) = self.selected_path.clone() {
                            import::start_bcm_import(data, path);
                        }
                    }
                    ModImportTab::Pack => {
                        if let Some(path) = self.selected_path.clone() {
                            import::start_pack_import(data, path);
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn handle_select_source(&mut self, data: &ModDataState) {
        match data.import.pack_type {
            ModPackType::Apk => {
                if let Some(path) = rfd::FileDialog::new().add_filter("APK", &["apk", "xapk", "apkm"]).pick_file() {
                    self.selected_path = Some(path);
                }
            }
            ModPackType::Pack => {
                let Some(files) = rfd::FileDialog::new().add_filter("Pack/List", &["pack", "list"]).pick_files() else { return; };
                let Some(first) = files.first() else { return; };

                let parent = first.parent().unwrap_or(Path::new(""));
                let stem = first.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let pack_file = parent.join(format!("{}.pack", stem));
                let list_file = parent.join(format!("{}.list", stem));

                if !pack_file.exists() {
                    self.selected_path = None;
                    self.pack_error = Some(("Missing .pack!".to_string(), Instant::now()));
                } else if !list_file.exists() {
                    self.selected_path = None;
                    self.pack_error = Some(("Missing .list!".to_string(), Instant::now()));
                } else {
                    self.selected_path = Some(pack_file);
                    self.pack_error = None;
                }
            }
        }
    }

    pub fn view<'a>(&'a self, data: &'a ModDataState, window: Size) -> Element<'a, Message> {
        self.popup.view("Import Mod", POPUP_SIZE, window, Message::Popup, move || {
            container(scrollable(self.content_view(data)))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
                .into()
        })
    }

    fn content_view<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = data.import.is_busy;

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

        let status = &data.import.status_message;
        let is_error = status.contains("Error") || status.contains("Failed");
        let is_success = status.contains("Success") || status.contains("Complete");

        let status_row: Element<'a, Message> = if is_busy && !is_error && !is_success {
            row![text(SPINNER_FRAMES[self.busy_frame]), text(status.clone())].spacing(8).into()
        } else {
            let color = if is_error {
                Color::from_rgb(1.0, 0.6, 0.6)
            } else if is_success {
                Color::from_rgb(0.6, 1.0, 0.6)
            } else {
                Color::from_rgb(0.6, 0.8, 1.0)
            };
            text(status.clone()).color(color).into()
        };

        let log_display = scrollable(text(data.import.log_content.clone()).size(12)).height(Length::Fixed(150.0));

        column![tabs_row, space().height(10), content, space().height(10), status_row, log_display].spacing(8).into()
    }

    fn view_adb<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let has_adb = paths::adb_status() == Presence::Installed;
        let is_busy = data.import.is_busy;

        let info: Element<'a, Message> = if has_adb {
            text("Import mod package using Android/Emulator").into()
        } else {
            text("Android Bridge is required. Download it in Settings > Add-Ons")
                .color(Color::from_rgb(0.8, 0.6, 0.2))
                .into()
        };

        let package_row = row![
            text("Package:"),
            text_input("en", &data.import.package_suffix)
                .on_input_maybe((!is_busy && has_adb).then_some(Message::PackageSuffixChanged))
                .width(Length::Fixed(60.0))
        ].align_y(Alignment::Center).spacing(5);

        let btn_text = if has_adb { "Start Import" } else { "ADB Missing" };
        let start_btn = button(text(btn_text))
            .on_press_maybe((!is_busy && has_adb).then_some(Message::StartImport))
            .style(primary_button_style);

        column![info, package_row, start_btn].spacing(12).into()
    }

    fn view_bcm<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = data.import.is_busy;

        let label = self.selected_path.as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "No archive selected".to_string());

        column![
            text("Import packaged .bcm or .zip mod archives"),
            row![
                button("Select Archive").on_press_maybe((!is_busy).then_some(Message::SelectArchive)),
                text(label)
            ].align_y(Alignment::Center).spacing(8),
            button("Start Import")
                .on_press_maybe((!is_busy && self.selected_path.is_some()).then_some(Message::StartImport))
                .style(primary_button_style)
        ].spacing(12).into()
    }

    fn view_pack<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let is_busy = data.import.is_busy;

        let pack_types = vec!["APK".to_string(), "Pack".to_string()];
        let selected_pack_type = match data.import.pack_type {
            ModPackType::Apk => "APK".to_string(),
            ModPackType::Pack => "Pack".to_string(),
        };

        let format_row = row![
            text("Format:"),
            pick_list(
                pack_types,
                Some(selected_pack_type),
                |s| Message::FormatSelected(if s == "APK" { ModPackType::Apk } else { ModPackType::Pack })
            )
        ].align_y(Alignment::Center).spacing(8);

        let select_label = if data.import.pack_type == ModPackType::Pack { "Select Pack/List" } else { "Select Source" };

        let selection_label: Element<'a, Message> = if let Some((message, at)) = &self.pack_error {
            if at.elapsed() <= FEEDBACK_DURATION {
                text(message.clone()).color(Color::from_rgb(1.0, 0.3, 0.3)).into()
            } else {
                self.view_pack_selection_label(data)
            }
        } else {
            self.view_pack_selection_label(data)
        };

        let source_row = row![
            button(select_label).on_press_maybe((!is_busy).then_some(Message::SelectSource)),
            selection_label
        ].align_y(Alignment::Center).spacing(8);

        column![
            text("Import modded files directly from game formats"),
            format_row,
            source_row,
            button("Start Import")
                .on_press_maybe((!is_busy && self.selected_path.is_some()).then_some(Message::StartImport))
                .style(primary_button_style)
        ].spacing(12).into()
    }

    fn view_pack_selection_label<'a>(&'a self, data: &'a ModDataState) -> Element<'a, Message> {
        let Some(path) = &self.selected_path else {
            return text("No source selected").into();
        };

        if data.import.pack_type == ModPackType::Pack {
            let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            text(format!("{} Found!", stem)).color(Color::from_rgb(0.4, 1.0, 0.4)).into()
        } else {
            text(path.to_string_lossy().to_string()).into()
        }
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
