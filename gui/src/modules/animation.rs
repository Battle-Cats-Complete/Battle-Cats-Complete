use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input, Space,
};
use iced::{Alignment, Element, Length, Task};
use tracing::info;

use core::modules::animation::export::{ExportFormat, ExportMode};
use core::modules::animation::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT, IDX_SURFACE,
    IDX_WALK,
};

#[derive(Debug, Clone)]
pub struct State {
    pub is_playing: bool,
    pub current_frame: f32,
    pub playback_speed: f32,
    pub speed_input: String,
    pub frame_input: String,
    pub controls_expanded: bool,
    pub active_animation_index: usize,

    pub export_popup_open: bool,
    pub export_mode: ExportMode,
    pub export_format: ExportFormat,
    pub export_name: String,

    pub loop_tolerance: String,
    pub loop_minimum: String,
    pub loop_maximum: String,

    pub showcase_walk: String,
    pub showcase_idle: String,
    pub showcase_attack: String,
    pub showcase_kb: String,

    pub export_start_frame: String,
    pub export_end_frame: String,

    pub region_x: String,
    pub region_y: String,
    pub region_w: String,
    pub region_h: String,

    pub quality_percent: String,
    pub compression_percent: String,
    pub background_enabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_playing: true,
            current_frame: 0.0,
            playback_speed: 1.0,
            speed_input: String::from("1.0"),
            frame_input: String::from("0"),
            controls_expanded: true,
            active_animation_index: IDX_NONE,

            export_popup_open: false,
            export_mode: ExportMode::Manual,
            export_format: ExportFormat::Gif,
            export_name: String::new(),

            loop_tolerance: String::from("30"),
            loop_minimum: String::from("15"),
            loop_maximum: String::new(),

            showcase_walk: String::new(),
            showcase_idle: String::new(),
            showcase_attack: String::new(),
            showcase_kb: String::new(),

            export_start_frame: String::from("0"),
            export_end_frame: String::from("100"),

            region_x: String::from("0"),
            region_y: String::from("0"),
            region_w: String::from("0"),
            region_h: String::from("0"),

            quality_percent: String::from("100"),
            compression_percent: String::from("0"),
            background_enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TogglePlay,
    ToggleControlsExpanded,
    SetFrameInput(String),
    ApplyFrameInput,
    SetSpeedInput(String),
    SelectAnimation(usize),

    ToggleExportPopup,
    SetExportMode(ExportMode),
    SetExportFormat(ExportFormat),
    SetExportName(String),

    SetExportStartFrame(String),
    SetExportEndFrame(String),
    SetLoopTolerance(String),
    SetLoopMinimum(String),
    SetLoopMaximum(String),

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

    TriggerExport,
}

impl State {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.is_playing {
                    self.current_frame += 0.5 * self.playback_speed;
                    self.frame_input = (self.current_frame.trunc() as i32).to_string();
                }
            }
            Message::TogglePlay => {
                self.is_playing = !self.is_playing;
            }
            Message::ToggleControlsExpanded => {
                self.controls_expanded = !self.controls_expanded;
            }
            Message::SetFrameInput(val) => {
                self.frame_input = val;
            }
            Message::ApplyFrameInput => {
                if let Ok(parsed_frame) = self.frame_input.parse::<f32>() {
                    self.current_frame = parsed_frame;
                }
            }
            Message::SetSpeedInput(val) => {
                self.speed_input = val.clone();
                if let Ok(parsed_speed) = val.parse::<f32>() {
                    self.playback_speed = parsed_speed;
                }
            }
            Message::SelectAnimation(idx) => {
                self.active_animation_index = idx;
                info!("Selected animation index: {}", idx);
            }
            Message::ToggleExportPopup => {
                self.export_popup_open = !self.export_popup_open;
            }
            Message::SetExportMode(mode) => {
                self.export_mode = mode;
            }
            Message::SetExportFormat(format) => {
                self.export_format = format;
            }
            Message::SetExportName(name) => {
                self.export_name = name;
            }
            Message::SetExportStartFrame(val) => {
                self.export_start_frame = val;
            }
            Message::SetExportEndFrame(val) => {
                self.export_end_frame = val;
            }
            Message::SetLoopTolerance(val) => {
                self.loop_tolerance = val;
            }
            Message::SetLoopMinimum(val) => {
                self.loop_minimum = val;
            }
            Message::SetLoopMaximum(val) => {
                self.loop_maximum = val;
            }
            Message::SetShowcaseWalk(val) => {
                self.showcase_walk = val;
            }
            Message::SetShowcaseIdle(val) => {
                self.showcase_idle = val;
            }
            Message::SetShowcaseAttack(val) => {
                self.showcase_attack = val;
            }
            Message::SetShowcaseKb(val) => {
                self.showcase_kb = val;
            }
            Message::SetRegionX(val) => {
                self.region_x = val;
            }
            Message::SetRegionY(val) => {
                self.region_y = val;
            }
            Message::SetRegionW(val) => {
                self.region_w = val;
            }
            Message::SetRegionH(val) => {
                self.region_h = val;
            }
            Message::SetQuality(val) => {
                self.quality_percent = val;
            }
            Message::SetCompression(val) => {
                self.compression_percent = val;
            }
            Message::ToggleBackground(enabled) => {
                self.background_enabled = enabled;
            }
            Message::TriggerExport => {
                info!("Triggering export workflow");
                // TODO: Core data flow integration goes here in Phase 2
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let canvas_layer = container(text("OpenGL Canvas Placeholder"))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::dark);

        let controls_ui = self.view_controls();
        let controls_layer = container(controls_ui)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_bottom(Length::Fill)
            .padding(10);

        let mut layered_stack = stack![canvas_layer, controls_layer];

        if self.export_popup_open {
            let export_modal = container(self.view_export_popup())
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            layered_stack = layered_stack.push(export_modal);
        }

        layered_stack.into()
    }

    fn view_controls(&self) -> Element<Message> {
        let play_icon = if self.is_playing { "⏸" } else { "▶" };
        let expand_icon = if self.controls_expanded { "▼" } else { "▲" };

        let primary_controls = row![
            button(text(play_icon)).on_press(Message::TogglePlay),
            text_input("Frame", &self.frame_input)
                .on_input(Message::SetFrameInput)
                .on_submit(Message::ApplyFrameInput)
                .width(Length::Fixed(80.0)),
            text_input("Speed", &self.speed_input)
                .on_input(Message::SetSpeedInput)
                .width(Length::Fixed(60.0)),
            button(text("Export")).on_press(Message::ToggleExportPopup).style(button::primary),
        ]
            .spacing(10)
            .align_y(Alignment::Center);

        let animation_grid = if self.controls_expanded {
            column![
                row![
                    self.anim_button("Walk", IDX_WALK),
                    self.anim_button("Idle", IDX_IDLE),
                    self.anim_button("Attack", IDX_ATTACK),
                    self.anim_button("Knockback", IDX_KB),
                ].spacing(5),
                row![
                    self.anim_button("Burrow", IDX_BURROW),
                    self.anim_button("Surface", IDX_SURFACE),
                    self.anim_button("Spirit", IDX_SPIRIT),
                    self.anim_button("Model", IDX_MODEL),
                ].spacing(5)
            ].spacing(5)
        } else {
            column![]
        };

        container(
            column![
                primary_controls,
                animation_grid,
                button(text(expand_icon))
                    .on_press(Message::ToggleControlsExpanded)
                    .width(Length::Fill)
                    .style(button::text),
            ]
                .spacing(10)
        )
            .padding(15)
            .style(container::rounded_box)
            .into()
    }

    fn anim_button(&self, label: &'static str, index: usize) -> Element<Message> {
        let mut btn = button(text(label)).on_press(Message::SelectAnimation(index)).width(Length::Fixed(80.0));

        if self.active_animation_index == index {
            btn = btn.style(button::primary);
        } else {
            btn = btn.style(button::secondary);
        }

        btn.into()
    }

    fn view_export_popup(&self) -> Element<Message> {
        let selected_mode_str = match self.export_mode {
            ExportMode::Manual => "Manual",
            ExportMode::Loop => "Loop",
            ExportMode::Showcase => "Showcase",
        };

        let mode_picker = row![
            text("Mode"),
            pick_list(
                &["Manual", "Loop", "Showcase"][..],
                Some(selected_mode_str),
                |selected: &str| {
                    Message::SetExportMode(match selected {
                        "Loop" => ExportMode::Loop,
                        "Showcase" => ExportMode::Showcase,
                        _ => ExportMode::Manual,
                    })
                }
            )
        ].spacing(10).align_y(Alignment::Center);

        let selected_format_str = match self.export_format {
            ExportFormat::Gif => "GIF",
            ExportFormat::WebP => "WebP",
            ExportFormat::Avif => "AVIF",
            ExportFormat::Png => "PNG",
            ExportFormat::Mp4 => "MP4",
            ExportFormat::Mkv => "MKV",
            ExportFormat::Webm => "WebM",
            ExportFormat::Zip => "ZIP",
        };

        let format_picker = row![
            text("Format"),
            pick_list(
                &["GIF", "WebP", "AVIF", "PNG", "MP4", "MKV", "WebM", "ZIP"][..],
                Some(selected_format_str),
                |selected: &str| {
                    Message::SetExportFormat(match selected {
                        "WebP" => ExportFormat::WebP,
                        "AVIF" => ExportFormat::Avif,
                        "PNG" => ExportFormat::Png,
                        "MP4" => ExportFormat::Mp4,
                        "MKV" => ExportFormat::Mkv,
                        "WebM" => ExportFormat::Webm,
                        "ZIP" => ExportFormat::Zip,
                        _ => ExportFormat::Gif,
                    })
                }
            )
        ].spacing(10).align_y(Alignment::Center);

        let input_section = match self.export_mode {
            ExportMode::Manual => column![
                row![
                    text_input("Start", &self.export_start_frame).on_input(Message::SetExportStartFrame).width(Length::Fixed(50.0)),
                    text("~"),
                    text_input("End", &self.export_end_frame).on_input(Message::SetExportEndFrame).width(Length::Fixed(50.0)),
                ].spacing(5).align_y(Alignment::Center)
            ].spacing(5),
            ExportMode::Loop => column![
                row![text("Tolerance %"), text_input("30", &self.loop_tolerance).on_input(Message::SetLoopTolerance).width(Length::Fixed(50.0))].spacing(5),
                row![text("Min Frames"), text_input("15", &self.loop_minimum).on_input(Message::SetLoopMinimum).width(Length::Fixed(50.0))].spacing(5),
                row![text("Max Frames"), text_input("None", &self.loop_maximum).on_input(Message::SetLoopMaximum).width(Length::Fixed(50.0))].spacing(5),
            ].spacing(5),
            ExportMode::Showcase => column![
                row![text("Walk"), text_input("Walk", &self.showcase_walk).on_input(Message::SetShowcaseWalk).width(Length::Fixed(50.0))].spacing(5),
                row![text("Idle"), text_input("Idle", &self.showcase_idle).on_input(Message::SetShowcaseIdle).width(Length::Fixed(50.0))].spacing(5),
                row![text("Attack"), text_input("Attack", &self.showcase_attack).on_input(Message::SetShowcaseAttack).width(Length::Fixed(50.0))].spacing(5),
                row![text("Knockback"), text_input("KB", &self.showcase_kb).on_input(Message::SetShowcaseKb).width(Length::Fixed(50.0))].spacing(5),
            ].spacing(5),
        };

        let camera_section = column![
            text("Camera").size(18),
            row![
                text_input("X", &self.region_x).on_input(Message::SetRegionX).width(Length::Fixed(60.0)),
                text_input("Y", &self.region_y).on_input(Message::SetRegionY).width(Length::Fixed(60.0)),
                text_input("W", &self.region_w).on_input(Message::SetRegionW).width(Length::Fixed(60.0)),
                text_input("H", &self.region_h).on_input(Message::SetRegionH).width(Length::Fixed(60.0)),
            ].spacing(10)
        ].spacing(10);

        let output_section = column![
            text("Output").size(18),
            row![
                text("Name"),
                text_input("animation", &self.export_name).on_input(Message::SetExportName).width(Length::Fixed(150.0)),
            ].spacing(10).align_y(Alignment::Center),
            format_picker,
            row![
                text("Quality %"),
                text_input("100", &self.quality_percent).on_input(Message::SetQuality).width(Length::Fixed(50.0)),
            ].spacing(10).align_y(Alignment::Center),
            row![
                text("Compression %"),
                text_input("0", &self.compression_percent).on_input(Message::SetCompression).width(Length::Fixed(50.0)),
            ].spacing(10).align_y(Alignment::Center),
            row![
                checkbox(self.background_enabled).on_toggle(Message::ToggleBackground),
                text("Background")
            ].spacing(8).align_y(Alignment::Center),
        ].spacing(10);

        let buttons_row = row![
            button(text("Close")).on_press(Message::ToggleExportPopup).style(button::secondary),
            button(text("Begin Export")).on_press(Message::TriggerExport).style(button::primary),
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
            Space::new().height(Length::Fixed(20.0)),
            buttons_row,
        ].spacing(10);

        container(
            scrollable(popup_content)
                .height(Length::Fixed(400.0))
        )
            .padding(25)
            .style(container::rounded_box)
            .into()
    }
}