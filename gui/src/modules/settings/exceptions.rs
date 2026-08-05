use std::fs;
use std::path::Path;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input, toggler};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use core::common::io::APP_LANGUAGES;
use core::modules::settings::{ExceptionList, ExceptionRule, RuleHandling};

use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::popup;

use super::hover_hint;

const POPUP_SIZE: Size = Size::new(750.0, 520.0);

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Open,
    AddRule,
    DeleteRule(usize),
    PatternChanged(usize, String),
    ExtensionChanged(usize, String),
    HandlingSelected(usize, RuleHandling),
    LanguageToggled(usize, String, bool),
    Import,
    ImportExpired,
    Export,
    ExportExpired,
    RequestReset,
    ConfirmReset,
    CancelReset,
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    rules: Vec<ExceptionRule>,
    import_feedback: Slot<bool>,
    export_feedback: Slot<bool>,
    confirm_reset: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            popup: popup::State::default(),
            rules: ExceptionList::load_or_default().rules,
            import_feedback: Slot::default(),
            export_feedback: Slot::default(),
            confirm_reset: false,
        }
    }
}

impl State {
    fn save(&mut self) {
        let mut list = ExceptionList { rules: self.rules.clone(), ..Default::default() };
        list.save();
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => {
                self.rules = ExceptionList::load_or_default().rules;
                self.is_open = true;
                Task::none()
            }
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    self.is_open = false;
                    self.confirm_reset = false;
                }
                Task::none()
            }
            Message::AddRule => {
                self.rules.push(ExceptionRule::default());
                self.save();
                Task::none()
            }
            Message::DeleteRule(index) => {
                if index < self.rules.len() {
                    self.rules.remove(index);
                    self.save();
                }
                Task::none()
            }
            Message::PatternChanged(index, value) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.pattern = value;
                }
                self.save();
                Task::none()
            }
            Message::ExtensionChanged(index, value) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.extension = value;
                }
                self.save();
                Task::none()
            }
            Message::HandlingSelected(index, handling) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.handling = handling;
                }
                self.save();
                Task::none()
            }
            Message::LanguageToggled(index, lang_code, enabled) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.languages.insert(lang_code, enabled);
                }
                self.save();
                Task::none()
            }
            Message::Import => {
                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    let success = ExceptionList::load_from_file(&path)
                        .map(|list| {
                            self.rules = list.rules;
                            self.save();
                        })
                        .is_ok();
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
                let mut export_list = ExceptionList { rules: self.rules.clone(), ..Default::default() };
                let success = export_list.save_to_file(&export_dir.join("exceptions.json")).is_ok();
                self.export_feedback.set(success, Message::ExportExpired)
            }
            Message::ExportExpired => {
                self.export_feedback.expire();
                Task::none()
            }
            Message::RequestReset => {
                self.confirm_reset = true;
                Task::none()
            }
            Message::CancelReset => {
                self.confirm_reset = false;
                Task::none()
            }
            Message::ConfirmReset => {
                self.rules = ExceptionList::default().rules;
                self.save();
                self.confirm_reset = false;
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, window: Size) -> Element<'a, Message> {
        self.popup.view("Manage Exceptions", POPUP_SIZE, window, Message::Popup, move || self.content_view(), None)
    }

    fn content_view<'a>(&'a self) -> Element<'a, Message> {
        let action_button = |label: &'a str, msg: Message, color: [u8; 3]| {
            button(text(label).size(12))
                .padding([6, 14])
                .style(move |_theme: &Theme, _status| theme::solid_button(iced::Color::from_rgb8(color[0], color[1], color[2])))
                .on_press(msg)
        };

        let import_label = match self.import_feedback.get().copied() {
            Some(true) => "Imported!",
            Some(false) => "Failed!",
            None => "Load List",
        };
        let import_color = match self.import_feedback.get().copied() {
            Some(true) => [40, 160, 60],
            Some(false) => [200, 40, 40],
            None => [31, 106, 165],
        };
        let export_label = match self.export_feedback.get().copied() {
            Some(true) => "Exported!",
            Some(false) => "Failed!",
            None => "Export List",
        };
        let export_color = match self.export_feedback.get().copied() {
            Some(true) => [40, 160, 60],
            Some(false) => [200, 40, 40],
            None => [31, 106, 165],
        };

        let actions = row![
            action_button("Add Rule", Message::AddRule, [31, 106, 165]),
            action_button(import_label, Message::Import, import_color),
            action_button(export_label, Message::Export, export_color),
            action_button("Reset to Default", Message::RequestReset, [180, 50, 50]),
        ].spacing(10);

        let header = row![
            text("Stem").width(Length::Fixed(180.0)).size(13),
            text("Extension").width(Length::Fixed(90.0)).size(13),
            text("Handling").width(Length::Fixed(110.0)).size(13),
            text("Languages").width(Length::Fixed(260.0)).size(13),
            text("").width(Length::Fixed(40.0)),
        ].spacing(10);

        let mut rows = column![header].spacing(6);

        for (index, rule) in self.rules.iter().enumerate() {
            let pattern_input = text_input("pattern", &rule.pattern)
                .on_input(move |v| Message::PatternChanged(index, v))
                .width(Length::Fixed(180.0))
                .style(theme::rounded_input);

            let extension_input = text_input("ext", &rule.extension)
                .on_input(move |v| Message::ExtensionChanged(index, v))
                .width(Length::Fixed(90.0))
                .style(theme::rounded_input);

            let handling_pick = pick_list(
                RuleHandling::all().map(|h| h.to_string()).to_vec(),
                Some(rule.handling.to_string()),
                move |selected| {
                    let handling = RuleHandling::all().into_iter()
                        .find(|h| h.to_string() == selected)
                        .unwrap_or(RuleHandling::Include);
                    Message::HandlingSelected(index, handling)
                }
            ).width(Length::Fixed(110.0)).style(theme::combo_box).menu_style(theme::combo_box_menu);

            let mut lang_row = row![].spacing(6);
            for &(lang_code, _) in APP_LANGUAGES {
                if let Some(&enabled) = rule.languages.get(lang_code) {
                    let code = lang_code.to_string();
                    lang_row = lang_row.push(
                        row![
                            toggler(enabled).on_toggle(move |v| Message::LanguageToggled(index, code.clone(), v)).size(16).style(theme::ios_toggle),
                            text(lang_code.to_uppercase()).size(11),
                        ].spacing(3).align_y(Alignment::Center)
                    );
                }
            }

            let delete_btn = hover_hint(
                button(text("🗑").size(14))
                    .on_press(Message::DeleteRule(index))
                    .style(button::danger),
                "Delete Rule",
            );

            rows = rows.push(
                row![
                    pattern_input,
                    extension_input,
                    handling_pick,
                    container(scrollable(lang_row).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4)))).width(Length::Fixed(260.0)),
                    delete_btn,
                ].spacing(10).align_y(Alignment::Center)
            );
        }

        let reset_confirm: Element<'a, Message> = if self.confirm_reset {
            container(
                row![
                    text("Reset all rules to default? Custom rules will be lost."),
                    button("Yes").on_press(Message::ConfirmReset).style(button::danger),
                    button("No").on_press(Message::CancelReset),
                ].spacing(10).align_y(Alignment::Center)
            ).padding(10).into()
        } else {
            container(text("")).into()
        };

        let content = column![
            actions,
            reset_confirm,
            scrollable(rows).height(Length::Fixed(360.0)),
        ].spacing(15).padding(20).align_x(Alignment::Center);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
