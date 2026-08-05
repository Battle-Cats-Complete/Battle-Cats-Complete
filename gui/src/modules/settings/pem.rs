use std::fs;
use std::path::Path;

use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};

use core::modules::settings::pem;

use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::{popup, smooth_scroll};

const POPUP_SIZE: Size = Size::new(650.0, 480.0);

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Open,
    Import,
    Export,
    ExportExpired,
    GenerateRequested,
    GenerateExpired,
    Generated(Option<String>),
    DeleteRequested,
    DeleteExpired,
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    active_pem: String,
    is_custom: bool,
    is_generating: bool,
    confirm_generate: Slot<()>,
    confirm_delete: Slot<()>,
    export_feedback: Slot<bool>,
}

impl Default for State {
    fn default() -> Self {
        let (active_pem, is_custom) = pem::get_active_pem();
        Self {
            is_open: false,
            popup: popup::State::default(),
            active_pem,
            is_custom,
            is_generating: false,
            confirm_generate: Slot::default(),
            confirm_delete: Slot::default(),
            export_feedback: Slot::default(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => {
                let (active_pem, is_custom) = pem::get_active_pem();
                self.active_pem = active_pem;
                self.is_custom = is_custom;
                self.is_open = true;
                Task::none()
            }
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    self.is_open = false;
                    self.confirm_generate.clear();
                    self.confirm_delete.clear();
                }
                Task::none()
            }
            Message::Import => {
                if let Some(path) = rfd::FileDialog::new().add_filter("PEM", &["pem", "txt"]).pick_file()
                    && let Ok(content) = fs::read_to_string(&path)
                    && content.contains("-----BEGIN PRIVATE KEY-----")
                    && content.contains("-----BEGIN CERTIFICATE-----") {
                    let _ = pem::save_pem(&content);
                    self.active_pem = content;
                    self.is_custom = true;
                    self.confirm_generate.clear();
                    self.confirm_delete.clear();
                }
                Task::none()
            }
            Message::Export => {
                let export_dir = Path::new("exports");
                let _ = fs::create_dir_all(export_dir);
                let filename = if self.is_custom { "identity.pem" } else { "bcc.pem" };
                let success = fs::write(export_dir.join(filename), &self.active_pem).is_ok();
                self.export_feedback.set(success, Message::ExportExpired)
            }
            Message::ExportExpired => {
                self.export_feedback.expire();
                Task::none()
            }
            Message::GenerateRequested => {
                if self.is_custom && !self.confirm_generate.is_set() {
                    self.confirm_delete.clear();
                    self.confirm_generate.set((), Message::GenerateExpired)
                } else {
                    self.confirm_generate.clear();
                    self.is_generating = true;
                    Task::perform(async { pem::generate_pem().ok() }, Message::Generated)
                }
            }
            Message::GenerateExpired => {
                self.confirm_generate.expire();
                Task::none()
            }
            Message::Generated(result) => {
                if let Some(new_pem) = result {
                    let _ = pem::save_pem(&new_pem);
                    self.active_pem = new_pem;
                    self.is_custom = true;
                }
                self.is_generating = false;
                Task::none()
            }
            Message::DeleteRequested => {
                if !self.confirm_delete.is_set() {
                    self.confirm_generate.clear();
                    self.confirm_delete.set((), Message::DeleteExpired)
                } else {
                    pem::delete_pem();
                    let (default_pem, _) = pem::get_active_pem();
                    self.active_pem = default_pem;
                    self.is_custom = false;
                    self.confirm_delete.clear();
                    Task::none()
                }
            }
            Message::DeleteExpired => {
                self.confirm_delete.expire();
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, window: Size) -> Element<'a, Message> {
        self.popup.view("Manage PEM", POPUP_SIZE, window, Message::Popup, move || self.content_view(), None)
    }

    fn content_view<'a>(&'a self) -> Element<'a, Message> {
        let export_label = match self.export_feedback.get().copied() {
            Some(true) => "Exported!",
            Some(false) => "Failed!",
            None => "Export PEM",
        };

        let generate_label = if self.is_generating {
            "Generating..."
        } else if self.confirm_generate.is_set() {
            "Are You Sure?"
        } else {
            "Generate PEM"
        };

        let delete_label = if self.confirm_delete.is_set() { "Are You Sure?" } else { "Delete PEM" };

        let import_msg = if self.is_generating { None } else { Some(Message::Import) };
        let export_msg = if self.is_generating { None } else { Some(Message::Export) };
        let generate_msg = if self.is_generating { None } else { Some(Message::GenerateRequested) };
        let delete_msg = if self.is_generating || !self.is_custom { None } else { Some(Message::DeleteRequested) };

        let actions = row![
            theme::sized_button("Import PEM", theme::POPUP_ACTION_BUTTON_WIDTH, theme::primary_button).on_press_maybe(import_msg),
            theme::sized_button(export_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::feedback_button_style(self.export_feedback.get().copied())).on_press_maybe(export_msg),
            theme::sized_button(generate_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::warning_button).on_press_maybe(generate_msg),
            theme::sized_button(delete_label, theme::POPUP_ACTION_BUTTON_WIDTH, if self.is_custom { theme::danger_button } else { theme::neutral_button }).on_press_maybe(delete_msg),
        ].spacing(10);

        let is_custom = self.is_custom;
        let pem_text_style = move |theme: &Theme| {
            let palette = theme.palette();
            let color = if is_custom { palette.text } else { Color { a: 0.5, ..palette.text } };
            text::Style { color: Some(color) }
        };

        let content = column![
            actions,
            container(
                smooth_scroll(
                    scrollable(
                        container(text(self.active_pem.clone()).size(12).font(iced::Font::MONOSPACE).style(pem_text_style))
                            .padding(10)
                            .width(Length::Fill)
                    ).height(Length::Fixed(320.0)),
                )
            ).style(theme::card_container),
        ].spacing(15).padding(20).align_x(Alignment::Center);

        container(smooth_scroll(scrollable(content)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
