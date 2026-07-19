use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, checkbox, column, container, image as iced_image, pick_list, row, scrollable,
    stack, text, text_input, tooltip, Space, Tooltip,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::cat::abilities::REGISTRY;
use nyanko::cat::unit::{Battle, LevelCurve, Talent, TalentCost, TalentGroup};
use tracing::{debug, error, info, trace, warn};

use core::common::game::CustomIcon;
use core::modules::cat::filter::{MatchMode, TalentFilterMode};
use core::modules::cat::game::registry::{AbilityIcon, DisplayGroup};
use core::modules::cat::scanner::CatEntry;
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
    ToggleFilter,
    ClearFilter,
    FilterMatchModeChanged(MatchMode),
    FilterTalentModeChanged(TalentFilterMode),
    FilterUltraTalentModeChanged(TalentFilterMode),
    FilterRarityToggled(usize),
    FilterFormToggled(usize),
    FilterIconToggled(AbilityIcon),
    SelectCat(u32),
    SelectForm(usize),
    SelectTab(DetailTab),
    LevelInputChanged(String),
    ToggleConjureExpand,
    ChangeTalentLevel(u8, u8),
    MaximizeTalents(bool),
    ExportStatblock(ExportAction),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tick => write!(f, "Tick"),
            Self::SearchChanged(s) => write!(f, "SearchChanged({})", s),
            Self::ToggleFilter => write!(f, "ToggleFilter"),
            Self::ClearFilter => write!(f, "ClearFilter"),
            Self::FilterMatchModeChanged(_) => write!(f, "FilterMatchModeChanged"),
            Self::FilterTalentModeChanged(_) => write!(f, "FilterTalentModeChanged"),
            Self::FilterUltraTalentModeChanged(_) => write!(f, "FilterUltraTalentModeChanged"),
            Self::FilterRarityToggled(i) => write!(f, "FilterRarityToggled({})", i),
            Self::FilterFormToggled(i) => write!(f, "FilterFormToggled({})", i),
            Self::FilterIconToggled(_) => write!(f, "FilterIconToggled"),
            Self::SelectCat(id) => write!(f, "SelectCat({})", id),
            Self::SelectForm(i) => write!(f, "SelectForm({})", i),
            Self::SelectTab(t) => write!(f, "SelectTab({:?})", t),
            Self::LevelInputChanged(s) => write!(f, "LevelInputChanged({})", s),
            Self::ToggleConjureExpand => write!(f, "ToggleConjureExpand"),
            Self::ChangeTalentLevel(i, l) => write!(f, "ChangeTalentLevel({}, {})", i, l),
            Self::MaximizeTalents(b) => write!(f, "MaximizeTalents({})", b),
            Self::ExportStatblock(e) => write!(f, "ExportStatblock({:?})", e),
        }
    }
}

pub struct State {
    pub cats: Vec<CatEntry>,
    pub selected_cat: Option<u32>,
    pub selected_form: usize,
    pub selected_tab: DetailTab,
    pub search_query: String,

    pub current_level: i32,
    pub level_input: String,
    pub is_conjure_expanded: bool,
    pub talent_levels: HashMap<u8, u8>,

    pub is_filter_open: bool,
    pub filter_match_mode: MatchMode,
    pub filter_talent_mode: TalentFilterMode,
    pub filter_ultra_talent_mode: TalentFilterMode,
    pub filter_rarities: [bool; 6],
    pub filter_forms: [bool; 4],
    pub filter_active_icons: HashSet<AbilityIcon>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            selected_cat: None,
            selected_form: 0,
            selected_tab: DetailTab::Abilities,
            search_query: String::new(),
            current_level: 1,
            level_input: String::from("1"),
            is_conjure_expanded: false,
            talent_levels: HashMap::new(),
            is_filter_open: false,
            filter_match_mode: MatchMode::Or,
            filter_talent_mode: TalentFilterMode::Ignore,
            filter_ultra_talent_mode: TalentFilterMode::Ignore,
            filter_rarities: [false; 6],
            filter_forms: [false; 4],
            filter_active_icons: HashSet::new(),
        }
    }
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::Tick => {
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::ToggleFilter => {
                self.is_filter_open = !self.is_filter_open;
                Task::none()
            }
            Message::ClearFilter => {
                self.filter_match_mode = MatchMode::Or;
                self.filter_talent_mode = TalentFilterMode::Ignore;
                self.filter_ultra_talent_mode = TalentFilterMode::Ignore;
                self.filter_rarities = [false; 6];
                self.filter_forms = [false; 4];
                self.filter_active_icons.clear();
                Task::none()
            }
            Message::FilterMatchModeChanged(mode) => {
                self.filter_match_mode = mode;
                Task::none()
            }
            Message::FilterTalentModeChanged(mode) => {
                self.filter_talent_mode = mode;
                Task::none()
            }
            Message::FilterUltraTalentModeChanged(mode) => {
                self.filter_ultra_talent_mode = mode;
                Task::none()
            }
            Message::FilterRarityToggled(index) => {
                if let Some(val) = self.filter_rarities.get_mut(index) {
                    *val = !*val;
                }
                Task::none()
            }
            Message::FilterFormToggled(index) => {
                if let Some(val) = self.filter_forms.get_mut(index) {
                    *val = !*val;
                }
                Task::none()
            }
            Message::FilterIconToggled(icon) => {
                if self.filter_active_icons.contains(&icon) {
                    self.filter_active_icons.remove(&icon);
                } else {
                    self.filter_active_icons.insert(icon);
                }
                Task::none()
            }
            Message::SelectCat(id) => {
                self.selected_cat = Some(id);
                self.selected_form = 0;
                self.selected_tab = DetailTab::Abilities;
                self.talent_levels.clear();

                if let Some(_cat) = self.cats.iter().find(|c| c.id == id) {
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
                    if let Some(cat) = self.cats.iter().find(|c| c.id == cat_id) {
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
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content();

        let base_layout = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        if self.is_filter_open {
            let filter_modal = self.view_filter_modal();
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
            .on_press(Message::ToggleFilter)
            .width(Length::Fill);

        let mut cat_list_col = column![].spacing(4).width(Length::Fill);

        for cat in &self.cats {
            let is_selected = self.selected_cat == Some(cat.id);

            let btn = button(text(format!("{} - {}", cat.id, cat.display_name(0))))
                .width(Length::Fill)
                .on_press(Message::SelectCat(cat.id));

            let styled_btn = if is_selected {
                btn // Applying theme logic directly requires wrapping via `.style()` if needed
            } else {
                btn
            };

            cat_list_col = cat_list_col.push(styled_btn);
        }

        let cat_scrollable = scrollable(cat_list_col)
            .height(Length::Fill)
            .width(Length::Fill);

        container(
            column![
                row![search_input, filter_button].spacing(4),
                Space::new().height(Length::Fixed(8.0)),
                cat_scrollable
            ]
                .spacing(4)
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

        let Some(cat) = self.cats.iter().find(|c| c.id == selected_id) else {
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

    fn view_filter_modal(&self) -> Element<Message> {
        let title = text("Advanced Cat Filter").size(24);

        let mut rarity_row = row![].spacing(8);
        let rarities = ["Normal", "Special", "Rare", "Super Rare", "Uber Rare", "Legend Rare"];
        for (i, label) in rarities.iter().enumerate() {
            let mut btn = button(*label);
            btn = btn.on_press(Message::FilterRarityToggled(i));
            rarity_row = rarity_row.push(btn);
        }

        let mut forms_row = row![].spacing(8);
        let forms = ["Normal Form", "Evolved Form", "True Form", "Ultra Form"];
        for (i, label) in forms.iter().enumerate() {
            let mut btn = button(*label);
            btn = btn.on_press(Message::FilterFormToggled(i));
            forms_row = forms_row.push(btn);
        }

        let selected_mode_str = if self.filter_match_mode == MatchMode::And { "And" } else { "Or" };

        let match_mode_picker = row![
            text("Mode:").align_y(Vertical::Center),
            pick_list(
                vec!["And", "Or"],
                Some(selected_mode_str),
                |s| if s == "And" { Message::FilterMatchModeChanged(MatchMode::And) } else { Message::FilterMatchModeChanged(MatchMode::Or) }
            )
        ].spacing(8);

        let clear_btn = button(text("Clear Filter").style(text::danger))
            .on_press(Message::ClearFilter)
            .padding([8, 16]);

        let close_btn = button("Close")
            .on_press(Message::ToggleFilter)
            .padding([8, 16]);

        let filter_content = column![
            title,
            Space::new().height(Length::Fixed(16.0)),
            text("Rarities").size(18),
            rarity_row,
            Space::new().height(Length::Fixed(12.0)),
            text("Forms").size(18),
            forms_row,
            Space::new().height(Length::Fixed(12.0)),
            match_mode_picker,
            Space::new().height(Length::Fixed(24.0)),
            row![clear_btn, close_btn].spacing(16)
        ].spacing(8).padding(24);

        container(scrollable(filter_content))
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(500.0))
            .style(container::bordered_box)
            .into()
    }
}