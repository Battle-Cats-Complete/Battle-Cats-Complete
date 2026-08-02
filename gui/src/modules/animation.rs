mod canvas;
mod controls;
mod data;
mod export;
mod offscreen;
mod overlay;
mod pipeline;

use iced::widget::{button, column, container, stack, text};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use core::modules::cat::scanner::CatEntry;
use core::modules::enemy::scanner::EnemyEntry;
use core::modules::settings::Settings;
use core::modules::state::AnimState;

use crate::app::theme;

#[derive(Default)]
pub struct State {
    data: data::State,
    canvas: canvas::State,
    controls: controls::State,
    export: export::State,
    overlay: overlay::State,
    is_expanded: bool,
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
    pub fn sync(&mut self, cat: &CatEntry, form: usize, settings: &Settings, anim_state: &AnimState) {
        self.data.sync(cat, form, settings);
        self.export.sync(&self.data, settings, anim_state);
    }

    pub fn sync_enemy(&mut self, enemy: &EnemyEntry, settings: &Settings, anim_state: &AnimState) {
        self.data.sync_enemy(enemy, settings);
        self.export.sync(&self.data, settings, anim_state);
    }

    pub fn preload(&mut self, cat: &CatEntry, form: usize, settings: &Settings) -> Task<Message> {
        Self::preload_task(self.data.preload_request(cat, form, settings))
    }

    pub fn preload_enemy(&mut self, enemy: &EnemyEntry, settings: &Settings) -> Task<Message> {
        Self::preload_task(self.data.preload_enemy_request(enemy, settings))
    }

    fn preload_task(request: Option<data::PreloadRequest>) -> Task<Message> {
        match request {
            Some(request) => Task::perform(smol::unblock(move || request.run()), Message::Preloaded),
            None => Task::none(),
        }
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
            return container(text("No unit loaded for this form"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
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
        let viewport = self.canvas.view(&self.data).map(Message::Canvas);

        let selection_overlay = self.overlay
            .view(&self.canvas, self.export.camera_region(anim_state), settings.animation.debug_view)
            .map(Message::Overlay);

        let controls_overlay = container(self.controls.view(&self.canvas, &self.data, anim_state).map(Message::Controls))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .padding(10);

        let is_expanded = self.is_expanded;
        let expand_button = container(
            button(text("⛶").size(20))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, is_expanded))
                .on_press(Message::ToggleExpanded),
        )
        .padding(8);

        let layers = stack![
            viewport,
            selection_overlay,
            container(controls_overlay)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom),
            expand_button,
        ];

        container(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
