use std::fs;
use std::path::Path;

use iced::widget::{button, column, container, mouse_area, opaque, pick_list, row, scrollable, stack, text, text_input, toggler, Space};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};

use core::common::io::APP_LANGUAGES;
use core::modules::settings::{ExceptionList, ExceptionRule, RuleHandling};

use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::{popup, smooth_scroll, toggle_label};

use super::hover_hint;

const POPUP: popup::Spec = popup::Spec::new(popup::Kind::Exceptions, Size::new(750.0, 520.0));
const STEM_WIDTH: f32 = 265.0;
const EXTENSION_WIDTH: f32 = 135.0;
const HANDLING_WIDTH: f32 = 90.0;
const LANGUAGES_WIDTH: f32 = 100.0;
const DELETE_WIDTH: f32 = 44.0;

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
    ToggleLangDropdown(usize),
    CloseLangDropdown,
    LangDropdownIgnore,
    Import,
    ImportExpired,
    Export,
    ExportExpired,
    RequestReset,
    ResetExpired,
}

pub struct State {
    pub is_open: bool,
    popup: popup::State,
    rules: Vec<ExceptionRule>,
    import_feedback: Slot<bool>,
    export_feedback: Slot<bool>,
    confirm_reset: Slot<()>,
    open_lang_dropdown: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_open: false,
            popup: popup::State::default(),
            rules: ExceptionList::load_or_default().rules,
            import_feedback: Slot::default(),
            export_feedback: Slot::default(),
            confirm_reset: Slot::default(),
            open_lang_dropdown: None,
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
                if self.popup.update(msg, POPUP) {
                    self.is_open = false;
                    self.confirm_reset.clear();
                    self.open_lang_dropdown = None;
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
            Message::ToggleLangDropdown(index) => {
                self.open_lang_dropdown = if self.open_lang_dropdown == Some(index) { None } else { Some(index) };
                Task::none()
            }
            Message::CloseLangDropdown => {
                self.open_lang_dropdown = None;
                Task::none()
            }
            Message::LangDropdownIgnore => Task::none(),
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
                if self.confirm_reset.is_set() {
                    self.rules = ExceptionList::default().rules;
                    self.save();
                    self.confirm_reset.clear();
                    Task::none()
                } else {
                    self.confirm_reset.set((), Message::ResetExpired)
                }
            }
            Message::ResetExpired => {
                self.confirm_reset.expire();
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, window: Size) -> Element<'a, Message> {
        self.popup.view("Manage Exceptions", POPUP, window, Message::Popup, move || self.content_view(), None)
    }

    fn content_view<'a>(&'a self) -> Element<'a, Message> {
        let import_label = match self.import_feedback.get().copied() {
            Some(true) => "Imported!",
            Some(false) => "Failed!",
            None => "Load List",
        };
        let export_label = match self.export_feedback.get().copied() {
            Some(true) => "Exported!",
            Some(false) => "Failed!",
            None => "Export List",
        };

        let reset_label = self.confirm_reset.confirm_label("Set to Default");

        let actions = row![
            theme::sized_button("Add Rule", theme::POPUP_ACTION_BUTTON_WIDTH, theme::primary_button).on_press(Message::AddRule),
            theme::sized_button(import_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::feedback_button_style(self.import_feedback.get().copied())).on_press(Message::Import),
            theme::sized_button(export_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::feedback_button_style(self.export_feedback.get().copied())).on_press(Message::Export),
            theme::sized_button(reset_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::danger_button).on_press(Message::RequestReset),
        ].spacing(10);

        let header = container(
            row![
                theme::table_cell_text("Stem", Length::Fixed(STEM_WIDTH)).size(13),
                theme::table_cell_text("Extension", Length::Fixed(EXTENSION_WIDTH)).size(13),
                theme::table_cell_text("Handling", Length::Fixed(HANDLING_WIDTH)).size(13),
                theme::table_cell_text("Languages", Length::Fixed(LANGUAGES_WIDTH)).size(13),
                theme::table_cell_text("", Length::Fixed(DELETE_WIDTH)).size(13),
            ].spacing(10).width(Length::Fill)
        )
        .style(theme::zebra_table_header)
        .padding([6, 10])
        .width(Length::Fill);

        let mut rows = column![header].spacing(0).width(Length::Fill);

        for (index, rule) in self.rules.iter().enumerate() {
            let pattern_input = text_input("pattern", &rule.pattern)
                .on_input(move |v| Message::PatternChanged(index, v))
                .size(12)
                .width(Length::Fixed(STEM_WIDTH))
                .style(theme::rounded_input);

            let extension_input = text_input("ext", &rule.extension)
                .on_input(move |v| Message::ExtensionChanged(index, v))
                .size(12)
                .width(Length::Fixed(EXTENSION_WIDTH))
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
            ).width(Length::Fixed(HANDLING_WIDTH)).style(theme::combo_box).menu_style(theme::combo_box_menu);

            let enabled_count = rule.languages.values().filter(|&&enabled| enabled).count();
            let lang_button = button(theme::button_label(format!("Languages ({enabled_count})")).size(12))
                .width(Length::Fixed(LANGUAGES_WIDTH))
                .padding([6, 8])
                .style(move |theme: &Theme, status| theme::toggle_button(theme, status, Some(index) == self.open_lang_dropdown))
                .on_press(Message::ToggleLangDropdown(index));

            let delete_btn = hover_hint(
                button(text("🗑").size(14))
                    .on_press(Message::DeleteRule(index))
                    .style(button::danger),
                "Delete Rule",
            );

            rows = rows.push(
                container(
                    row![
                        pattern_input,
                        extension_input,
                        handling_pick,
                        lang_button,
                        delete_btn,
                    ].spacing(10).align_y(Alignment::Center).width(Length::Fill)
                )
                .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
                .padding([6, 10])
                .width(Length::Fill)
            );
        }

        let content = column![actions].spacing(15).padding(20).width(Length::Fill).height(Length::Fill).align_x(Alignment::Center);

        let content = content.push(
            smooth_scroll(
                scrollable(rows)
                    .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(8).margin(2)))
                    .height(Length::Fill)
            )
        );

        let base: Element<'a, Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        match self.open_lang_dropdown {
            Some(index) => stack![base, self.lang_dropdown_overlay(index)].into(),
            None => base,
        }
    }

    fn lang_dropdown_overlay<'a>(&'a self, index: usize) -> Element<'a, Message> {
        let Some(rule) = self.rules.get(index) else {
            return Space::new().into();
        };

        let mut list = row![].spacing(18).align_y(Alignment::Center);
        for &(lang_code, _) in APP_LANGUAGES {
            if let Some(&enabled) = rule.languages.get(lang_code) {
                let code = lang_code.to_string();
                list = list.push(
                    column![
                        toggle_label(
                            text(lang_code.to_uppercase()).size(12),
                            enabled,
                            Some({
                                let code = code.clone();
                                move |v| Message::LanguageToggled(index, code.clone(), v)
                            }),
                        ),
                        toggler(enabled).on_toggle(move |v| Message::LanguageToggled(index, code.clone(), v)).style(theme::ios_toggle),
                    ].spacing(6).align_x(Alignment::Center)
                );
            }
        }

        let card = mouse_area(
            container(
                column![
                    text("Languages").size(16),
                    smooth_scroll(scrollable(list).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(6)))),
                ].spacing(14).align_x(Alignment::Center)
            )
            .padding(20)
            .style(theme::card_container_outlined)
        ).on_press(Message::LangDropdownIgnore);

        let backdrop = opaque(
            mouse_area(
                container(Space::new().width(Length::Fill).height(Length::Fill))
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Color { a: 0.55, ..Color::BLACK }.into()),
                        ..container::Style::default()
                    })
            ).on_press(Message::CloseLangDropdown)
        );

        let panel = container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        stack![backdrop, panel].into()
    }
}
