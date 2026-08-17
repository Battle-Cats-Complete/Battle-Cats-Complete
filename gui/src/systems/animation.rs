mod canvas;
mod controls;
mod data;
mod export;
mod expand;
mod offscreen;
mod overlay;
mod pipeline;

use iced::widget::{button, column, container, stack, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Size, Task, Theme};

use core::domains::cat::scanner::CatEntry;
use core::domains::enemy::scanner::EnemyEntry;
use core::domains::settings::Settings;
use core::Vfs;

use crate::app::state::AnimState;
use crate::app::theme;
use crate::widget::smooth_scroll;

const FRAME_BORDER_WIDTH: f32 = 4.0;
const FRAME_BORDER_RADIUS: f32 = 5.0;
const EMPTY_BACKGROUND_SHADE: f32 = 0.6;

const EXPAND_BUTTON_SIZE: f32 = 30.0;
const EXPAND_BUTTON_INSET: f32 = 8.0;

const CONTROLS_INSET_LEFT: f32 = 7.0;

const ZOOM_SCROLL_STRENGTH: f32 = 2.5;

const MISSING_FILES_NOTICE: &str = "Missing essential files needed to load Entity";

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

#[derive(Default)]
pub struct State {
    data: data::State,
    canvas: canvas::State,
    controls: controls::State,
    export: export::State,
    overlay: overlay::State,
    is_expanded: bool,
    playhead_id: String,
    playhead_index: usize,
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
    pub fn sync(&mut self, cat: &CatEntry, form: usize, vfs: &Vfs, settings: &Settings, anim_state: &AnimState) {
        self.data.sync(cat, form, vfs);
        self.export.sync(&self.data, settings, anim_state);
        self.sync_playhead();
    }

    pub fn sync_enemy(&mut self, enemy: &EnemyEntry, vfs: &Vfs, settings: &Settings, anim_state: &AnimState) {
        self.data.sync_enemy(enemy, vfs);
        self.export.sync(&self.data, settings, anim_state);
        self.sync_playhead();
    }

    pub fn reset_playhead(&mut self) {
        self.playhead_reset = true;
    }

    fn sync_playhead(&mut self) {
        if self.playhead_id == self.data.loaded_id() && self.playhead_index == self.data.active_index() {
            return;
        }

        self.playhead_id = self.data.loaded_id().to_string();
        self.playhead_index = self.data.active_index();

        if std::mem::take(&mut self.playhead_reset) {
            self.canvas.current_frame = 0.0;
        }

        controls::clamp_frame(&mut self.canvas, &self.data);
    }

    pub fn preload(&mut self, cat: &CatEntry, form: usize, vfs: &Vfs) -> Task<Message> {
        Self::preload_task(self.data.preload_request(cat, form, vfs))
    }

    pub fn preload_enemy(&mut self, enemy: &EnemyEntry, vfs: &Vfs) -> Task<Message> {
        Self::preload_task(self.data.preload_enemy_request(enemy, vfs))
    }

    fn preload_task(request: Option<data::PreloadRequest>) -> Task<Message> {
        request.map_or_else(Task::none, |request| Task::perform(smol::unblock(move || request.run()), Message::Preloaded))
    }

    pub fn invalidate_paths(&mut self) {
        self.data.invalidate_paths();
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
                let was_open = anim_state.export_popup_open;
                anim_state.export_popup_open = true;
                if settings.animation.auto_set_camera_region && !was_open && !self.overlay.selecting {
                    return self.export.update(export::Message::UseBounds, &self.data, settings, anim_state).map(Message::Export);
                }
                Task::none()
            }
            Message::Controls(msg) => {
                self.controls.update(msg, &mut self.canvas, &mut self.data, anim_state);
                self.sync_playhead();
                Task::none()
            }
            Message::Export(export::Message::SetCamera) => {
                anim_state.export_popup_open = false;
                self.overlay.selecting = true;
                Task::none()
            }
            Message::Export(msg) => self.export.update(msg, &self.data, settings, anim_state).map(Message::Export),
            Message::Overlay(msg) => {
                match msg {
                    overlay::Message::Selected(region) => self.export.set_region(region),
                    overlay::Message::Cancelled => {}
                }
                self.overlay.selecting = false;
                anim_state.export_popup_open = true;
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

    pub fn export_popup_open(&self, anim_state: &AnimState) -> bool {
        anim_state.export_popup_open
    }

    pub fn export_popup_view(&self, window: Size, anim_state: &AnimState) -> Option<Element<'_, Message>> {
        (anim_state.export_popup_open && self.data.held_unit.is_some())
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
            .view(&self.canvas, self.export.camera_region(anim_state), settings.animation.debug_view)
            .map(Message::Overlay);

        let controls_overlay = container(self.controls.view(&self.canvas, &self.data, anim_state).map(Message::Controls))
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
            selection_overlay,
            self.overlay.hint_view(),
            expand_button,
            controls_overlay,
            frame_border(),
        ];

        container(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
