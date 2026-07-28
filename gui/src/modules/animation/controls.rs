use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Vector};

use core::modules::animation::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT, IDX_SURFACE,
    IDX_WALK,
};

use super::{canvas, data};

const HOLD_DELAY_SECS: f32 = 0.2;
const TICK_SECS: f32 = 1.0 / 60.0;

const ANIM_BUTTONS: [(&str, usize); 8] = [
    ("Walk", IDX_WALK),
    ("Idle", IDX_IDLE),
    ("Attack", IDX_ATTACK),
    ("Knockback", IDX_KB),
    ("Burrow", IDX_BURROW),
    ("Surface", IDX_SURFACE),
    ("Spirit", IDX_SPIRIT),
    ("Model", IDX_MODEL),
];

#[derive(Default)]
pub struct State {
    pub controls_expanded: bool,
    frame_input: String,
    speed_input: String,
    range_start_input: String,
    range_end_input: String,
    hold_dir: i8,
    hold_timer: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePlay,
    ToggleExpanded,
    StepPrev,
    StepNext,
    HoldStart(i8),
    HoldEnd,
    FrameInputChanged(String),
    FrameInputSubmitted,
    SpeedInputChanged(String),
    RangeStartChanged(String),
    RangeEndChanged(String),
    SelectAnimation(usize),
    ResetPan,
    OpenExport,
}

fn loop_max(data: &data::State) -> Option<f32> {
    data.current_anim.as_ref().map(|anim| anim.max_frame.max(0) as f32)
}

impl State {
    pub fn update(&mut self, message: Message, canvas: &mut canvas::State, data: &mut data::State) {
        match message {
            Message::TogglePlay => canvas.is_playing = !canvas.is_playing,
            Message::ToggleExpanded => self.controls_expanded = !self.controls_expanded,
            Message::StepPrev => self.step(canvas, data, -1.0),
            Message::StepNext => self.step(canvas, data, 1.0),
            Message::HoldStart(dir) => {
                self.hold_dir = dir;
                self.hold_timer = 0.0;
            }
            Message::HoldEnd => {
                self.hold_dir = 0;
                self.hold_timer = 0.0;
            }
            Message::FrameInputChanged(value) => self.frame_input = value,
            Message::FrameInputSubmitted => {
                if let Ok(parsed) = self.frame_input.parse::<f32>() {
                    canvas.current_frame = parsed.max(0.0);
                }
            }
            Message::SpeedInputChanged(value) => {
                self.speed_input = value.clone();
                if value.is_empty() {
                    canvas.playback_speed = 1.0;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    canvas.playback_speed = parsed;
                }
            }
            Message::RangeStartChanged(value) => {
                if value.is_empty() {
                    canvas.loop_start = None;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    canvas.loop_start = Some(parsed.max(0.0));
                }
                self.range_start_input = value;
            }
            Message::RangeEndChanged(value) => {
                if value.is_empty() {
                    canvas.loop_end = None;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    canvas.loop_end = Some(parsed.max(0.0));
                }
                self.range_end_input = value;
            }
            Message::SelectAnimation(index) => {
                data.select(index);
                canvas.current_frame = 0.0;
            }
            Message::ResetPan => canvas.pan = Vector::new(0.0, 0.0),
            Message::OpenExport => {}
        }
    }

    fn step(&mut self, canvas: &mut canvas::State, data: &data::State, direction: f32) {
        let max = loop_max(data);
        let mut new_frame = canvas.current_frame + direction;

        match max {
            Some(max) if new_frame > max => new_frame = 0.0,
            Some(max) if new_frame < 0.0 => new_frame = max,
            None if new_frame < 0.0 => new_frame = 0.0,
            _ => {}
        }

        canvas.current_frame = new_frame;
    }

    pub fn tick(&mut self, canvas: &mut canvas::State, data: &data::State) {
        if self.hold_dir == 0 {
            self.hold_timer = 0.0;
            return;
        }

        self.hold_timer += TICK_SECS;
        if self.hold_timer <= HOLD_DELAY_SECS {
            return;
        }

        let speed_factor = if canvas.playback_speed.abs() < 0.05 { 1.0 } else { canvas.playback_speed.abs() };
        let delta = self.hold_dir as f32 * TICK_SECS * 30.0 * speed_factor;
        self.step(canvas, data, delta);
    }

    pub fn view<'a>(&'a self, canvas: &'a canvas::State, data: &'a data::State) -> Element<'a, Message> {
        let play_icon = if canvas.is_playing { "⏸" } else { "▶" };
        let is_locked = false;

        let transport_row = row![
            button(text(play_icon).size(16))
                .on_press(Message::TogglePlay)
                .width(Length::Fixed(50.0)),
            button(text("Orient")).on_press(Message::ResetPan).width(Length::Fixed(60.0)),
            button(text("Export")).on_press(Message::OpenExport).width(Length::Fixed(70.0)).style(button::primary),
        ].spacing(4);

        let frame_display = if canvas.is_playing {
            let start_hint = canvas.loop_start.map(|v| v.to_string()).unwrap_or_default();
            let end_hint = canvas.loop_end.map(|v| v.to_string()).unwrap_or_default();
            row![
                text_input(&start_hint, &self.range_start_input).on_input(Message::RangeStartChanged).width(Length::Fixed(60.0)),
                text("~"),
                text_input(&end_hint, &self.range_end_input).on_input(Message::RangeEndChanged).width(Length::Fixed(60.0)),
            ].spacing(4)
        } else {
            row![
                button(text("◀"))
                    .on_press(Message::StepPrev)
                    .width(Length::Fixed(30.0)),
                text_input("0", &self.frame_input)
                    .on_input(Message::FrameInputChanged)
                    .on_submit(Message::FrameInputSubmitted)
                    .width(Length::Fixed(80.0)),
                button(text("▶"))
                    .on_press(Message::StepNext)
                    .width(Length::Fixed(30.0)),
            ].spacing(4)
        };

        let frame_status = row![
            text(format!("{:.0}", canvas.current_frame)),
            text("/"),
            text(loop_max(data).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "???".to_string())),
        ].spacing(4);

        let speed_row = row![
            text("Speed"),
            text_input("1.0", &self.speed_input).on_input(Message::SpeedInputChanged).width(Length::Fixed(50.0)),
        ].spacing(4);

        let expand_icon = if self.controls_expanded { "▼" } else { "▲" };
        let toggle_button = button(text(expand_icon)).on_press(Message::ToggleExpanded).width(Length::Fill).style(button::text);

        let body: Element<'_, Message> = if self.controls_expanded {
            let controls_column = column![transport_row, frame_display, frame_status, speed_row].spacing(6);

            let base_available = data.base_assets_available();
            let secondary_available = data.secondary_available();

            let mut rows: Vec<Element<'_, Message>> = Vec::new();
            for chunk in ANIM_BUTTONS.chunks(4) {
                let mut buttons_row = row![].spacing(4);
                for (label, index) in chunk {
                    let available = match *index {
                        IDX_SPIRIT => secondary_available,
                        IDX_MODEL => base_available,
                        idx => base_available && data.available_anims.iter().any(|(a, _)| *a == idx),
                    };
                    let is_active = data.loaded_anim_index == *index && data.loaded_anim_index != IDX_NONE;

                    let mut btn = button(text(*label).size(13))
                        .width(Length::Fixed(70.0));
                    btn = if is_active { btn.style(button::primary) } else { btn.style(button::secondary) };
                    if available && !is_locked {
                        btn = btn.on_press(Message::SelectAnimation(*index));
                    }
                    buttons_row = buttons_row.push(btn);
                }
                rows.push(buttons_row.into());
            }
            let grid = column(rows).spacing(4);

            column![controls_column, grid].spacing(8).into()
        } else {
            column![].into()
        };

        container(
            column![body, toggle_button].spacing(8)
        )
            .padding(10)
            .style(container::rounded_box)
            .into()
    }
}
