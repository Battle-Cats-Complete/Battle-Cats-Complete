use tracing::warn;

use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use core::modules::addons::paths::{self, Presence};
use core::modules::animation::export::process::STATUS_RX;
use core::modules::animation::export::{EncoderStatus, ExportFormat, ExportMode, ExporterState};
use core::modules::settings::Settings;

const MODE_OPTIONS: [&str; 3] = ["Manual", "Loop", "Showcase"];
const FORMAT_OPTIONS: [&str; 8] = ["GIF", "WebP", "AVIF", "PNG", "MP4", "MKV", "WebM", "ZIP"];

#[derive(Default)]
pub struct State {
    pub is_open: bool,
    exporter: ExporterState,
}

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    SetMode(ExportMode),
    SetFormat(ExportFormat),
    SetFileName(String),
    SetStartFrame(String),
    SetEndFrame(String),
    SetLoopTolerance(String),
    SetLoopMin(String),
    SetLoopMax(String),
    SetShowcaseWalk(String),
    SetShowcaseIdle(String),
    SetShowcaseAttack(String),
    SetShowcaseKb(String),
    SetRegionX(String),
    SetRegionY(String),
    SetRegionW(String),
    SetRegionH(String),
    SetQuality(String),
    SetCompression(String),
    ToggleBackground(bool),
    BeginExport,
}

impl State {
    pub fn open(&mut self, settings: &Settings, loop_supported: bool) {
        let previous_mode = self.exporter.export_mode.clone();
        self.exporter = ExporterState::with_settings(settings);
        self.exporter.export_mode = previous_mode;
        self.exporter.loop_supported = loop_supported;

        if self.exporter.export_mode == ExportMode::Loop && !loop_supported {
            self.exporter.export_mode = ExportMode::Manual;
        }

        self.is_open = true;
    }

    pub fn tick(&mut self) {
        if !self.exporter.is_processing {
            return;
        }

        if let Ok(receiver_lock) = STATUS_RX.lock()
            && let Some(receiver) = receiver_lock.as_ref() {
            while let Ok(status) = receiver.try_recv() {
                match status {
                    EncoderStatus::Encoding => {}
                    EncoderStatus::Progress(progress) => self.exporter.encoded_frames = progress as i32,
                    EncoderStatus::Finished => self.exporter.is_processing = false,
                }
            }
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.is_open = !self.is_open,
            Message::SetMode(mode) => {
                if mode == ExportMode::Showcase {
                    self.exporter.showcase_walk_str.clear();
                    self.exporter.showcase_idle_str.clear();
                    self.exporter.showcase_attack_str.clear();
                    self.exporter.showcase_kb_str.clear();
                }
                self.exporter.export_mode = mode;
            }
            Message::SetFormat(format) => self.exporter.format = format,
            Message::SetFileName(name) => self.exporter.file_name = name,
            Message::SetStartFrame(value) => {
                self.exporter.frame_start_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.frame_start = parsed;
                }
            }
            Message::SetEndFrame(value) => {
                self.exporter.frame_end_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.frame_end = parsed;
                }
            }
            Message::SetLoopTolerance(value) => {
                self.exporter.loop_tolerance_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_tolerance = parsed;
                }
            }
            Message::SetLoopMin(value) => {
                self.exporter.loop_min_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_min = parsed;
                }
            }
            Message::SetLoopMax(value) => {
                self.exporter.loop_max = value.parse::<i32>().ok();
                self.exporter.loop_max_str = value;
            }
            Message::SetShowcaseWalk(value) => {
                self.exporter.showcase_walk_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_walk_len = parsed;
                }
            }
            Message::SetShowcaseIdle(value) => {
                self.exporter.showcase_idle_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_idle_len = parsed;
                }
            }
            Message::SetShowcaseAttack(value) => {
                self.exporter.showcase_attack_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_attack_len = parsed;
                }
            }
            Message::SetShowcaseKb(value) => {
                self.exporter.showcase_kb_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_kb_len = parsed;
                }
            }
            Message::SetRegionX(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_x = parsed;
                }
            }
            Message::SetRegionY(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_y = parsed;
                }
            }
            Message::SetRegionW(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_w = parsed;
                }
            }
            Message::SetRegionH(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_h = parsed;
                }
            }
            Message::SetQuality(value) => {
                self.exporter.quality_percent_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.quality_percent = parsed;
                }
            }
            Message::SetCompression(value) => {
                self.exporter.compression_percent_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.compression_percent = parsed;
                }
            }
            Message::ToggleBackground(enabled) => self.exporter.background = enabled,
            Message::BeginExport => {
                warn!("Export not yet wired: the batch encoder still requires a GL frame source, which the new canvas-based viewer doesn't provide");
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let is_avif_missing = paths::avifenc_status() != Presence::Installed;
        let is_ffmpeg_missing = paths::ffmpeg_status() != Presence::Installed;
        let is_locked = self.exporter.is_processing || self.exporter.is_loop_searching;

        let selected_mode = match self.exporter.export_mode {
            ExportMode::Manual => "Manual",
            ExportMode::Loop => "Loop",
            ExportMode::Showcase => "Showcase",
        };

        let mode_picker = row![
            text("Mode"),
            pick_list(&MODE_OPTIONS[..], Some(selected_mode), |selected: &str| {
                Message::SetMode(match selected {
                    "Loop" => ExportMode::Loop,
                    "Showcase" => ExportMode::Showcase,
                    _ => ExportMode::Manual,
                })
            })
        ].spacing(10).align_y(Alignment::Center);

        let selected_format = match self.exporter.format {
            ExportFormat::Gif => "GIF",
            ExportFormat::WebP => "WebP",
            ExportFormat::Avif => "AVIF",
            ExportFormat::Png => "PNG",
            ExportFormat::Mp4 => "MP4",
            ExportFormat::Mkv => "MKV",
            ExportFormat::Webm => "WebM",
            ExportFormat::Zip => "ZIP",
        };

        let format_options: Vec<&str> = FORMAT_OPTIONS.iter().copied().filter(|format| {
            match *format {
                "AVIF" => !is_avif_missing,
                "MP4" | "MKV" | "WebM" | "PNG" => !is_ffmpeg_missing,
                _ => true,
            }
        }).collect();

        let format_picker = row![
            text("Format"),
            pick_list(format_options, Some(selected_format), |selected: &str| {
                Message::SetFormat(match selected {
                    "WebP" => ExportFormat::WebP,
                    "AVIF" => ExportFormat::Avif,
                    "PNG" => ExportFormat::Png,
                    "MP4" => ExportFormat::Mp4,
                    "MKV" => ExportFormat::Mkv,
                    "WebM" => ExportFormat::Webm,
                    "ZIP" => ExportFormat::Zip,
                    _ => ExportFormat::Gif,
                })
            })
        ].spacing(10).align_y(Alignment::Center);

        let input_section: Element<'_, Message> = match self.exporter.export_mode {
            ExportMode::Manual => column![
                row![
                    text_input("Start", &self.exporter.frame_start_str).on_input(Message::SetStartFrame).width(Length::Fixed(60.0)),
                    text("~"),
                    text_input("End", &self.exporter.frame_end_str).on_input(Message::SetEndFrame).width(Length::Fixed(60.0)),
                ].spacing(5).align_y(Alignment::Center)
            ].into(),
            ExportMode::Loop => column![
                row![text("Tolerance %"), text_input("30", &self.exporter.loop_tolerance_str).on_input(Message::SetLoopTolerance).width(Length::Fixed(50.0))].spacing(5),
                row![text("Min Frames"), text_input("15", &self.exporter.loop_min_str).on_input(Message::SetLoopMin).width(Length::Fixed(50.0))].spacing(5),
                row![text("Max Frames"), text_input("None", &self.exporter.loop_max_str).on_input(Message::SetLoopMax).width(Length::Fixed(50.0))].spacing(5),
            ].spacing(5).into(),
            ExportMode::Showcase => column![
                row![text("Walk"), text_input("90", &self.exporter.showcase_walk_str).on_input(Message::SetShowcaseWalk).width(Length::Fixed(50.0))].spacing(5),
                row![text("Idle"), text_input("90", &self.exporter.showcase_idle_str).on_input(Message::SetShowcaseIdle).width(Length::Fixed(50.0))].spacing(5),
                row![text("Attack"), text_input("0", &self.exporter.showcase_attack_str).on_input(Message::SetShowcaseAttack).width(Length::Fixed(50.0))].spacing(5),
                row![text("Knockback"), text_input("60", &self.exporter.showcase_kb_str).on_input(Message::SetShowcaseKb).width(Length::Fixed(50.0))].spacing(5),
            ].spacing(5).into(),
        };

        let camera_section = column![
            text("Camera").size(18),
            row![
                text_input("X", &self.exporter.region_x.to_string()).on_input(Message::SetRegionX).width(Length::Fixed(60.0)),
                text_input("Y", &self.exporter.region_y.to_string()).on_input(Message::SetRegionY).width(Length::Fixed(60.0)),
                text_input("W", &self.exporter.region_w.to_string()).on_input(Message::SetRegionW).width(Length::Fixed(60.0)),
                text_input("H", &self.exporter.region_h.to_string()).on_input(Message::SetRegionH).width(Length::Fixed(60.0)),
            ].spacing(10)
        ].spacing(10);

        let output_section = column![
            text("Output").size(18),
            row![
                text("Name"),
                text_input("animation", &self.exporter.file_name).on_input(Message::SetFileName).width(Length::Fixed(150.0)),
            ].spacing(10).align_y(Alignment::Center),
            format_picker,
            row![
                text("Quality %"),
                text_input("100", &self.exporter.quality_percent_str).on_input(Message::SetQuality).width(Length::Fixed(50.0)),
            ].spacing(10).align_y(Alignment::Center),
            row![
                text("Compression %"),
                text_input("0", &self.exporter.compression_percent_str).on_input(Message::SetCompression).width(Length::Fixed(50.0)),
            ].spacing(10).align_y(Alignment::Center),
            row![
                checkbox(self.exporter.background).on_toggle(Message::ToggleBackground),
                text("Background")
            ].spacing(8).align_y(Alignment::Center),
        ].spacing(10);

        let status_text = self.exporter.export_result_msg.clone()
            .or_else(|| if self.exporter.is_processing { Some(format!("Encoding… {} frames", self.exporter.encoded_frames)) } else { None })
            .unwrap_or_default();

        let mut begin_button = button(text("Begin Export")).style(button::primary);
        if !is_locked {
            begin_button = begin_button.on_press(Message::BeginExport);
        }

        let buttons_row = row![
            button(text("Close")).on_press(Message::Toggle).style(button::secondary),
            begin_button,
        ].spacing(10);

        let popup_content = column![
            text("Export Animation").size(22),
            Space::new().height(Length::Fixed(10.0)),
            mode_picker,
            input_section,
            Space::new().height(Length::Fixed(10.0)),
            camera_section,
            Space::new().height(Length::Fixed(10.0)),
            output_section,
            Space::new().height(Length::Fixed(10.0)),
            text(status_text),
            Space::new().height(Length::Fixed(10.0)),
            buttons_row,
        ].spacing(10);

        container(
            scrollable(popup_content).height(Length::Fixed(420.0))
        )
            .width(Length::Fixed(320.0))
            .padding(25)
            .style(container::rounded_box)
            .into()
    }
}
