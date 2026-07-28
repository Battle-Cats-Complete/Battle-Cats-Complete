use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Border, Element, Length, Theme};

use core::modules::settings::UserKeys;

const FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const COLUMN_INPUT_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionSlot {
    Ja,
    En,
    Tw,
    Ko,
}

impl RegionSlot {
    const ALL: [Self; 4] = [Self::Ja, Self::En, Self::Tw, Self::Ko];

    fn label(self) -> &'static str {
        match self {
            Self::Ja => "Japan",
            Self::En => "Global",
            Self::Tw => "Taiwan",
            Self::Ko => "Korea",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Tick,
    KeyChanged(RegionSlot, String),
    IvChanged(RegionSlot, String),
    Import,
    Export,
    Validate,
    DeleteRequested,
}

pub struct State {
    pub is_open: bool,
    keys: UserKeys,
    validation_status: Option<[(bool, bool); 4]>,
    import_feedback: Option<(bool, Instant)>,
    export_feedback: Option<(bool, Instant)>,
    confirm_delete: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            keys: UserKeys::load(),
            validation_status: None,
            import_feedback: None,
            export_feedback: None,
            confirm_delete: None,
        }
    }
}

impl State {
    fn region_mut(&mut self, slot: RegionSlot) -> &mut core::modules::settings::RegionKey {
        match slot {
            RegionSlot::Ja => &mut self.keys.ja,
            RegionSlot::En => &mut self.keys.en,
            RegionSlot::Tw => &mut self.keys.tw,
            RegionSlot::Ko => &mut self.keys.ko,
        }
    }

    fn region_ref(&self, slot: RegionSlot) -> &core::modules::settings::RegionKey {
        match slot {
            RegionSlot::Ja => &self.keys.ja,
            RegionSlot::En => &self.keys.en,
            RegionSlot::Tw => &self.keys.tw,
            RegionSlot::Ko => &self.keys.ko,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                self.keys = UserKeys::load();
                self.validation_status = None;
                self.is_open = true;
            }
            Message::Close => {
                self.is_open = false;
                self.validation_status = None;
            }
            Message::Tick => {
                let expired = |feedback: &Option<(bool, Instant)>| {
                    feedback.is_some_and(|(_, at)| at.elapsed() > FEEDBACK_DURATION)
                };
                if expired(&self.import_feedback) { self.import_feedback = None; }
                if expired(&self.export_feedback) { self.export_feedback = None; }
                if self.confirm_delete.is_some_and(|at| at.elapsed() > FEEDBACK_DURATION) {
                    self.confirm_delete = None;
                }
            }
            Message::KeyChanged(slot, value) => {
                self.region_mut(slot).key = value;
                self.validation_status = None;
                self.keys.save();
            }
            Message::IvChanged(slot, value) => {
                self.region_mut(slot).iv = value;
                self.validation_status = None;
                self.keys.save();
            }
            Message::Import => {
                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    let success = fs::read_to_string(&path).ok()
                        .and_then(|data| serde_json::from_str::<UserKeys>(&data).ok())
                        .map(|parsed| {
                            self.keys = parsed;
                            self.validation_status = None;
                            self.keys.save();
                        })
                        .is_some();
                    self.import_feedback = Some((success, Instant::now()));
                }
            }
            Message::Export => {
                let export_dir = Path::new("exports");
                let _ = fs::create_dir_all(export_dir);
                let success = serde_json::to_string_pretty(&self.keys).ok()
                    .is_some_and(|json| fs::write(export_dir.join("keys.json"), json).is_ok());
                self.export_feedback = Some((success, Instant::now()));
            }
            Message::Validate => {
                self.validation_status = Some(self.keys.validate());
            }
            Message::DeleteRequested => {
                if self.confirm_delete.is_some() {
                    self.keys = UserKeys::default();
                    self.validation_status = None;
                    self.keys.save();
                    self.confirm_delete = None;
                } else {
                    self.confirm_delete = Some(Instant::now());
                }
            }
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let action_button = |label: &'a str, msg: Message, color: [u8; 3]| {
            button(text(label).size(12))
                .padding([6, 14])
                .style(move |_theme: &Theme, _status| button::Style {
                    background: Some(iced::Color::from_rgb8(color[0], color[1], color[2]).into()),
                    text_color: iced::Color::WHITE,
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(msg)
        };

        let import_label = match self.import_feedback {
            Some((true, _)) => "Loaded!",
            Some((false, _)) => "Failed!",
            None => "Load Keys",
        };
        let import_color = match self.import_feedback {
            Some((true, _)) => [40, 160, 60],
            Some((false, _)) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let export_label = match self.export_feedback {
            Some((true, _)) => "Exported!",
            Some((false, _)) => "Failed!",
            None => "Export Keys",
        };
        let export_color = match self.export_feedback {
            Some((true, _)) => [40, 160, 60],
            Some((false, _)) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let delete_label = if self.confirm_delete.is_some() { "Are You Sure?" } else { "Delete Keys" };

        let actions = row![
            action_button(import_label, Message::Import, import_color),
            action_button(export_label, Message::Export, export_color),
            action_button("Validate Keys", Message::Validate, [31, 106, 165]),
            action_button(delete_label, Message::DeleteRequested, [180, 50, 50]),
        ].spacing(10);

        let default_validation = [(true, true); 4];
        let validations = self.validation_status.unwrap_or(default_validation);

        let mut grid = column![
            row![
                text("Region").width(Length::Fixed(60.0)).size(13),
                text("Decryption Key").width(Length::Fixed(COLUMN_INPUT_WIDTH)).size(13),
                text("Initialization Vector").width(Length::Fixed(COLUMN_INPUT_WIDTH)).size(13),
            ].spacing(15)
        ].spacing(8);

        for (index, slot) in RegionSlot::ALL.into_iter().enumerate() {
            let region = self.region_ref(slot);
            let (key_valid, iv_valid) = validations[index];

            let key_input = text_input("Key", &region.key)
                .on_input(move |value| Message::KeyChanged(slot, value))
                .width(Length::Fixed(COLUMN_INPUT_WIDTH))
                .style(move |theme: &Theme, status| {
                    let mut style = text_input::default(theme, status);
                    if self.validation_status.is_some() {
                        style.background = if key_valid {
                            iced::Color::from_rgb8(30, 80, 40).into()
                        } else {
                            iced::Color::from_rgb8(120, 30, 30).into()
                        };
                    }
                    style
                });

            let iv_input = text_input("IV", &region.iv)
                .on_input(move |value| Message::IvChanged(slot, value))
                .width(Length::Fixed(COLUMN_INPUT_WIDTH))
                .style(move |theme: &Theme, status| {
                    let mut style = text_input::default(theme, status);
                    if self.validation_status.is_some() {
                        style.background = if iv_valid {
                            iced::Color::from_rgb8(30, 80, 40).into()
                        } else {
                            iced::Color::from_rgb8(120, 30, 30).into()
                        };
                    }
                    style
                });

            grid = grid.push(
                row![
                    text(slot.label()).width(Length::Fixed(60.0)),
                    key_input,
                    iv_input,
                ].spacing(15).align_y(Alignment::Center)
            );
        }

        let content = column![
            text("Manage Decryption Keys").size(22),
            actions,
            scrollable(grid).height(Length::Shrink),
            row![
                button("Close").on_press(Message::Close),
            ]
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
