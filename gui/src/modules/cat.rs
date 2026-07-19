mod filter;
mod list;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, checkbox, column, container, image as iced_image, row, scrollable,
    stack, text, text_input, tooltip, Space, Tooltip,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::cat::unit::{Battle, LevelCurve, Talent, TalentCost, TalentGroup};
use tracing::{debug, error, info, trace, warn};

use core::modules::cat::scanner::CatEntry;
use core::modules::cat::CatDataState;
use core::modules::settings::Settings;

use crate::common::CustomAssets;
use crate::common::SpriteSheet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Abilities,
    Talents,
    Details,
    Animation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportAction {
    Copy,
    Save,
}

#[derive(Clone)]
pub enum Message {
    Tick,
    SearchChanged(String),
    SelectCat(u32),
    SelectForm(usize),
    SelectTab(DetailTab),
    LevelInputChanged(String),
    ToggleConjureExpand,
    ChangeTalentLevel(u8, u8),
    MaximizeTalents(bool),
    ExportStatblock(ExportAction),
    List(list::Message),
    Filter(filter::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tick => write!(f, "Tick"),
            Self::SearchChanged(s) => write!(f, "SearchChanged({})", s),
            Self::SelectCat(id) => write!(f, "SelectCat({})", id),
            Self::SelectForm(i) => write!(f, "SelectForm({})", i),
            Self::SelectTab(t) => write!(f, "SelectTab({:?})", t),
            Self::LevelInputChanged(s) => write!(f, "LevelInputChanged({})", s),
            Self::ToggleConjureExpand => write!(f, "ToggleConjureExpand"),
            Self::ChangeTalentLevel(i, l) => write!(f, "ChangeTalentLevel({}, {})", i, l),
            Self::MaximizeTalents(b) => write!(f, "MaximizeTalents({})", b),
            Self::ExportStatblock(e) => write!(f, "ExportStatblock({:?})", e),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
        }
    }
}

pub struct State {
    pub data: CatDataState,
    pub selected_cat: Option<u32>,
    pub selected_form: usize,
    pub selected_tab: DetailTab,
    pub search_query: String,

    pub current_level: i32,
    pub level_input: String,
    pub is_conjure_expanded: bool,
    pub talent_levels: HashMap<u8, u8>,

    list: list::State,
    filter: filter::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            data: CatDataState::default(),
            selected_cat: None,
            selected_form: 0,
            selected_tab: DetailTab::Abilities,
            search_query: String::new(),
            current_level: 1,
            level_input: String::from("1"),
            is_conjure_expanded: false,
            talent_levels: HashMap::new(),

            list: list::State::default(),
            filter: filter::State::default(),
        }
    }
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        let task = self.update_inner(message, settings);

        self.list.refresh(&self.data.cats, &self.search_query, &self.filter.filter_state);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::Tick => {
                if !self.data.initialized {
                    self.data.initialized = true;
                    info!("Triggering initial cat scan");
                    self.data.restart_scan(settings.scanner_config());
                } else if self.data.scan_receiver.is_some() {
                    self.data.update_data();
                }

                self.list
                    .update(list::Message::Tick, &self.data.cats, &self.search_query, &self.filter.filter_state, settings.cat_data.high_banner_quality)
                    .map(Message::List)
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::SelectCat(id) => {
                self.selected_cat = Some(id);
                self.selected_form = 0;
                self.selected_tab = DetailTab::Abilities;
                self.talent_levels.clear();

                if let Some(_cat) = self.data.cats.iter().find(|c| c.id == id) {
                    let default_level = settings.cat_data.default_level.max(1);
                    self.current_level = default_level;
                    self.level_input = default_level.to_string();
                }

                info!("Selected cat ID: {}", id);
                Task::none()
            }
            Message::SelectForm(form_idx) => {
                self.selected_form = form_idx;

                if self.selected_tab == DetailTab::Talents && form_idx < 2 {
                    self.selected_tab = DetailTab::Abilities;
                }
                Task::none()
            }
            Message::SelectTab(tab) => {
                self.selected_tab = tab;
                Task::none()
            }
            Message::LevelInputChanged(input) => {
                self.level_input = input;
                let parsed: i32 = self.level_input.split('+')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .sum();
                self.current_level = if parsed <= 0 { 1 } else { parsed };
                Task::none()
            }
            Message::ToggleConjureExpand => {
                self.is_conjure_expanded = !self.is_conjure_expanded;
                Task::none()
            }
            Message::ChangeTalentLevel(index, level) => {
                self.talent_levels.insert(index, level);
                Task::none()
            }
            Message::MaximizeTalents(is_ultra) => {
                if let Some(cat_id) = self.selected_cat {
                    if let Some(cat) = self.data.cats.iter().find(|c| c.id == cat_id) {
                        if let Some(talent_data) = &cat.talent_data {
                            for (index, group) in talent_data.groups.iter().enumerate() {
                                let target_group = if is_ultra { group.limit == 1 } else { group.limit != 1 };
                                if target_group {
                                    self.talent_levels.insert(index as u8, group.max_level.max(1));
                                }
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ExportStatblock(action) => {
                match action {
                    ExportAction::Copy => info!("ExportAction: Copying to clipboard"),
                    ExportAction::Save => info!("ExportAction: Saving to disk"),
                }
                Task::none()
            }
            Message::List(msg) => {
                if let list::Message::SelectCat(id) = msg {
                    return self.update(Message::SelectCat(id), settings);
                }

                self.list
                    .update(msg, &self.data.cats, &self.search_query, &self.filter.filter_state, settings.cat_data.high_banner_quality)
                    .map(Message::List)
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content();

        let base_layout = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        if self.filter.filter_state.is_open {
            let filter_modal = self.filter.view().map(Message::Filter);
            stack![
                base_layout,
                container(filter_modal)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(container::transparent)
            ].into()
        } else {
            base_layout.into()
        }
    }

    fn view_sidebar(&self) -> Element<Message> {
        let search_input = text_input("Search Cat...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(8)
            .width(Length::Fill);

        let filter_button = button(text("Filter").align_x(Horizontal::Center))
            .on_press(Message::Filter(filter::Message::Toggle))
            .width(Length::Fill);

        let cat_list = self.list.view(&self.data.cats, self.selected_cat).map(Message::List);

        container(
            column![
                row![search_input, filter_button].spacing(4),
                Space::new().height(Length::Fixed(8.0)),
                cat_list
            ]
                .spacing(4)
                .height(Length::Fill)
        )
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .padding(8)
            .style(container::bordered_box)
            .into()
    }

    fn view_main_content(&self) -> Element<Message> {
        let Some(selected_id) = self.selected_cat else {
            return container(text("Select a Unit").size(24))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let Some(cat) = self.data.cats.iter().find(|c| c.id == selected_id) else {
            return container(text("Loading Unit Data..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let header = self.view_header(cat);

        let content = match self.selected_tab {
            DetailTab::Abilities => self.view_abilities(cat),
            DetailTab::Talents => self.view_talents(cat),
            DetailTab::Details => self.view_details(cat),
            DetailTab::Animation => container(text("Animation Viewer Placeholder")).center_x(Length::Fill).center_y(Length::Fill).into(),
        };

        column![
            header,
            Space::new().height(Length::Fixed(16.0)),
            content
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .into()
    }

    fn view_header(&self, cat: &CatEntry) -> Element<Message> {
        let mut form_row = row![].spacing(8);
        let form_labels = ["Normal", "Evolved", "True", "Ultra"];

        for (i, label) in form_labels.iter().enumerate() {
            let exists = cat.forms.get(i).copied().unwrap_or(false);
            if exists {
                let mut btn = button(text(*label)).padding([4, 12]);
                if self.selected_form != i {
                    btn = btn.on_press(Message::SelectForm(i));
                }
                form_row = form_row.push(btn);
            }
        }

        let copy_btn = button("Copy Image")
            .on_press(Message::ExportStatblock(ExportAction::Copy));
        let save_btn = button("Export Image")
            .on_press(Message::ExportStatblock(ExportAction::Save));

        let mut tab_row = row![].spacing(8);
        let tabs = [
            (DetailTab::Abilities, "Abilities"),
            (DetailTab::Talents, "Talents"),
            (DetailTab::Details, "Details"),
            (DetailTab::Animation, "Animation"),
        ];

        for (tab_enum, label) in tabs {
            let is_talents = tab_enum == DetailTab::Talents;
            let enabled = if is_talents {
                self.selected_form >= 2 && cat.talent_data.is_some()
            } else {
                true
            };

            if enabled {
                let mut btn = button(text(label)).padding([4, 12]);
                if self.selected_tab != tab_enum {
                    btn = btn.on_press(Message::SelectTab(tab_enum));
                }
                tab_row = tab_row.push(btn);
            }
        }

        let level_row = row![
            text("Level:").align_y(Vertical::Center),
            text_input("Level", &self.level_input)
                .on_input(Message::LevelInputChanged)
                .width(Length::Fixed(60.0))
        ].spacing(8).align_y(Vertical::Center);

        column![
            row![form_row, Space::new().width(Length::Fill), copy_btn, save_btn].spacing(8),
            Space::new().height(Length::Fixed(16.0)),
            row![
                text(format!("ID: {:03}-{}", cat.id, self.selected_form + 1)).size(12),
                Space::new().width(Length::Fill),
                level_row
            ],
            Space::new().height(Length::Fixed(16.0)),
            tab_row
        ].into()
    }

    fn view_abilities(&self, _cat: &CatEntry) -> Element<Message> {
        scrollable(
            column![
                text("Traits").size(18),
                row![text("Trait Icon Placeholder")].spacing(4),
                Space::new().height(Length::Fixed(16.0)),
                text("Abilities").size(18),
                row![text("Ability Icon Placeholder")].spacing(4),
            ].spacing(8)
        ).into()
    }

    fn view_talents(&self, cat: &CatEntry) -> Element<Message> {
        let Some(talent_data) = &cat.talent_data else {
            return container(text("No Talents Available")).into();
        };

        let mut talent_col = column![].spacing(12);

        let normal_talents_btn = button("Max Normal Talents")
            .on_press(Message::MaximizeTalents(false));
        let ultra_talents_btn = button("Max Ultra Talents")
            .on_press(Message::MaximizeTalents(true));

        talent_col = talent_col.push(row![normal_talents_btn, ultra_talents_btn].spacing(8));

        for (index, group) in talent_data.groups.iter().enumerate() {
            let current_lvl = *self.talent_levels.get(&(index as u8)).unwrap_or(&0);

            let max_lvl = group.max_level.max(1);
            let next_lvl = if current_lvl < max_lvl { current_lvl + 1 } else { max_lvl };
            let prev_lvl = if current_lvl > 0 { current_lvl - 1 } else { 0 };

            let group_row = row![
                text(format!("Talent {}", group.ability_id)).width(Length::Fixed(120.0)),
                button("-").on_press(Message::ChangeTalentLevel(index as u8, prev_lvl)),
                text(format!("{}/{}", current_lvl, max_lvl)),
                button("+").on_press(Message::ChangeTalentLevel(index as u8, next_lvl)),
            ].spacing(8).align_y(Vertical::Center);

            talent_col = talent_col.push(container(group_row).padding(8).style(container::bordered_box));
        }

        scrollable(talent_col).into()
    }

    fn view_details(&self, cat: &CatEntry) -> Element<Message> {
        let description = cat.description.get(self.selected_form)
            .and_then(|d| d.as_ref())
            .map(|lines| lines.join("\n"))
            .unwrap_or_else(|| "No description available".to_string());

        scrollable(
            column![
                text("Description").size(20),
                text(description),
            ].spacing(16)
        ).into()
    }
}