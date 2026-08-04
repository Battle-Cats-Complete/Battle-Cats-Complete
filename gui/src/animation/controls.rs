use iced::widget::{button, column, container, mouse_area, opaque, row, rule, text, text_input, Button, Container, TextInput};
use iced::border::Radius;
use iced::{alignment, border, font, Background, Border, Color, Element, Length, Padding, Theme, Vector};

use core::animation::{
    IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_NONE, IDX_SPIRIT, IDX_SURFACE,
    IDX_WALK,
};

use crate::app::state::AnimState;
use crate::app::theme;

use super::{canvas, data};

const HOLD_DELAY_SECS: f32 = 0.2;
const TICK_SECS: f32 = 1.0 / 60.0;

const TILE_HEIGHT: f32 = 28.0;
const GAP: f32 = 4.0;
const ROW_GAP: f32 = 3.0;
const ICON_W: f32 = 60.0;
const NAV_W: f32 = 30.0;
const FRAME_INPUT_W: f32 = 80.0;
const RANGE_W: f32 = 60.0;
const TILDE_W: f32 = 20.0;
const COL2_W: f32 = NAV_W * 2.0 + FRAME_INPUT_W + GAP * 2.0;
const COL3_W: f32 = 100.0;
const SPEED_LABEL_W: f32 = 50.0;
const DIVIDER_W: f32 = 10.0;
const PANEL_WIDTH: f32 = ICON_W + COL2_W + COL3_W + DIVIDER_W * 2.0 + GAP * 4.0;

const ANIM_BUTTON_W: f32 = 70.0;
const ANIM_BUTTON_H: f32 = 25.0;
const GRID_GAP: f32 = 5.0;
const GRID_PAD: f32 = 2.0;

const TOGGLE_HEIGHT: f32 = 18.0;
const TOGGLE_TEXT_SIZE: f32 = 14.0;
const PLAY_TEXT_SIZE: f32 = 16.0;
const TILE_TEXT_SIZE: f32 = 14.0;
const ANIM_TEXT_SIZE: f32 = 13.0;

const PANEL_PAD: f32 = 8.0;
const PANEL_ALPHA: f32 = 160.0 / 255.0;
const PANEL_SHADE: f32 = 0.15;
const DISABLED_ALPHA: f32 = 0.5;

const PLAY_GLYPH: &str = "\u{25B6}";
const PAUSE_GLYPH: &str = "\u{25AE}\u{25AE}";

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
    HoldStart(i8),
    HoldEnd,
    HoldCancel,
    FrameInputChanged(String),
    SpeedInputChanged(String),
    RangeStartChanged(String),
    RangeEndChanged(String),
    SelectAnimation(usize),
    ResetPan,
    OpenExport,
}

fn loop_max(data: &data::State) -> Option<f32> {
    data.loop_bound().map(|v| v.max(0) as f32)
}

fn display_max(data: &data::State) -> String {
    match data.loop_bound() {
        Some(value) if value > 999_999 => "???".to_string(),
        Some(value) => value.to_string(),
        None => "???".to_string(),
    }
}

fn frame_text(canvas: &canvas::State) -> String {
    (canvas.current_frame.trunc() as i32).to_string()
}

fn frame_bounds(canvas: &canvas::State, data: &data::State) -> (f32, Option<f32>) {
    if data.active_index() == IDX_MODEL {
        return (0.0, Some(0.0));
    }

    let min = canvas.loop_start.unwrap_or(0.0).max(0.0);
    let max = canvas.loop_end.or_else(|| loop_max(data)).map(|max| max.max(min));

    (min, max)
}

pub(super) fn clamp_frame(canvas: &mut canvas::State, data: &data::State) {
    let (min, max) = frame_bounds(canvas, data);
    canvas.current_frame = max.map_or(canvas.current_frame.max(min), |max| canvas.current_frame.clamp(min, max));
}

fn effective_max(canvas: &canvas::State, data: &data::State) -> String {
    if data.active_index() == IDX_MODEL {
        return "0".to_string();
    }

    canvas.loop_end.map_or_else(|| display_max(data), |end| (end.trunc() as i32).to_string())
}

fn bold() -> font::Font {
    font::Font { weight: font::Weight::Bold, ..font::Font::DEFAULT }
}

fn panel_style(t: &Theme) -> container::Style {
    let palette = t.palette();
    let shade = |c: f32| c * PANEL_SHADE;

    container::Style {
        background: Some(Background::Color(Color {
            r: shade(palette.background.r),
            g: shade(palette.background.g),
            b: shade(palette.background.b),
            a: PANEL_ALPHA,
        })),
        border: Border {
            color: t.extended_palette().background.strong.color,
            width: 1.0,
            radius: Radius {
                top_left: theme::RADIUS_LG,
                top_right: theme::RADIUS_LG,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
        },
        ..container::Style::default()
    }
}

fn tile_style(t: &Theme) -> container::Style {
    let palette = t.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: theme::RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

fn step_style(t: &Theme, enabled: bool) -> container::Style {
    let pair = t.extended_palette().background.strong;
    let alpha = if enabled { 1.0 } else { DISABLED_ALPHA };

    container::Style {
        background: Some(Background::Color(Color { a: alpha, ..pair.color })),
        text_color: Some(Color { a: alpha, ..pair.text }),
        border: border::rounded(theme::RADIUS_SM),
        ..container::Style::default()
    }
}

fn input_style(t: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        ..text_input::default(t, status)
    }
}

fn control_style(t: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let base = theme::toggle_button(t, status, is_active);

    if status != button::Status::Disabled {
        return base;
    }

    button::Style {
        background: base.background.map(|background| match background {
            Background::Color(color) => Background::Color(Color { a: color.a * DISABLED_ALPHA, ..color }),
            other => other,
        }),
        text_color: Color { a: DISABLED_ALPHA, ..base.text_color },
        ..base
    }
}

fn tile<'a>(width: f32, content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .width(Length::Fixed(width))
        .height(Length::Fixed(TILE_HEIGHT))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(tile_style)
}

fn label_tile<'a>(width: f32, label: impl text::IntoFragment<'a>) -> Container<'a, Message> {
    tile(width, text(label).size(TILE_TEXT_SIZE))
}

fn tile_input<'a>(
    placeholder: &str,
    value: &str,
    enabled: bool,
    on_input: impl Fn(String) -> Message + 'a,
) -> TextInput<'a, Message> {
    let input = text_input(placeholder, value)
        .width(Length::Fill)
        .padding(0)
        .size(TILE_TEXT_SIZE)
        .align_x(alignment::Horizontal::Center)
        .style(input_style);

    if enabled { input.on_input(on_input) } else { input }
}

fn control_button<'a>(label: &'a str, size: f32, width: f32, on_press: Option<Message>) -> Button<'a, Message> {
    button(text(label).size(size).width(Length::Fill).height(Length::Fill).center())
        .width(Length::Fixed(width))
        .height(Length::Fixed(TILE_HEIGHT))
        .padding(0)
        .on_press_maybe(on_press)
        .style(|t: &Theme, status| control_style(t, status, false))
}

fn step_control<'a>(label: &'a str, direction: i8, enabled: bool) -> Element<'a, Message> {
    let face = container(text(label).size(TILE_TEXT_SIZE))
        .width(Length::Fixed(NAV_W))
        .height(Length::Fixed(TILE_HEIGHT))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |t: &Theme| step_style(t, enabled));

    if !enabled {
        return face.into();
    }

    mouse_area(face)
        .on_press(Message::HoldStart(direction))
        .on_release(Message::HoldEnd)
        .on_exit(Message::HoldCancel)
        .into()
}

fn divider<'a>() -> Container<'a, Message> {
    container(rule::vertical(1))
        .width(Length::Fixed(DIVIDER_W))
        .height(Length::Fixed(TILE_HEIGHT * 2.0 + GAP))
        .align_x(alignment::Horizontal::Center)
}

fn anim_grid(data: &data::State) -> Element<'_, Message> {
    let base_available = data.base_assets_available();
    let secondary_available = data.secondary_available();

    let mut grid = column![].spacing(GRID_GAP);

    for chunk in ANIM_BUTTONS.chunks(4) {
        let mut buttons = row![].spacing(GRID_GAP);

        for (label, index) in chunk {
            let available = match *index {
                IDX_SPIRIT => secondary_available,
                IDX_MODEL => base_available,
                idx => base_available && data.available_anims.iter().any(|(a, _)| *a == idx),
            };
            let is_active = data.loaded_anim_index == *index && data.loaded_anim_index != IDX_NONE;

            buttons = buttons.push(
                button(text(*label).size(ANIM_TEXT_SIZE).width(Length::Fill).height(Length::Fill).center())
                    .width(Length::Fixed(ANIM_BUTTON_W))
                    .height(Length::Fixed(ANIM_BUTTON_H))
                    .padding(0)
                    .on_press_maybe(available.then_some(Message::SelectAnimation(*index)))
                    .style(move |t: &Theme, status| control_style(t, status, is_active)),
            );
        }

        grid = grid.push(buttons);
    }

    grid.into()
}

impl State {
    pub fn update(&mut self, message: Message, canvas: &mut canvas::State, data: &mut data::State, anim_state: &mut AnimState) {
        match message {
            Message::TogglePlay => canvas.is_playing = !canvas.is_playing,
            Message::ToggleExpanded => anim_state.controls_expanded = !anim_state.controls_expanded,
            Message::HoldStart(dir) => {
                self.hold_dir = dir;
                self.hold_timer = 0.0;
            }
            Message::HoldEnd => {
                if self.hold_dir != 0 && self.hold_timer <= HOLD_DELAY_SECS {
                    self.step(canvas, data, self.hold_dir as f32);
                }
                self.hold_dir = 0;
                self.hold_timer = 0.0;
            }
            Message::HoldCancel => {
                self.hold_dir = 0;
                self.hold_timer = 0.0;
            }
            Message::FrameInputChanged(value) => {
                let trimmed = value.trim();
                let target = if trimmed.is_empty() { Some(0.0) } else { trimmed.parse::<f32>().ok() };

                if let Some(target) = target {
                    canvas.current_frame = target;
                    clamp_frame(canvas, data);
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
                    let start = parsed.max(0.0);
                    canvas.loop_start = Some(start);
                    if canvas.current_frame < start {
                        canvas.current_frame = start;
                    }
                }
                self.range_start_input = value;
            }
            Message::RangeEndChanged(value) => {
                if value.is_empty() {
                    canvas.loop_end = None;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    let end = parsed.max(0.0);
                    canvas.loop_end = Some(end);
                    if canvas.current_frame > end {
                        canvas.current_frame = end;
                    }
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
        let (min, max) = frame_bounds(canvas, data);
        let stepped = canvas.current_frame + direction;

        canvas.current_frame = if stepped < min {
            max.unwrap_or(min)
        } else if max.is_some_and(|max| stepped > max) {
            min
        } else {
            stepped
        };
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

    pub fn view<'a>(&'a self, canvas: &'a canvas::State, data: &'a data::State, anim_state: &AnimState) -> Element<'a, Message> {
        let is_expanded = anim_state.controls_expanded;
        let expand_icon = if is_expanded { "▼" } else { "▲" };

        let toggle = button(
            text(expand_icon)
                .size(TOGGLE_TEXT_SIZE)
                .font(bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .center(),
        )
        .width(Length::Fill)
        .height(Length::Fixed(TOGGLE_HEIGHT))
        .padding(0)
        .on_press(Message::ToggleExpanded)
        .style(button::text);

        let mut panel = column![toggle].spacing(ROW_GAP).width(Length::Fixed(PANEL_WIDTH));

        if is_expanded {
            panel = panel
                .push(rule::horizontal(1))
                .push(
                    container(anim_grid(data))
                        .width(Length::Fill)
                        .padding(Padding { top: GRID_PAD, right: 0.0, bottom: GRID_PAD, left: 0.0 })
                        .align_x(alignment::Horizontal::Center),
                )
                .push(rule::horizontal(1))
                .push(
                    container(self.transport_row(canvas, data))
                        .padding(Padding { top: GRID_PAD, right: 0.0, bottom: 0.0, left: 0.0 }),
                );
        }

        opaque(container(panel).padding(PANEL_PAD).style(panel_style))
    }

    fn transport_row<'a>(&'a self, canvas: &'a canvas::State, data: &'a data::State) -> Element<'a, Message> {
        let base_available = data.base_assets_available();
        let anim_loaded = base_available && data.loaded_anim_index != IDX_NONE;

        let play_icon = if canvas.is_playing { PAUSE_GLYPH } else { PLAY_GLYPH };

        let transport = column![
            control_button(play_icon, PLAY_TEXT_SIZE, ICON_W, anim_loaded.then_some(Message::TogglePlay)),
            control_button("Orient", TILE_TEXT_SIZE, ICON_W, base_available.then_some(Message::ResetPan)),
        ]
        .spacing(GAP);

        let frame_row: Element<'a, Message> = if canvas.is_playing {
            row![
                tile(RANGE_W, tile_input("0", &self.range_start_input, anim_loaded, Message::RangeStartChanged)),
                label_tile(TILDE_W, "~"),
                tile(RANGE_W, tile_input(&display_max(data), &self.range_end_input, anim_loaded, Message::RangeEndChanged)),
            ]
            .spacing(GAP)
            .into()
        } else {
            let frame_input = tile_input("0", &frame_text(canvas), anim_loaded, Message::FrameInputChanged);

            row![
                step_control("◀", -1, anim_loaded),
                tile(FRAME_INPUT_W, frame_input),
                step_control("▶", 1, anim_loaded),
            ]
            .spacing(GAP)
            .into()
        };

        let counter_row = row![
            label_tile(RANGE_W, frame_text(canvas)),
            label_tile(TILDE_W, "/"),
            label_tile(RANGE_W, effective_max(canvas, data)),
        ]
        .spacing(GAP);

        let playback = column![frame_row, counter_row].spacing(GAP);

        let speed_row = row![
            label_tile(SPEED_LABEL_W, "Speed"),
            tile(
                COL3_W - SPEED_LABEL_W - GAP,
                tile_input("1.0", &self.speed_input, base_available, Message::SpeedInputChanged),
            ),
        ]
        .spacing(GAP);

        let output = column![
            control_button("Export", TILE_TEXT_SIZE, COL3_W, base_available.then_some(Message::OpenExport)),
            speed_row,
        ]
        .spacing(GAP);

        row![transport, divider(), playback, divider(), output]
            .spacing(GAP)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}
