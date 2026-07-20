use std::hash::{Hash, Hasher};

use iced::widget::{button, column, container, row, scrollable, space, stack, text, text_input};
use iced::{font, Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::chapter::Category;
use nyanko::chapter::stage::{BossType, EnemyAmount};
use tracing::{debug, warn};

use core::modules::stage::filter::{CompiledStageFilter, StageFilterState};
use core::modules::stage::{navigate, GlobalMapId, GlobalStageId, Stage, StageDataState};
use core::modules::settings::Settings;

fn bold_text<'a>(content: impl ToString) -> iced::widget::Text<'a> {
    text(content.to_string()).font(font::Font {
        weight: font::Weight::Bold,
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    ToggleSidebar,
    ToggleFilter,
    ClearFilter,
    SelectCategory(Category),
    SelectMap(GlobalMapId),
    SelectStage(GlobalStageId),
    SelectCrown(u8),
    FilterCategoryChanged(String),
    FilterMapChanged(String),
    FilterStageChanged(String),
    FilterContinuesToggled(Option<bool>),
    FilterBossGuardToggled(Option<bool>),
    FilterCpuToggled(Option<bool>),
}

#[derive(Default)]
pub struct State {
    pub data: StageDataState,
    pub is_sidebar_open: bool,
    pub is_filter_open: bool,
    pub selected_crown: u8,
    pub filter_state: StageFilterState,
    pub compiled_filter: Option<CompiledStageFilter>,
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.data.scan_receiver.is_none() && !self.data.initialized {
                    debug!("Triggering initial stage scan");
                    self.data.restart_scan(settings.scanner_config());
                } else if self.data.scan_receiver.is_some() {
                    self.data.update_data();
                }
                Task::none()
            }
            Message::ToggleSidebar => {
                self.is_sidebar_open = !self.is_sidebar_open;
                Task::none()
            }
            Message::ToggleFilter => {
                self.is_filter_open = !self.is_filter_open;
                Task::none()
            }
            Message::ClearFilter => {
                self.filter_state = StageFilterState::default();
                self.compile_filters();
                Task::none()
            }
            Message::SelectCategory(category) => {
                self.data.selected_category = Some(category);
                self.data.selected_map = None;
                self.data.selected_stage = None;
                Task::none()
            }
            Message::SelectMap(map_id) => {
                self.data.selected_map = Some(map_id);
                self.data.selected_stage = None;
                Task::none()
            }
            Message::SelectStage(stage_id) => {
                self.data.selected_stage = Some(stage_id);
                Task::none()
            }
            Message::SelectCrown(crown) => {
                self.selected_crown = crown;
                Task::none()
            }
            Message::FilterCategoryChanged(val) => {
                self.filter_state.category_name = val;
                self.compile_filters();
                Task::none()
            }
            Message::FilterMapChanged(val) => {
                self.filter_state.map_name = val;
                self.compile_filters();
                Task::none()
            }
            Message::FilterStageChanged(val) => {
                self.filter_state.stage_name = val;
                self.compile_filters();
                Task::none()
            }
            Message::FilterContinuesToggled(val) => {
                self.filter_state.continues = val;
                self.compile_filters();
                Task::none()
            }
            Message::FilterBossGuardToggled(val) => {
                self.filter_state.boss_guard = val;
                self.compile_filters();
                Task::none()
            }
            Message::FilterCpuToggled(val) => {
                self.filter_state.use_super_cpu = val;
                self.compile_filters();
                Task::none()
            }
        }
    }

    fn compile_filters(&mut self) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.filter_state.hash(&mut hasher);
        let hash = hasher.finish();

        let mut compiled = self.filter_state.compile();
        compiled.source_hash = hash;
        self.compiled_filter = Some(compiled);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = row![
            self.view_sidebar(),
            self.view_main_panel(),
        ]
            .width(Length::Fill)
            .height(Length::Fill);

        if self.is_filter_open {
            stack![main_content, self.view_filter_modal()]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            main_content.into()
        }
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        if !self.is_sidebar_open {
            return space().width(0).into();
        }

        let categories = navigate::get_categories(&self.data.registry);
        if categories.is_empty() {
            return container(text("No Stages Found").style(|theme: &Theme| text::Style {
                color: Some(theme.palette().danger),
            }))
                .width(180)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let mut sidebar_row = row![].height(Length::Fill);

        // Category Column
        let mut cat_col = column![].spacing(6).width(180);
        let mut sorted_categories = categories.to_vec();
        sorted_categories.sort_by_key(|c| format!("{:?}", c));

        for cat in sorted_categories {
            let is_selected = self.data.selected_category.as_ref() == Some(&cat);
            cat_col = cat_col.push(self.sidebar_button(
                format!("{:?}", cat),
                is_selected,
                Message::SelectCategory(cat)
            ));
        }
        sidebar_row = sidebar_row.push(scrollable(cat_col));

        // Maps Column
        if let Some(cat) = &self.data.selected_category {
            let mut map_col = column![].spacing(6).width(200);
            let maps = navigate::get_maps(&self.data.registry, cat);

            for map in maps {
                let map_key = GlobalMapId { category: cat.clone(), map: map.map_id };
                let is_selected = self.data.selected_map.as_ref() == Some(&map_key);
                map_col = map_col.push(self.sidebar_button(
                    map.name.clone(),
                    is_selected,
                    Message::SelectMap(map_key)
                ));
            }
            sidebar_row = sidebar_row.push(scrollable(map_col));
        }

        // Stages Column
        if let Some(map_id) = &self.data.selected_map {
            let mut stage_col = column![].spacing(6).width(200);
            let stages = navigate::get_stages(&self.data.registry, map_id);

            for stage in stages {
                let stage_key = GlobalStageId {
                    category: map_id.category.clone(),
                    map: map_id.map,
                    stage: stage.stage_id,
                };
                let is_selected = self.data.selected_stage.as_ref() == Some(&stage_key);
                stage_col = stage_col.push(self.sidebar_button(
                    stage.name.clone(),
                    is_selected,
                    Message::SelectStage(stage_key)
                ));
            }
            sidebar_row = sidebar_row.push(scrollable(stage_col));
        }

        container(sidebar_row)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..Default::default()
            })
            .height(Length::Fill)
            .into()
    }

    fn sidebar_button(&self, label: String, is_selected: bool, msg: Message) -> Element<'_, Message> {
        let btn = button(text(label).size(13))
            .width(Length::Fill)
            .on_press(msg);

        if is_selected {
            btn.style(|theme: &Theme, _status| button::Style {
                background: Some(theme.palette().primary.into()),
                text_color: theme.palette().text,
                ..Default::default()
            }).into()
        } else {
            btn.style(|theme: &Theme, _status| button::Style {
                background: Some(theme.palette().background.into()),
                text_color: theme.palette().text,
                ..Default::default()
            }).into()
        }
    }

    fn view_main_panel(&self) -> Element<'_, Message> {
        let Some(stage_id) = &self.data.selected_stage else {
            return container(text("Select a stage to view details"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let Some(stage) = self.data.registry.stages.get(stage_id) else {
            warn!("Selected stage could not be located in registry");
            return space().into();
        };

        let mut content = column![]
            .spacing(20)
            .padding(40)
            .push(self.view_stage_header(stage))
            .push(self.view_crowns(stage));

        content = content.push(self.view_battleground(stage));

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_stage_header(&self, stage: &Stage) -> Element<'_, Message> {
        column![
            bold_text(&stage.name).size(32),
            text(format!("Base HP: {}", stage.base_hp)),
            text(format!("Energy: {}", stage.energy)),
            text(format!("XP: {}", stage.xp)),
        ]
            .spacing(8)
            .into()
    }

    fn view_crowns(&self, stage: &Stage) -> Element<'_, Message> {
        if stage.max_crowns <= 1 {
            return space().into();
        }

        let mut row_btns = row![].spacing(5);
        for c in 0..stage.max_crowns {
            let label = format!("{}♔", c + 1);
            let is_selected = self.selected_crown == c;

            let btn = button(bold_text(label))
                .on_press(Message::SelectCrown(c));

            let styled_btn = if is_selected {
                btn.style(|theme: &Theme, _status| button::Style {
                    background: Some(theme.palette().primary.into()),
                    text_color: theme.palette().text,
                    ..Default::default()
                })
            } else {
                btn.style(|theme: &Theme, _status| button::Style {
                    background: Some(theme.palette().background.into()),
                    text_color: theme.palette().text,
                    ..Default::default()
                })
            };

            row_btns = row_btns.push(styled_btn);
        }

        row_btns.into()
    }

    fn view_battleground(&self, stage: &Stage) -> Element<'_, Message> {
        if stage.enemies.is_empty() {
            return text("No enemies defined for this stage.").into();
        }

        let mut grid = column![
            row![
                bold_text("Enemy").width(100),
                bold_text("Count").width(50),
                bold_text("Mag %").width(80),
                bold_text("Base %").width(60),
                bold_text("Spawn").width(60),
                bold_text("Boss").width(60),
            ].spacing(15)
        ].spacing(4);

        for (idx, spawn) in stage.enemies.iter().enumerate() {
            let enemy_name = format!("{:03}-E", spawn.enemy_id);
            let amount = match spawn.amount {
                EnemyAmount::Infinite => "∞".to_string(),
                EnemyAmount::Limit(l) => l.to_string(),
            };
            let boss = match spawn.boss_type {
                BossType::None => "-",
                BossType::Boss => "Yes",
                BossType::ScreenShake => "Shake",
                BossType::Unknown(_) => "Unknown",
            };

            let row_element = row![
                text(enemy_name).width(100),
                text(amount).width(50),
                text(format!("{}%", spawn.magnification)).width(80),
                text(format!("{}%", spawn.base_hp_perc)).width(60),
                text(format!("{}f", spawn.start_frame)).width(60),
                text(boss).width(60),
            ].spacing(15);

            let mut wrapped_row = container(row_element).padding(4);
            if idx % 2 == 0 {
                wrapped_row = wrapped_row.style(|_theme: &Theme| container::Style {
                    background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.05).into()),
                    ..Default::default()
                });
            }

            grid = grid.push(wrapped_row);
        }

        column![
            bold_text("Battleground").size(24),
            grid
        ]
            .spacing(10)
            .into()
    }

    fn view_filter_modal(&self) -> Element<'_, Message> {
        let content = column![
            bold_text("Advanced Stage Filter").size(24),
            row![
                text("Category:").width(80),
                text_input("Any", &self.filter_state.category_name)
                    .on_input(Message::FilterCategoryChanged)
            ].align_y(Alignment::Center),
            row![
                text("Map:").width(80),
                text_input("Any", &self.filter_state.map_name)
                    .on_input(Message::FilterMapChanged)
            ].align_y(Alignment::Center),
            row![
                text("Stage:").width(80),
                text_input("Any", &self.filter_state.stage_name)
                    .on_input(Message::FilterStageChanged)
            ].align_y(Alignment::Center),

            button("Clear Filters")
                .on_press(Message::ClearFilter)
                .style(|theme: &Theme, _status| button::Style {
                    background: Some(theme.palette().danger.into()),
                    text_color: theme.palette().text,
                    ..Default::default()
                }),
            button("Close")
                .on_press(Message::ToggleFilter)
        ]
            .spacing(15)
            .padding(20);

        container(scrollable(content))
            .width(400)
            .height(500)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..Default::default()
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}