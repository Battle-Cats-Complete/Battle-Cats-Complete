mod canvas;
mod controls;
mod data;
mod export;
mod pipeline;

use iced::widget::{container, stack, text};
use iced::{Element, Length, Task};

use core::modules::animation::{IDX_IDLE, IDX_WALK};
use core::modules::cat::scanner::CatEntry;
use core::modules::settings::Settings;

#[derive(Default)]
pub struct State {
    data: data::State,
    canvas: canvas::State,
    controls: controls::State,
    export: export::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(canvas::Message),
    Controls(controls::Message),
    Export(export::Message),
}

impl State {
    pub fn sync(&mut self, cat: &CatEntry, form: usize, settings: &Settings) {
        self.data.sync(cat, form, settings);
    }

    pub fn tick(&mut self) {
        self.canvas.update(canvas::Message::Tick, &self.data);
        self.controls.tick(&mut self.canvas, &self.data);
        self.export.tick();
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::Canvas(msg) => {
                self.canvas.update(msg, &self.data);
                Task::none()
            }
            Message::Controls(controls::Message::OpenExport) => {
                let loop_supported = self.data.loaded_anim_index == IDX_WALK || self.data.loaded_anim_index == IDX_IDLE;
                self.export.open(settings, loop_supported);
                Task::none()
            }
            Message::Controls(msg) => {
                self.controls.update(msg, &mut self.canvas, &mut self.data);
                Task::none()
            }
            Message::Export(msg) => {
                self.export.update(msg);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.data.held_unit.is_none() {
            return container(text("No unit loaded for this form"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let viewport = self.canvas.view(&self.data).map(Message::Canvas);

        let controls_overlay = container(self.controls.view(&self.canvas, &self.data).map(Message::Controls))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .padding(10);

        let mut layers = stack![
            viewport,
            container(controls_overlay)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom),
        ];

        if self.export.is_open {
            let modal = container(self.export.view().map(Message::Export))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(container::transparent);
            layers = layers.push(modal);
        }

        container(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
