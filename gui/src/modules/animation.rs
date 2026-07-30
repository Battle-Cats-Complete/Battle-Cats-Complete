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
    pub fn sync(&mut self, cat: &CatEntry, form: usize, settings: &Settings) {
        self.data.sync(cat, form, settings);
        self.export.sync(&self.data, settings);
    }

    pub fn sync_enemy(&mut self, enemy: &EnemyEntry, settings: &Settings) {
        self.data.sync_enemy(enemy, settings);
        self.export.sync(&self.data, settings);
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

    pub fn update(&mut self, message: Message, settings: &mut Settings) -> Task<Message> {
        match message {
            Message::Canvas(msg) => {
                self.canvas.update(msg, &self.data);
                Task::none()
            }
            Message::Controls(controls::Message::OpenExport) => {
                let was_open = settings.animation.export_popup_open;
                settings.animation.export_popup_open = true;
                if settings.animation.auto_set_camera_region && !was_open && !self.overlay.selecting {
                    return self.export.update(export::Message::UseBounds, &self.data, settings).map(Message::Export);
                }
                Task::none()
            }
            Message::Controls(msg) => {
                self.controls.update(msg, &mut self.canvas, &mut self.data, settings);
                Task::none()
            }
            Message::Export(export::Message::SetCamera) => {
                settings.animation.export_popup_open = false;
                self.overlay.selecting = true;
                Task::none()
            }
            Message::Export(msg) => self.export.update(msg, &self.data, settings).map(Message::Export),
            Message::Overlay(msg) => {
                match msg {
                    overlay::Message::Selected(region) => self.export.set_region(region),
                    overlay::Message::Cancelled => {}
                }
                self.overlay.selecting = false;
                settings.animation.export_popup_open = true;
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

    pub fn view(&self, settings: &Settings) -> Element<'_, Message> {
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

        self.viewer_view(settings)
    }

    pub fn export_popup_open(&self, settings: &Settings) -> bool {
        settings.animation.export_popup_open
    }

    pub fn export_popup_view(&self, window: Size, settings: &Settings) -> Option<Element<'_, Message>> {
        (settings.animation.export_popup_open && self.data.held_unit.is_some())
            .then(|| self.export.view(window).map(Message::Export))
    }

    pub fn expanded_view(&self, settings: &Settings) -> Option<Element<'_, Message>> {
        if !self.is_expanded || self.data.held_unit.is_none() {
            return None;
        }

        Some(
            container(self.viewer_view(settings))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    background: Some(theme.palette().background.into()),
                    ..container::Style::default()
                })
                .into(),
        )
    }

    fn viewer_view(&self, settings: &Settings) -> Element<'_, Message> {
        let viewport = self.canvas.view(&self.data).map(Message::Canvas);

        let selection_overlay = self.overlay
            .view(&self.canvas, self.export.camera_region(settings), settings.animation.debug_view)
            .map(Message::Overlay);

        let controls_overlay = container(self.controls.view(&self.canvas, &self.data, settings).map(Message::Controls))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .padding(10);

        let expand_style = if self.is_expanded { button::primary } else { button::secondary };
        let expand_button = container(
            button(text("⛶").size(20))
                .style(expand_style)
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
