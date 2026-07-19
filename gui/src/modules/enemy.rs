use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::theme::{self, Palette};
use iced::widget::{
    button, column, container, image as iced_image, opaque, row, scrollable, stack, text, text_input, Button, Column, Container, Row, Scrollable, Space, Text
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::enemy::abilities::{Identity, REGISTRY};
use nyanko::graphics::rig::Unit;
use tracing::{debug, error, info, trace, warn};

use core::modules::enemy::filter::evaluation::{entity_passes_filter, get_identity_name};
use core::modules::enemy::filter::{EnemyFilterState, MatchMode, ATTACK_TYPE_IDENTITIES};
use core::modules::enemy::game::abilities::collect_ability_data;
use core::modules::enemy::game::registry::{
    format_enemy_stat, get_display_def, get_enemy_stat, AbilityIcon, DisplayGroup, Magnification,
};
use core::modules::enemy::game::EnemyRenderContext;
use core::modules::enemy::scanner::{self, EnemyEntry};
use core::modules::enemy::{EnemyDataState, EnemyDetailTab};
use core::modules::settings::Settings;

use crate::common::CustomAssets;
use crate::common::SpriteSheet;

#[derive(Debug, Clone)]
pub enum ExportAction {
    Copy,
    Save,
}

#[derive(Clone)]
pub enum Message {
    Tick,
    SearchQueryChanged(String),
    EnemySelected(u32),
    TabSelected(EnemyDetailTab),
    ToggleFilterModal,
    ClearFilters,
    FilterMatchModeToggled(MatchMode),
    FilterMagChanged(String),
    FilterIdentityToggled(Identity),
    FilterAdvMinChanged(Identity, &'static str, String),
    FilterAdvMaxChanged(Identity, &'static str, String),
    MagnificationChanged(String),
    ExportClicked(ExportAction),
    NavigateAppearances(u32),
    RequestIconLoad(u32, PathBuf),
    IconLoaded(u32, Option<iced::widget::image::Handle>),
}

// Implemented manually to prevent iced Handle struct Debug issues
// and to avoid deriving Debug on external structs we don't control.
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tick => write!(f, "Tick"),
            Self::SearchQueryChanged(s) => write!(f, "SearchQueryChanged({})", s),
            Self::EnemySelected(id) => write!(f, "EnemySelected({})", id),
            Self::TabSelected(tab) => {
                let tab_name = if *tab == EnemyDetailTab::Abilities {
                    "Abilities"
                } else if *tab == EnemyDetailTab::Details {
                    "Details"
                } else if *tab == EnemyDetailTab::Animation {
                    "Animation"
                } else {
                    "Unknown"
                };
                write!(f, "TabSelected({})", tab_name)
            }
            Self::ToggleFilterModal => write!(f, "ToggleFilterModal"),
            Self::ClearFilters => write!(f, "ClearFilters"),
            Self::FilterMatchModeToggled(_) => write!(f, "FilterMatchModeToggled"),
            Self::FilterMagChanged(s) => write!(f, "FilterMagChanged({})", s),
            Self::FilterIdentityToggled(_) => write!(f, "FilterIdentityToggled"),
            Self::FilterAdvMinChanged(_, _, s) => write!(f, "FilterAdvMinChanged({})", s),
            Self::FilterAdvMaxChanged(_, _, s) => write!(f, "FilterAdvMaxChanged({})", s),
            Self::MagnificationChanged(s) => write!(f, "MagnificationChanged({})", s),
            Self::ExportClicked(_) => write!(f, "ExportClicked"),
            Self::NavigateAppearances(id) => write!(f, "NavigateAppearances({})", id),
            Self::RequestIconLoad(id, _) => write!(f, "RequestIconLoad({})", id),
            Self::IconLoaded(id, _) => write!(f, "IconLoaded({})", id),
        }
    }
}

pub struct EnemyState {
    pub data: EnemyDataState,
    pub filter_state: EnemyFilterState,

    // UI State
    pub search_query: String,
    pub mag_input: String,
    pub magnification: Magnification,
    pub selected_tab: EnemyDetailTab,
    pub show_filter_modal: bool,

    // Caches
    pub texture_cache: HashMap<u32, iced::widget::image::Handle>,
    pub pending_requests: HashSet<u32>,
    pub missing_ids: HashSet<u32>,

    // External Context
    pub img015_sheets: Vec<SpriteSheet>,
    pub custom_assets: Option<CustomAssets>,
    pub rig: Option<Arc<Unit>>,
}

impl Default for EnemyState {
    fn default() -> Self {
        Self {
            data: EnemyDataState::default(),
            filter_state: EnemyFilterState::default(),
            search_query: String::new(),
            mag_input: "100".to_string(),
            magnification: Magnification { hitpoints: 100, attack: 100 },
            selected_tab: EnemyDetailTab::Abilities,
            show_filter_modal: false,
            texture_cache: HashMap::new(),
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            img015_sheets: Vec::new(),
            custom_assets: None,
            rig: None,
        }
    }
}

impl EnemyState {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.data.scan_receiver.is_some() {
                    self.data.update_data();
                }
                Task::none()
            }
            Message::SearchQueryChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::EnemySelected(id) => {
                if self.data.selected_enemy != Some(id) {
                    self.data.selected_enemy = Some(id);
                    self.rig = None;
                    self.mag_input = "100".to_string();
                    self.magnification = Magnification { hitpoints: 100, attack: 100 };
                }
                Task::none()
            }
            Message::TabSelected(tab) => {
                self.selected_tab = tab;
                Task::none()
            }
            Message::ToggleFilterModal => {
                self.show_filter_modal = !self.show_filter_modal;
                Task::none()
            }
            Message::ClearFilters => {
                self.filter_state = EnemyFilterState::default();
                Task::none()
            }
            Message::FilterMatchModeToggled(mode) => {
                self.filter_state.match_mode = mode;
                Task::none()
            }
            Message::FilterMagChanged(mag) => {
                self.filter_state.mag_input = mag;
                Task::none()
            }
            Message::FilterIdentityToggled(identity) => {
                if self.filter_state.active_identities.contains(&identity) {
                    self.filter_state.active_identities.remove(&identity);
                } else {
                    self.filter_state.active_identities.insert(identity);
                }
                Task::none()
            }
            Message::FilterAdvMinChanged(identity, attr, val) => {
                let range = self.filter_state.adv_ranges
                    .entry(identity)
                    .or_default()
                    .entry(attr)
                    .or_default();
                range.min = val;
                Task::none()
            }
            Message::FilterAdvMaxChanged(identity, attr, val) => {
                let range = self.filter_state.adv_ranges
                    .entry(identity)
                    .or_default()
                    .entry(attr)
                    .or_default();
                range.max = val;
                Task::none()
            }
            Message::MagnificationChanged(input) => {
                self.mag_input = input.clone();
                let text = input.trim();
                let parts: Vec<&str> = text.split(['/', '|', '\\']).collect();

                if parts.len() >= 2 {
                    let hp = parts[0].trim().parse::<i32>().unwrap_or(100);
                    let atk = parts[1].trim().parse::<i32>().unwrap_or(hp);
                    self.magnification = Magnification { hitpoints: hp, attack: atk };
                } else {
                    let mag = text.parse::<i32>().unwrap_or(100);
                    self.magnification = Magnification { hitpoints: mag, attack: mag };
                }
                Task::none()
            }
            Message::ExportClicked(action) => {
                match action {
                    ExportAction::Copy => info!("Export requested: Copy (Statblock WIP)"),
                    ExportAction::Save => info!("Export requested: Save (Statblock WIP)"),
                }
                Task::none()
            }
            Message::NavigateAppearances(id) => {
                info!("Navigating to stage appearances for enemy {}", id);
                Task::none()
            }
            Message::RequestIconLoad(id, _path) => {
                if !self.pending_requests.contains(&id) {
                    self.pending_requests.insert(id);
                    return Task::perform(async move {
                        Message::IconLoaded(id, None)
                    }, |m| m);
                }
                Task::none()
            }
            Message::IconLoaded(id, handle_opt) => {
                self.pending_requests.remove(&id);
                if let Some(handle) = handle_opt {
                    self.texture_cache.insert(id, handle);
                } else {
                    self.missing_ids.insert(id);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let content = row![
            self.view_sidebar(),
            self.view_main_panel(),
        ]
            .width(Length::Fill)
            .height(Length::Fill);

        if self.show_filter_modal {
            let modal = opaque(
                container(self.view_filter_modal())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|theme: &Theme| {
                        let palette = theme.palette();
                        container::Style {
                            background: Some(iced::Color { a: 0.8, ..palette.background }.into()),
                            ..Default::default()
                        }
                    })
            );

            stack![content, modal].into()
        } else {
            content.into()
        }
    }

    fn view_sidebar(&self) -> Element<Message> {
        let search_bar = row![
            text_input("Search Enemy...", &self.search_query)
                .on_input(Message::SearchQueryChanged)
                .width(Length::Fill),
            button(text("Filter"))
                .on_press(Message::ToggleFilterModal)
                .style(if self.filter_state.is_active() { iced::widget::button::primary } else { iced::widget::button::secondary })
        ]
            .spacing(5)
            .padding(5);

        let mut list_col = column![].spacing(2).width(Length::Fill);

        let query_lower = self.search_query.to_lowercase();
        let is_empty = query_lower.is_empty();

        for entry in &self.data.enemies {
            if !entity_passes_filter(entry, &self.filter_state) {
                continue;
            }

            let full_id = entry.id_str().to_lowercase();
            let is_id_search = query_lower.chars().next().map_or(false, |c| c.is_ascii_digit());

            let matches_search = is_empty
                || (is_id_search && full_id.contains(&query_lower))
                || entry.name.to_lowercase().contains(&query_lower);

            if matches_search {
                let is_selected = self.data.selected_enemy == Some(entry.id);

                let btn = button(
                    row![
                        text(entry.id_str()).size(12),
                        text(entry.display_name()).size(14)
                    ].spacing(8).align_y(Alignment::Center)
                )
                    .width(Length::Fill)
                    .style(if is_selected { iced::widget::button::primary } else { iced::widget::button::secondary })
                    .on_press(Message::EnemySelected(entry.id));

                list_col = list_col.push(btn);
            }
        }

        container(
            column![
                search_bar,
                scrollable(list_col).height(Length::Fill)
            ]
        )
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 1.0,
                        color: theme.palette().text,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_main_panel(&self) -> Element<Message> {
        let Some(selected_id) = self.data.selected_enemy else {
            return container(text("Select an Enemy").size(24))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let Some(enemy_entry) = self.data.enemies.iter().find(|e| e.id == selected_id) else {
            return container(text("Loading...").size(24))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let header = self.view_header(enemy_entry);

        let content = match self.selected_tab {
            EnemyDetailTab::Abilities => {
                column![
                    self.view_stats(enemy_entry),
                    Space::new().height(Length::Fixed(10.0)),
                    self.view_abilities(enemy_entry)
                ].into()
            }
            EnemyDetailTab::Details => {
                self.view_details(enemy_entry)
            }
            EnemyDetailTab::Animation => {
                column![text("Animation Viewer Placeholder (Phase 1 Constraint)").size(18)].into()
            }
        };

        container(
            column![
                header,
                Space::new().height(Length::Fixed(10.0)),
                scrollable(content).height(Length::Fill)
            ]
                .padding(15)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_header(&self, enemy: &EnemyEntry) -> Element<Message> {
        let tabs = row![
            button("Abilities").on_press(Message::TabSelected(EnemyDetailTab::Abilities))
                .style(if self.selected_tab == EnemyDetailTab::Abilities { iced::widget::button::primary } else { iced::widget::button::secondary }),
            button("Details").on_press(Message::TabSelected(EnemyDetailTab::Details))
                .style(if self.selected_tab == EnemyDetailTab::Details { iced::widget::button::primary } else { iced::widget::button::secondary }),
            button("Animation").on_press(Message::TabSelected(EnemyDetailTab::Animation))
                .style(if self.selected_tab == EnemyDetailTab::Animation { iced::widget::button::primary } else { iced::widget::button::secondary }),
        ].spacing(10);

        let info_box = column![
            text(enemy.display_name()).size(24),
            text(format!("ID: {:03}-E", enemy.id)).size(14),
            row![
                text("Magnification:"),
                text_input("100", &self.mag_input)
                    .on_input(Message::MagnificationChanged)
                    .width(Length::Fixed(60.0)),
                text("%")
            ].spacing(5).align_y(Alignment::Center)
        ].spacing(5);

        let mut actions = row![].spacing(10);
        if self.selected_tab == EnemyDetailTab::Abilities {
            actions = actions.push(button("Copy Image").on_press(Message::ExportClicked(ExportAction::Copy)));
            actions = actions.push(button("Export Image").on_press(Message::ExportClicked(ExportAction::Save)));
        }
        actions = actions.push(button("Appearances").on_press(Message::NavigateAppearances(enemy.id)));

        column![
            tabs,
            Space::new().height(Length::Fixed(15.0)),
            row![
                info_box,
                Space::new().width(Length::Fill),
                actions
            ].align_y(Alignment::Center)
        ].into()
    }

    fn view_stats(&self, enemy: &EnemyEntry) -> Element<Message> {
        let stats = &enemy.stats;
        let frames = enemy.atk_anim_frames;
        let mag = self.magnification;

        let atk_str = format_enemy_stat("Attack", stats, frames, mag);
        let dps_str = format_enemy_stat("Dps", stats, frames, mag);
        let range_str = format_enemy_stat("Range", stats, frames, mag);
        let cycle = (get_enemy_stat("Atk Cycle").get_value)(stats, frames, mag);

        let hp_str = format_enemy_stat("Hitpoints", stats, frames, mag);
        let kb_str = format_enemy_stat("Knockbacks", stats, frames, mag);
        let speed_str = format_enemy_stat("Speed", stats, frames, mag);
        let cash_str = format_enemy_stat("Cash Drop", stats, frames, mag);

        column![
            row![
                self.stat_cell(get_enemy_stat("Attack").display_name.to_string(), atk_str),
                self.stat_cell(get_enemy_stat("Dps").display_name.to_string(), dps_str),
                self.stat_cell(get_enemy_stat("Range").display_name.to_string(), range_str),
                self.stat_cell(get_enemy_stat("Atk Cycle").display_name.to_string(), format!("{}f", cycle)),
            ].spacing(10),
            Space::new().height(Length::Fixed(10.0)),
            row![
                self.stat_cell(get_enemy_stat("Hitpoints").display_name.to_string(), hp_str),
                self.stat_cell(get_enemy_stat("Knockbacks").display_name.to_string(), kb_str),
                self.stat_cell(get_enemy_stat("Speed").display_name.to_string(), speed_str),
                self.stat_cell(get_enemy_stat("Cash Drop").display_name.to_string(), cash_str),
            ].spacing(10)
        ].into()
    }

    fn stat_cell<'a>(&self, label: String, value: String) -> Element<'a, Message> {
        container(
            column![
                text(label).size(12),
                text(value).size(16)
            ].align_x(Alignment::Center)
        )
            .width(Length::Fixed(120.0))
            .padding(5)
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 1.0,
                        color: theme.palette().text,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_abilities(&self, _enemy: &EnemyEntry) -> Element<Message> {
        column![
            text("Abilities Section").size(20),
            text("Detailed ability icons and text parsed from core::modules::enemy::game::abilities go here.")
        ].into()
    }

    fn view_details(&self, enemy: &EnemyEntry) -> Element<Message> {
        let mut col = column![text("Description").size(20)].spacing(10).align_x(Alignment::Center);

        if enemy.description.is_empty() {
            col = col.push(text("No description available"));
        } else {
            for line in &enemy.description {
                if line.trim().is_empty() {
                    col = col.push(Space::new().height(Length::Fixed(10.0)));
                } else {
                    col = col.push(text(line.clone()).size(15));
                }
            }
        }

        container(col).width(Length::Fill).into()
    }

    fn view_filter_modal(&self) -> Element<Message> {
        let match_mode_row = row![
            text("Mode:"),
            button(if self.filter_state.match_mode == MatchMode::And { "And" } else { "Or" })
                .on_press(Message::FilterMatchModeToggled(
                    if self.filter_state.match_mode == MatchMode::And { MatchMode::Or } else { MatchMode::And }
                ))
        ].spacing(10).align_y(Alignment::Center);

        let mag_row = row![
            text("Target Magnification:"),
            text_input("100", &self.filter_state.mag_input).on_input(Message::FilterMagChanged).width(Length::Fixed(60.0)),
            text("%")
        ].spacing(10).align_y(Alignment::Center);

        let actions = row![
            button("Clear Filter").on_press(Message::ClearFilters).style(iced::widget::button::danger),
            button("Close").on_press(Message::ToggleFilterModal).style(iced::widget::button::primary)
        ].spacing(15);

        let content = column![
            text("Advanced Enemy Filter").size(24),
            Space::new().height(Length::Fixed(10.0)),
            match_mode_row,
            mag_row,
            Space::new().height(Length::Fixed(20.0)),
            text("Trait & Attack Types... (Filter groups parsed from REGISTRY)"),
            Space::new().height(Length::Fixed(20.0)),
            actions
        ].spacing(10).padding(20);

        container(scrollable(content))
            .width(Length::Fixed(450.0))
            .height(Length::Fixed(500.0))
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 2.0,
                        color: theme.palette().text,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}