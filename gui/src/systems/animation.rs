mod canvas;
pub(crate) mod controls;
mod data;
mod diagnostics;
mod export;
mod expand;
mod offscreen;
pub(crate) mod overlay;
mod pipeline;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{button, column, container, stack, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Size, Task, Theme};

use nyanko::graphics::rig::{Animation, Model, Rig};

use kore::domains::settings::Settings;
use kore::systems::animation::ClipSet;

use crate::app::state::AnimState;
use crate::app::theme;
use crate::editor;
use crate::widget::{smooth_scroll, toggle_row};

const FRAME_BORDER_WIDTH: f32 = 4.0;
const FRAME_BORDER_RADIUS: f32 = 5.0;
const EMPTY_BACKGROUND_SHADE: f32 = 0.6;

const EXPAND_BUTTON_SIZE: f32 = 30.0;
const EXPAND_BUTTON_INSET: f32 = 8.0;

const CONTROLS_INSET_LEFT: f32 = 7.0;

const ZOOM_SCROLL_STRENGTH: f32 = 2.5;
const CULL_SCALE: f32 = 100.0;

const DEBUG_LABEL_SIZE: f32 = 12.0;
const DEBUG_TOGGLE_GAP: f32 = 6.0;

const MISSING_FILES_NOTICE: &str = "Missing essential files needed to load Entity";

pub fn debug_toggles<'a, M: Clone + 'a>(
    settings: &Settings,
    origin: impl Fn(bool) -> M + 'a,
    parts: impl Fn(bool) -> M + 'a,
    world: impl Fn(bool) -> M + 'a,
) -> Element<'a, M> {
    column![
        toggle_row(settings.animation.show_rig, text("Show Rig").size(DEBUG_LABEL_SIZE), Some(parts)),
        toggle_row(settings.animation.show_origin, text("Show Origin").size(DEBUG_LABEL_SIZE), Some(origin)),
        toggle_row(settings.animation.show_world, text("Show World").size(DEBUG_LABEL_SIZE), Some(world)),
    ]
    .spacing(DEBUG_TOGGLE_GAP)
    .into()
}

fn frame_border<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|t: &Theme| container::Style {
            border: Border {
                color: t.palette().primary,
                width: FRAME_BORDER_WIDTH,
                radius: FRAME_BORDER_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

#[derive(Clone, Copy)]
pub struct Action {
    pub label: &'static str,
    pub danger: bool,
    pub enabled: bool,
}

pub struct State {
    data: data::State,
    canvas: canvas::State,
    controls: controls::State,
    export: export::State,
    overlay: overlay::State,
    export_open: bool,
    highlight: Option<usize>,
    action: Option<Action>,
    is_expanded: bool,
    playhead_rig: String,
    playhead_clip: Option<usize>,
    playhead_reset: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(canvas::Message),
    Controls(controls::Message),
    Export(export::Message),
    Overlay(overlay::Message),
    Preloaded(data::PreloadResult),
    ToggleExpanded,
}

impl State {
    pub fn with_popup(kind: crate::widget::popup::Kind) -> Self {
        Self {
            data: data::State::default(),
            canvas: canvas::State::default(),
            controls: controls::State::default(),
            export: export::State::new(kind),
            overlay: overlay::State::default(),
            export_open: false,
            highlight: None,
            action: None,
            is_expanded: false,
            playhead_rig: String::new(),
            playhead_clip: None,
            playhead_reset: false,
        }
    }

    pub fn sync(&mut self, key: &str, build: impl FnOnce() -> ClipSet, settings: &Settings, anim_state: &AnimState) {
        self.data.restore_offset(anim_state.offset_row);
        self.data.sync(key, build);
        self.data.measure(settings.animation.bounds_cull as f32 / CULL_SCALE);
        self.export.sync(&self.data, settings, anim_state);
        self.sync_playhead();
    }

    pub fn reset_playhead(&mut self) {
        self.playhead_reset = true;
    }

    pub fn clear(&mut self) {
        self.data.reset_display();
        self.playhead_rig.clear();
        self.playhead_clip = None;
    }

    pub fn is_model_selected(&self) -> bool {
        self.data.is_model()
    }

    pub fn selected_anim(&self) -> Option<&PathBuf> {
        self.data.current_clip().and_then(|clip| clip.anim.as_ref())
    }

    fn sync_playhead(&mut self) {
        let (rig, clip) = self.data.playhead_key();

        if self.playhead_rig == rig && self.playhead_clip == clip {
            return;
        }

        self.playhead_rig = rig.to_string();
        self.playhead_clip = clip;

        if std::mem::take(&mut self.playhead_reset) {
            self.canvas.current_frame = 0.0;
        }

        controls::clamp_frame(&mut self.canvas, &self.data);
    }

    pub fn preload(&mut self, key: &str, build: impl FnOnce() -> ClipSet, anim_state: &AnimState) -> Task<Message> {
        self.data.restore_offset(anim_state.offset_row);
        Self::preload_task(self.data.preload_request(key, build))
    }

    fn preload_task(request: Option<data::PreloadRequest>) -> Task<Message> {
        request.map_or_else(Task::none, |request| Task::perform(smol::unblock(move || request.run()), Message::Preloaded))
    }

    pub fn invalidate_paths(&mut self) {
        self.data.invalidate_paths();
    }

    pub fn adopt_anim(&mut self, path: &Path, anim: Arc<Animation>) {
        self.data.adopt_anim(path, anim);
    }

    pub fn selected_model(&self) -> Option<&Path> {
        self.data.selected_model()
    }

    pub fn anim_paths(&self) -> Vec<PathBuf> {
        self.data.anim_paths()
    }

    pub fn selected_sheet(&self) -> Option<&Path> {
        self.data.selected_sheet()
    }

    pub fn selected_cuts(&self) -> Option<&Path> {
        self.data.selected_cuts()
    }

    pub fn adopt_model(&mut self, model: Arc<Model>) {
        self.data.adopt_model(model);
    }

    pub fn rig(&self) -> Option<&Rig> {
        self.data.held_unit.as_deref()
    }

    pub fn frame(&self) -> i32 {
        self.data.playback_frame(self.canvas.current_frame).floor() as i32
    }

    pub fn seek(&mut self, frame: f32) {
        self.canvas.is_playing = false;
        self.canvas.current_frame = frame.max(0.0);
    }

    pub fn bound(&mut self, start: f32, end: f32) {
        let (start, end) = (start.max(0.0), end.max(0.0));

        self.canvas.loop_start = Some(start.min(end));
        self.canvas.loop_end = Some(start.max(end));
        self.controls.set_range(start.min(end), start.max(end));

        controls::clamp_frame(&mut self.canvas, &self.data);
    }

    pub fn set_action(&mut self, action: Option<Action>) {
        self.action = action;
    }

    pub fn playing(&self) -> bool {
        self.canvas.is_playing
    }

    pub fn locatable(&self) -> bool {
        self.highlight.is_some()
    }

    pub fn locate(&mut self) {
        let Some(part) = self.highlight else {
            return;
        };

        let Some(anchor) = diagnostics::anchor(&self.data, self.canvas.current_frame, part) else {
            return;
        };

        self.canvas.pan = iced::Vector::new(-anchor.x, -anchor.y);
    }

    pub fn set_highlight(&mut self, part: Option<usize>) {
        self.highlight = part;
    }

    pub fn resolved(&self) -> bool {
        self.data.resolved()
    }

    pub fn selected_label(&self) -> Option<String> {
        self.data.selected_label()
    }

    pub fn select_label(&mut self, label: &str) {
        self.data.select_label(label);
        self.sync_playhead();
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    pub fn tick(&mut self) {
        self.canvas.update(canvas::Message::Tick, &self.data);
        self.controls.tick(&mut self.canvas, &self.data);
        self.export.tick();
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, anim_state: &mut AnimState) -> Task<Message> {
        match message {
            Message::Canvas(msg) => {
                self.canvas.update(msg, &self.data);
                Task::none()
            }
            Message::Controls(controls::Message::OpenExport) => {
                let was_open = self.export_open;
                self.export_open = true;
                if settings.animation.auto_set_camera_region && !was_open && !self.overlay.selecting {
                    return self
                        .export
                        .update(export::Message::UseBounds, &self.data, settings, anim_state, &mut self.export_open)
                        .map(Message::Export);
                }
                Task::none()
            }
            Message::Controls(msg) => {
                self.controls.update(msg, &mut self.canvas, &mut self.data, anim_state);
                self.sync_playhead();
                Task::none()
            }
            Message::Export(export::Message::SetCamera) => {
                self.export_open = false;
                self.overlay.selecting = true;
                Task::none()
            }
            Message::Export(msg) => {
                self.export.update(msg, &self.data, settings, anim_state, &mut self.export_open).map(Message::Export)
            }
            Message::Overlay(msg) => {
                match msg {
                    overlay::Message::Selected(region) => self.export.set_region(region),
                    overlay::Message::Cancelled => {}
                }
                self.overlay.selecting = false;
                self.export_open = true;
                Task::none()
            }
            Message::Preloaded(result) => {
                self.data.apply_preload(result);
                self.sync_playhead();
                Task::none()
            }
            Message::ToggleExpanded => {
                self.is_expanded = !self.is_expanded;
                Task::none()
            }
        }
    }

    pub fn view(&self, settings: &Settings, anim_state: &AnimState) -> Element<'_, Message> {
        if self.data.held_unit.is_none() {
            let notice = container(text(MISSING_FILES_NOTICE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|t: &Theme| {
                    let palette = t.palette();
                    let shade = |c: f32| c * EMPTY_BACKGROUND_SHADE;
                    container::Style {
                        background: Some(Background::Color(Color {
                            r: shade(palette.background.r),
                            g: shade(palette.background.g),
                            b: shade(palette.background.b),
                            a: palette.background.a,
                        })),
                        ..container::Style::default()
                    }
                });

            return stack![notice, frame_border()].into();
        }

        if self.is_expanded {
            return container(
                column![
                    text("Animation Expanded").size(16),
                    button(text("Restore View")).on_press(Message::ToggleExpanded),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        self.viewer_view(settings, anim_state)
    }

    pub fn export_popup_open(&self) -> bool {
        self.export_open
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        (self.export_open && self.data.held_unit.is_some())
            .then(|| self.export.view(window).map(Message::Export))
    }

    pub fn expanded_view(&self, settings: &Settings, anim_state: &AnimState) -> Option<Element<'_, Message>> {
        if !self.is_expanded || self.data.held_unit.is_none() {
            return None;
        }

        Some(
            container(self.viewer_view(settings, anim_state))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    background: Some(theme.palette().background.into()),
                    ..container::Style::default()
                })
                .into(),
        )
    }

    fn viewer_view(&self, settings: &Settings, anim_state: &AnimState) -> Element<'_, Message> {
        let viewport = smooth_scroll(self.canvas.view(&self.data).map(Message::Canvas)).strength(ZOOM_SCROLL_STRENGTH);

        let selection_overlay = self.overlay
            .view(&self.canvas, self.export.camera_region(self.export_open))
            .map(Message::Overlay);

        let controls_overlay = container(self.controls.view(&self.canvas, &self.data, anim_state, self.action).map(Message::Controls))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(Padding { left: CONTROLS_INSET_LEFT, ..Padding::ZERO });

        let is_expanded = self.is_expanded;
        let expand_button = container(
            button(expand::expand(is_expanded))
                .width(Length::Fixed(EXPAND_BUTTON_SIZE))
                .height(Length::Fixed(EXPAND_BUTTON_SIZE))
                .padding(0)
                .style(move |t: &Theme, status| theme::overlay_button(t, status, is_expanded))
                .on_press(Message::ToggleExpanded),
        )
        .padding(EXPAND_BUTTON_INSET);

        let layers = stack![
            viewport,
            diagnostics::view(&self.data, &self.canvas, &settings.animation, self.highlight),
            selection_overlay,
            self.overlay.hint_view(),
            expand_button,
            controls_overlay,
            frame_border(),
        ];

        editor::suppress(
            container(layers).width(Length::Fill).height(Length::Fill),
            self.overlay.selecting,
        )
    }
}
