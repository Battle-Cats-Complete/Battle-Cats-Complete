mod abilities;
mod filter;
mod list;
mod statblock;

use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::enemy::unit::Battle;
use nyanko::graphics::rig::Unit;
use tracing::{error, info};

use core::common::context::GlobalContext;
use core::modules::enemy::game::registry::{format_enemy_stat, get_enemy_stat, Magnification};
use core::modules::enemy::game::EnemyRenderContext;
use core::modules::enemy::scanner::{self, EnemyEntry};
use core::modules::enemy::{EnemyDataState, EnemyDetailTab};
use core::modules::settings::Settings;

use crate::common::stat_grid;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::modules::statblock::button_feedback;

use super::statblock::builder;
use statblock::build_enemy_statblock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    MagnificationChanged(String),
    ExportClicked(ExportAction),
    NavigateAppearances(u32),
    Filter(filter::Message),
    List(list::Message),
}

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
            Self::MagnificationChanged(s) => write!(f, "MagnificationChanged({})", s),
            Self::ExportClicked(_) => write!(f, "ExportClicked"),
            Self::NavigateAppearances(id) => write!(f, "NavigateAppearances({})", id),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
            Self::List(msg) => write!(f, "List({:?})", msg),
        }
    }
}

pub struct EnemyState {
    pub data: EnemyDataState,
    pub search_query: String,
    pub mag_input: String,
    pub magnification: Magnification,
    pub selected_tab: EnemyDetailTab,
    pub img015_sheets: Vec<SpriteSheet>,
    pub custom_assets: CustomAssets,
    pub rig: Option<Arc<Unit>>,

    statblock_pending: Option<ExportAction>,
    statblock_job: Option<mpsc::Receiver<(ExportAction, Result<(), String>)>>,
    statblock_copy_feedback: Option<(bool, Instant)>,
    statblock_save_feedback: Option<(bool, Instant)>,

    filter: filter::State,
    list: list::State,
    abilities: abilities::State,
}

impl Default for EnemyState {
    fn default() -> Self {
        Self {
            data: EnemyDataState::default(),
            search_query: String::new(),
            mag_input: "100".to_string(),
            magnification: Magnification { hitpoints: 100, attack: 100 },
            selected_tab: EnemyDetailTab::Abilities,
            img015_sheets: Vec::new(),
            custom_assets: CustomAssets::new(),
            rig: None,

            statblock_pending: None,
            statblock_job: None,
            statblock_copy_feedback: None,
            statblock_save_feedback: None,

            filter: filter::State::default(),
            list: list::State::default(),
            abilities: abilities::State::default(),
        }
    }
}

impl EnemyState {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        let task = self.update_inner(message, settings, global_ctx);

        self.list.refresh(&self.data.enemies, &self.search_query, &self.filter.filter_state);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        match message {
            Message::Tick => {
                if !self.data.initialized {
                    self.data.initialized = true;
                    info!("Triggering initial enemy scan");
                    self.data.restart_scan(settings.scanner_config());
                } else if self.data.scan_receiver.is_some() {
                    self.data.update_data();
                }

                crate::common::img015::ensure_loaded(&mut self.img015_sheets, settings);

                let received = self.statblock_job.as_ref().and_then(|rx| rx.try_recv().ok());
                if let Some((action, result)) = received {
                    self.statblock_pending = None;
                    self.statblock_job = None;
                    let success = result.is_ok();
                    match action {
                        ExportAction::Copy => self.statblock_copy_feedback = Some((success, Instant::now())),
                        ExportAction::Save => self.statblock_save_feedback = Some((success, Instant::now())),
                    }
                }

                self.list
                    .update(list::Message::Tick, &self.data.enemies, &self.search_query, &self.filter.filter_state)
                    .map(Message::List)
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
                self.start_statblock_export(action, settings, global_ctx);
                Task::none()
            }
            Message::NavigateAppearances(id) => {
                info!("Navigating to stage appearances for enemy {}", id);
                Task::none()
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
            Message::List(msg) => {
                if let list::Message::SelectEnemy(id) = msg {
                    return self.update(Message::EnemySelected(id), settings, global_ctx);
                }

                self.list
                    .update(msg, &self.data.enemies, &self.search_query, &self.filter.filter_state)
                    .map(Message::List)
            }
        }
    }

    fn start_statblock_export(&mut self, action: ExportAction, settings: &Settings, global_ctx: GlobalContext<'_>) {
        if self.statblock_pending.is_some() {
            return;
        }

        let Some(selected_id) = self.data.selected_enemy else { return; };
        let Some(enemy_entry) = self.data.enemies.iter().find(|e| e.id == selected_id) else { return; };

        let dynamic_entry = scanner::scan_single(enemy_entry.id, &settings.scanner_config());
        let stats = dynamic_entry.as_ref().map(|e| &e.stats).unwrap_or(&enemy_entry.stats);

        let ctx = EnemyRenderContext {
            global: global_ctx,
            stats,
            magnification: self.magnification,
        };

        let data = build_enemy_statblock(&ctx, enemy_entry);
        let is_cat = data.is_cat;
        let id_str = data.id_str.clone();
        let top_value = data.top_value.clone();

        let mut cuts_map = std::collections::HashMap::new();
        for sheet in self.img015_sheets.iter().rev() {
            cuts_map.extend(sheet.core.cuts_map.clone());
        }
        let priority = settings.general.language_priority.clone();

        let (tx, rx) = mpsc::channel();
        self.statblock_job = Some(rx);
        self.statblock_pending = Some(action);

        std::thread::spawn(move || {
            let result = builder::build_statblock_image(&priority, data, cuts_map)
                .and_then(|image| match action {
                    ExportAction::Copy => builder::copy_to_clipboard(&image),
                    ExportAction::Save => builder::save_to_disk(&image, is_cat, &id_str, &top_value).map(|_| ()),
                });

            if let Err(err) = &result {
                error!("Enemy statblock export failed: {err}");
            }

            let _ = tx.send((action, result));
        });
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        row![
            self.view_sidebar(),
            self.view_main_panel(settings, global_ctx),
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn filter_popup_open(&self) -> bool {
        self.filter.filter_state.is_open
    }

    pub fn filter_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.filter
            .filter_state
            .is_open
            .then(|| self.filter.view(&self.img015_sheets, &self.custom_assets, window).map(Message::Filter))
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let search_bar = row![
            text_input("Search Enemy...", &self.search_query)
                .on_input(Message::SearchQueryChanged)
                .width(Length::Fill),
            button(text("Filter"))
                .on_press(Message::Filter(filter::Message::Toggle))
                .style(if self.filter.filter_state.is_active() { iced::widget::button::primary } else { iced::widget::button::secondary })
        ]
            .spacing(5)
            .padding(5);

        let enemy_list = self.list.view(&self.data.enemies, self.data.selected_enemy).map(Message::List);

        container(
            column![
                search_bar,
                enemy_list
            ]
                .height(Length::Fill)
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

    fn view_main_panel<'a>(&'a self, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
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
            EnemyDetailTab::Abilities => self.view_abilities(enemy_entry, settings, global_ctx),
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

    fn view_header(&self, enemy: &EnemyEntry) -> Element<'_, Message> {
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
            let (copy_label, copy_color) = button_feedback(
                self.statblock_pending == Some(ExportAction::Copy),
                self.statblock_copy_feedback,
                "Copy Image", "Copying...", "Copied!", "Failed!",
            );
            let copy_btn = button(text(copy_label).size(12))
                .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportClicked(ExportAction::Copy)))
                .style(move |_theme: &Theme, _status| button::Style {
                    background: Some(Background::Color(copy_color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(4.0),
                    ..Default::default()
                });

            let (save_label, save_color) = button_feedback(
                self.statblock_pending == Some(ExportAction::Save),
                self.statblock_save_feedback,
                "Export Image", "Exporting...", "Exported!", "Failed!",
            );
            let save_btn = button(text(save_label).size(12))
                .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportClicked(ExportAction::Save)))
                .style(move |_theme: &Theme, _status| button::Style {
                    background: Some(Background::Color(save_color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(4.0),
                    ..Default::default()
                });

            actions = actions.push(copy_btn);
            actions = actions.push(save_btn);
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

    fn view_abilities<'a>(&'a self, enemy: &'a EnemyEntry, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let dynamic_entry = scanner::scan_single(enemy.id, &settings.scanner_config());
        let stats = dynamic_entry.as_ref().map(|e| &e.stats).unwrap_or(&enemy.stats);

        let ctx = EnemyRenderContext {
            global: global_ctx,
            stats,
            magnification: self.magnification,
        };

        column![
            self.view_stats(enemy, stats),
            Space::new().height(Length::Fixed(8.0)),
            self.abilities.view(&ctx, &self.img015_sheets, &self.custom_assets)
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_stats(&self, enemy: &EnemyEntry, stats: &Battle) -> Element<'_, Message> {
        let frames = enemy.atk_anim_frames;
        let mag = self.magnification;

        let atk_str = format_enemy_stat("Attack", stats, frames, mag);
        let dps_str = format_enemy_stat("Dps", stats, frames, mag);
        let range_str = format_enemy_stat("Range", stats, frames, mag);
        let cash_str = format_enemy_stat("Cash Drop", stats, frames, mag);

        let hp_str = format_enemy_stat("Hitpoints", stats, frames, mag);
        let kb_str = format_enemy_stat("Knockbacks", stats, frames, mag);
        let speed_str = format_enemy_stat("Speed", stats, frames, mag);

        let cycle = (get_enemy_stat("Atk Cycle").get_value)(stats, frames, mag);

        let header_row = row![
            stat_grid::grid_cell(get_enemy_stat("Attack").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Dps").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Range").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Atk Cycle").display_name, true),
        ].spacing(4);

        let value_row = row![
            stat_grid::grid_cell(&atk_str, false),
            stat_grid::grid_cell(&dps_str, false),
            stat_grid::grid_cell(&range_str, false),
            stat_grid::grid_cell_element(stat_grid::render_frames(cycle, 60.0), false),
        ].spacing(4);

        let header_row2 = row![
            stat_grid::grid_cell(get_enemy_stat("Hitpoints").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Knockbacks").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Speed").display_name, true),
            stat_grid::grid_cell(get_enemy_stat("Cash Drop").display_name, true),
        ].spacing(4);

        let value_row2 = row![
            stat_grid::grid_cell(&hp_str, false),
            stat_grid::grid_cell(&kb_str, false),
            stat_grid::grid_cell(&speed_str, false),
            stat_grid::grid_cell(&cash_str, false),
        ].spacing(4);

        column![header_row, value_row, header_row2, value_row2].spacing(4).into()
    }

    fn view_details(&self, enemy: &EnemyEntry) -> Element<'_, Message> {
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
}
