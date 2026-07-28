use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Element, Length, Task, Theme};

use core::modules::settings::pem;

const CONFIRM_WINDOW: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Tick,
    Import,
    Export,
    GenerateRequested,
    Generated(Option<String>),
    DeleteRequested,
}

pub struct State {
    pub is_open: bool,
    active_pem: String,
    is_custom: bool,
    is_generating: bool,
    confirm_generate: Option<Instant>,
    confirm_delete: Option<Instant>,
    export_feedback: Option<(bool, Instant)>,
}

impl Default for State {
    fn default() -> Self {
        let (active_pem, is_custom) = pem::get_active_pem();
        Self {
            is_open: false,
            active_pem,
            is_custom,
            is_generating: false,
            confirm_generate: None,
            confirm_delete: None,
            export_feedback: None,
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
            Message::Close => {
                self.is_open = false;
                self.confirm_generate = None;
                self.confirm_delete = None;
                Task::none()
            }
            Message::Tick => {
                if self.confirm_generate.is_some_and(|at| at.elapsed() > CONFIRM_WINDOW) {
                    self.confirm_generate = None;
                }
                if self.confirm_delete.is_some_and(|at| at.elapsed() > CONFIRM_WINDOW) {
                    self.confirm_delete = None;
                }
                if self.export_feedback.is_some_and(|(_, at)| at.elapsed() > CONFIRM_WINDOW) {
                    self.export_feedback = None;
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
                    self.confirm_generate = None;
                    self.confirm_delete = None;
                }
                Task::none()
            }
            Message::Export => {
                let export_dir = Path::new("exports");
                let _ = fs::create_dir_all(export_dir);
                let filename = if self.is_custom { "identity.pem" } else { "bcc.pem" };
                let success = fs::write(export_dir.join(filename), &self.active_pem).is_ok();
                self.export_feedback = Some((success, Instant::now()));
                Task::none()
            }
            Message::GenerateRequested => {
                if self.is_custom && self.confirm_generate.is_none() {
                    self.confirm_generate = Some(Instant::now());
                    self.confirm_delete = None;
                    Task::none()
                } else {
                    self.confirm_generate = None;
                    self.is_generating = true;
                    Task::perform(async { pem::generate_pem().ok() }, Message::Generated)
                }
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
                if self.confirm_delete.is_none() {
                    self.confirm_delete = Some(Instant::now());
                    self.confirm_generate = None;
                } else {
                    pem::delete_pem();
                    let (default_pem, _) = pem::get_active_pem();
                    self.active_pem = default_pem;
                    self.is_custom = false;
                    self.confirm_delete = None;
                }
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let action_button = |label: &'a str, msg: Option<Message>, color: [u8; 3]| {
            let mut b = button(text(label).size(12))
                .padding([6, 14])
                .style(move |_theme: &Theme, _status| button::Style {
                    background: Some(iced::Color::from_rgb8(color[0], color[1], color[2]).into()),
                    text_color: iced::Color::WHITE,
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                });
            if let Some(msg) = msg { b = b.on_press(msg); }
            b
        };

        let export_label = match self.export_feedback {
            Some((true, _)) => "Exported!",
            Some((false, _)) => "Failed!",
            None => "Export PEM",
        };
        let export_color = match self.export_feedback {
            Some((true, _)) => [40, 160, 60],
            Some((false, _)) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let generate_label = if self.is_generating {
            "Generating..."
        } else if self.confirm_generate.is_some() {
            "Are You Sure?"
        } else {
            "Generate PEM"
        };

        let delete_label = if self.confirm_delete.is_some() { "Are You Sure?" } else { "Delete PEM" };

        let import_msg = if self.is_generating { None } else { Some(Message::Import) };
        let export_msg = if self.is_generating { None } else { Some(Message::Export) };
        let generate_msg = if self.is_generating { None } else { Some(Message::GenerateRequested) };
        let delete_msg = if self.is_generating || !self.is_custom { None } else { Some(Message::DeleteRequested) };

        let actions = row![
            action_button("Import PEM", import_msg, [31, 106, 165]),
            action_button(export_label, export_msg, export_color),
            action_button(generate_label, generate_msg, [200, 180, 50]),
            action_button(delete_label, delete_msg, [180, 50, 50]),
        ].spacing(10);

        let content = column![
            text("Manage PEM").size(22),
            actions,
            scrollable(
                container(text(self.active_pem.clone()).size(12).font(iced::Font::MONOSPACE))
                    .padding(10)
                    .width(Length::Fill)
            ).height(Length::Fixed(320.0)),
            row![button("Close").on_press(Message::Close)]
        ].spacing(15).padding(20).align_x(Alignment::Center);

        container(content)
            .style(|theme: &Theme| {
                container::background(theme.palette().background)
                    .border(Border { color: theme.palette().text, width: 1.0, radius: 8.0.into() })
            })
            .max_width(650.0)
            .into()
    }
}
