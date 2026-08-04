use std::fs;
use std::path::Path;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use core::modules::settings::UserKeys;

use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::popup;

const POPUP_SIZE: Size = Size::new(650.0, 335.0);
const REGION_COLUMN_WIDTH: f32 = 60.0;

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
    Popup(popup::Message),
    Open,
    KeyChanged(RegionSlot, String),
    IvChanged(RegionSlot, String),
    Import,
    ImportExpired,
    Export,
    ExportExpired,
    Validate,
    DeleteRequested,
    ConfirmExpired,
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    keys: UserKeys,
    validation_status: Option<[(bool, bool); 4]>,
    import_feedback: Slot<bool>,
    export_feedback: Slot<bool>,
    confirm_delete: Slot<()>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            popup: popup::State::default(),
            keys: UserKeys::load(),
            validation_status: None,
            import_feedback: Slot::default(),
            export_feedback: Slot::default(),
            confirm_delete: Slot::default(),
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => {
                self.keys = UserKeys::load();
                self.validation_status = None;
                self.is_open = true;
                Task::none()
            }
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    self.is_open = false;
                    self.validation_status = None;
                }
                Task::none()
            }
            Message::KeyChanged(slot, value) => {
                self.region_mut(slot).key = value;
                self.validation_status = None;
                self.keys.save();
                Task::none()
            }
            Message::IvChanged(slot, value) => {
                self.region_mut(slot).iv = value;
                self.validation_status = None;
                self.keys.save();
                Task::none()
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
                    return self.import_feedback.set(success, Message::ImportExpired);
                }
                Task::none()
            }
            Message::ImportExpired => {
                self.import_feedback.expire();
                Task::none()
            }
            Message::Export => {
                let export_dir = Path::new("exports");
                let _ = fs::create_dir_all(export_dir);
                let success = serde_json::to_string_pretty(&self.keys).ok()
                    .is_some_and(|json| fs::write(export_dir.join("keys.json"), json).is_ok());
                self.export_feedback.set(success, Message::ExportExpired)
            }
            Message::ExportExpired => {
                self.export_feedback.expire();
                Task::none()
            }
            Message::Validate => {
                self.validation_status = Some(self.keys.validate());
                Task::none()
            }
            Message::DeleteRequested => {
                if self.confirm_delete.is_set() {
                    self.keys = UserKeys::default();
                    self.validation_status = None;
                    self.keys.save();
                    self.confirm_delete.clear();
                    Task::none()
                } else {
                    self.confirm_delete.set((), Message::ConfirmExpired)
                }
            }
            Message::ConfirmExpired => {
                self.confirm_delete.expire();
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, window: Size) -> Element<'a, Message> {
        self.popup.view("Manage Decryption Keys", POPUP_SIZE, window, Message::Popup, move || self.content_view())
    }

    fn content_view<'a>(&'a self) -> Element<'a, Message> {
        let action_button = |label: &'a str, msg: Message, color: [u8; 3]| {
            button(text(label).size(12))
                .padding([6, 14])
                .style(move |_theme: &Theme, _status| theme::solid_button(iced::Color::from_rgb8(color[0], color[1], color[2])))
                .on_press(msg)
        };

        let import_label = match self.import_feedback.get().copied() {
            Some(true) => "Loaded!",
            Some(false) => "Failed!",
            None => "Load Keys",
        };
        let import_color = match self.import_feedback.get().copied() {
            Some(true) => [40, 160, 60],
            Some(false) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let export_label = match self.export_feedback.get().copied() {
            Some(true) => "Exported!",
            Some(false) => "Failed!",
            None => "Export Keys",
        };
        let export_color = match self.export_feedback.get().copied() {
            Some(true) => [40, 160, 60],
            Some(false) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let delete_label = if self.confirm_delete.is_set() { "Are You Sure?" } else { "Delete Keys" };

        let actions = row![
            action_button(import_label, Message::Import, import_color),
            action_button(export_label, Message::Export, export_color),
            action_button("Validate Keys", Message::Validate, [31, 106, 165]),
            action_button(delete_label, Message::DeleteRequested, [180, 50, 50]),
        ].spacing(10);

        let default_validation = [(true, true); 4];
        let validations = self.validation_status.unwrap_or(default_validation);

        let header = container(
            row![
                theme::table_cell_text("Region", Length::Fixed(REGION_COLUMN_WIDTH)).size(13),
                theme::table_cell_text("Decryption Key", Length::FillPortion(1)).size(13),
                theme::table_cell_text("Initialization Vector", Length::FillPortion(1)).size(13),
            ].spacing(15).width(Length::Fill)
        )
        .style(theme::zebra_table_header)
        .padding([6, 10])
        .width(Length::Fill);

        let mut grid = column![header].spacing(0).width(Length::Fill);

        for (index, slot) in RegionSlot::ALL.into_iter().enumerate() {
            let region = self.region_ref(slot);
            let (key_valid, iv_valid) = validations[index];

            let key_input = text_input("Key", &region.key)
                .on_input(move |value| Message::KeyChanged(slot, value))
                .size(12)
                .width(Length::FillPortion(1))
                .style(move |theme: &Theme, status| {
                    let mut style = theme::rounded_input(theme, status);
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
                .size(12)
                .width(Length::FillPortion(1))
                .style(move |theme: &Theme, status| {
                    let mut style = theme::rounded_input(theme, status);
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
                container(
                    row![
                        theme::table_cell_text(slot.label(), Length::Fixed(REGION_COLUMN_WIDTH)),
                        key_input,
                        iv_input,
                    ].spacing(15).align_y(Alignment::Center).width(Length::Fill)
                )
                .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
                .padding([6, 10])
                .width(Length::Fill)
            );
        }

        let content = column![
            actions,
            scrollable(grid).height(Length::Shrink).width(Length::Fill),
        ].spacing(15).padding(20).width(Length::Fill).align_x(Alignment::Center);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
