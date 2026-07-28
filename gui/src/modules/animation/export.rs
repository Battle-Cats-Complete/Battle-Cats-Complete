use std::fs;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use iced::widget::{button, checkbox, column, container, pick_list, progress_bar, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use nyanko::graphics::rig::Animation;

use core::modules::addons::paths::{self, Presence};
use core::modules::animation::export::process::{start_export, STATUS_RX};
use core::modules::animation::export::{EncoderStatus, ExportFormat, ExportMode, ExporterState};
use core::modules::animation::{IDX_ATTACK, IDX_IDLE, IDX_KB, IDX_WALK};
use core::modules::settings::Settings;

use super::data;
use super::offscreen::{self, Camera, ShowcaseLengths};
use super::overlay::Region;

const MODE_OPTIONS: [&str; 3] = ["Manual", "Loop", "Showcase"];
const FORMAT_OPTIONS: [&str; 8] = ["GIF", "WebP", "AVIF", "PNG", "MP4", "MKV", "WebM", "ZIP"];

#[derive(Default)]
pub struct State {
    pub is_open: bool,
    exporter: ExporterState,
    render_progress: Arc<AtomicI32>,
    done_at: Option<Instant>,
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
    AbortExport,
    SetCamera,
    UseBounds,
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

    pub fn set_region(&mut self, region: Region) {
        self.exporter.region_x = region.x;
        self.exporter.region_y = region.y;
        self.exporter.region_w = region.w;
        self.exporter.region_h = region.h;
        self.exporter.zoom = 1.0;
    }

    pub fn camera_region(&self) -> Option<Region> {
        if self.is_open && self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1 {
            Some(Region {
                x: self.exporter.region_x,
                y: self.exporter.region_y,
                w: self.exporter.region_w,
                h: self.exporter.region_h,
            })
        } else {
            None
        }
    }

    pub fn tick(&mut self) {
        if let Some(done) = self.done_at
            && done.elapsed().as_secs_f32() > 3.0 {
            self.done_at = None;
            self.exporter.export_result_msg = None;
        }

        if !self.exporter.is_processing {
            return;
        }

        self.exporter.current_progress = self.render_progress.load(Ordering::Relaxed);

        if let Ok(receiver_lock) = STATUS_RX.lock()
            && let Some(receiver) = receiver_lock.as_ref() {
            while let Ok(status) = receiver.try_recv() {
                match status {
                    EncoderStatus::Encoding => {}
                    EncoderStatus::Progress(progress) => self.exporter.encoded_frames = progress as i32,
                    EncoderStatus::Finished => {
                        self.exporter.is_processing = false;
                        self.exporter.tx = None;
                        self.done_at = Some(Instant::now());
                    }
                }
            }
        }
    }

    pub fn update(&mut self, message: Message, data: &data::State, settings: &Settings) {
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
                if self.exporter.is_processing {
                    return;
                }

                let Some(unit) = data.held_unit.clone() else {
                    return;
                };

                if self.exporter.region_w <= 0.1 || self.exporter.region_h <= 0.1 {
                    return;
                }

                self.exporter.region_w = self.exporter.region_w.round();
                self.exporter.region_h = self.exporter.region_h.round();
                self.exporter.export_result_msg = None;
                self.done_at = None;

                start_export(&mut self.exporter);

                let (Some(tx), Some(abort)) = (self.exporter.tx.clone(), self.exporter.abort.clone()) else {
                    return;
                };

                self.render_progress = Arc::new(AtomicI32::new(0));

                offscreen::spawn(offscreen::Job {
                    unit,
                    animation: data.current_anim.clone(),
                    available_anims: data.available_anims.clone(),
                    mode: self.exporter.export_mode.clone(),
                    frame_start: self.exporter.frame_start,
                    frame_end: self.exporter.frame_end,
                    loop_supported: self.exporter.loop_supported,
                    lengths: ShowcaseLengths {
                        walk: self.exporter.showcase_walk_len,
                        idle: self.exporter.showcase_idle_len,
                        attack: self.exporter.showcase_attack_len,
                        kb: self.exporter.showcase_kb_len,
                    },
                    camera: Camera {
                        region_x: self.exporter.region_x,
                        region_y: self.exporter.region_y,
                        zoom: self.exporter.zoom,
                    },
                    region_w: self.exporter.region_w,
                    region_h: self.exporter.region_h,
                    fps: self.exporter.fps,
                    background: self.exporter.background,
                    tx,
                    abort,
                    progress: self.render_progress.clone(),
                });
            }
            Message::AbortExport => {
                if let Some(abort) = &self.exporter.abort {
                    abort.store(true, Ordering::Relaxed);
                }

                self.exporter.export_result_msg = Some("Export Terminated!".to_string());
                self.done_at = Some(Instant::now());
                self.exporter.is_processing = false;
                self.exporter.current_progress = 0;
                self.exporter.encoded_frames = 0;
            }
            Message::SetCamera => {}
            Message::UseBounds => {
                let tolerance = if settings.animation.use_tight_bounds { 1.0 } else { 0.0 };
                let mut calculated = false;

                if let Some(unit) = &data.held_unit {
                    let mut showcase_clips = Vec::new();
                    let mut anim_refs: Vec<&Animation> = Vec::new();

                    if self.exporter.export_mode == ExportMode::Showcase {
                        for slot in [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB] {
                            if let Some((_, path)) = data.available_anims.iter().find(|(idx, _)| *idx == slot)
                                && let Ok(bytes) = fs::read(path)
                                && let Some(anim) = Animation::parse(&bytes) {
                                showcase_clips.push(anim);
                            }
                        }
                        anim_refs.extend(showcase_clips.iter());
                    } else if let Some(anim) = &data.current_anim {
                        anim_refs.push(anim.as_ref());
                    }

                    if !anim_refs.is_empty()
                        && let Some((x, y, w, h)) = unit.calculate_bounds(&anim_refs, tolerance) {
                        self.exporter.region_x = x;
                        self.exporter.region_y = y;
                        self.exporter.region_w = w;
                        self.exporter.region_h = h;
                        self.exporter.zoom = 1.0;
                        calculated = true;
                    }
                }

                if !calculated {
                    self.exporter.region_x = 0.0;
                    self.exporter.region_y = 0.0;
                    self.exporter.region_w = 0.0;
                    self.exporter.region_h = 0.0;
                    self.exporter.zoom = 1.0;
                }
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

        let camera_buttons = row![
            button(text("Set Camera")).on_press_maybe((!is_locked).then_some(Message::SetCamera)),
            button(text("Use Bounds")).on_press_maybe((!is_locked).then_some(Message::UseBounds)),
        ].spacing(10);

        let camera_section = column![
            text("Camera").size(18),
            camera_buttons,
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

        let frame_count = (self.exporter.frame_end - self.exporter.frame_start).abs() + 1;
        let rendered = self.exporter.current_progress;
        let encoded = self.exporter.encoded_frames;
        let progress_ratio = |value: i32| (value as f32 / frame_count.max(1) as f32).min(1.0);

        let (ratio, status_label) = if self.exporter.is_processing {
            if rendered < frame_count {
                let ratio = progress_ratio(rendered);
                (ratio, format!("Rendering | {}f/{}f ({}%)", rendered, frame_count, (ratio * 100.0) as i32))
            } else {
                let ratio = progress_ratio(encoded);
                (ratio, format!("Encoding | {}f/{}f ({}%)", encoded, frame_count, (ratio * 100.0) as i32))
            }
        } else if self.done_at.is_some() {
            (1.0, self.exporter.export_result_msg.clone().unwrap_or_else(|| "Done".to_string()))
        } else {
            let ratio = progress_ratio(rendered);
            if ratio > 0.0 && ratio < 1.0 {
                (ratio, format!("Paused | {}f/{}f ({}%)", rendered, frame_count, (ratio * 100.0) as i32))
            } else {
                (1.0, "Ready".to_string())
            }
        };

        let progress_section = column![
            progress_bar(0.0..=1.0, ratio),
            text(status_label).size(13),
        ].spacing(5);

        let action_button: Element<'_, Message> = if self.exporter.is_processing {
            button(text("Abort Export"))
                .style(button::danger)
                .on_press(Message::AbortExport)
                .into()
        } else {
            let is_valid = self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1;
            let is_terminated = self.done_at.is_some()
                && self.exporter.export_result_msg.as_ref().is_some_and(|message| message.contains("Terminated"));

            let label = if is_terminated {
                "Export Terminated!"
            } else if is_valid {
                "Begin Export"
            } else {
                "No Camera Set"
            };

            let style = if is_terminated { button::danger } else { button::primary };

            let mut begin = button(text(label)).style(style);
            if is_valid && !is_locked {
                begin = begin.on_press(Message::BeginExport);
            }
            begin.into()
        };

        let buttons_row = row![
            button(text("Close")).on_press(Message::Toggle).style(button::secondary),
            action_button,
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
            progress_section,
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
