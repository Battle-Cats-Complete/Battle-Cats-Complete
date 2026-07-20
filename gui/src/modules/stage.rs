mod category;
mod list;

use iced::alignment;
use iced::widget::{button, column, container, row, scrollable, space, stack, text, text_input};
use iced::{font, Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::chapter::stage::{BossType, EnemyAmount};
use tracing::{debug, warn};

use core::modules::stage::filter::StageFilterState;
use core::modules::stage::{Stage, StageDataState};
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
    ClearFilter,
    SelectCrown(u8),
    FilterCategoryChanged(String),
    FilterMapChanged(String),
    FilterStageChanged(String),
    FilterContinuesToggled(Option<bool>),
    FilterBossGuardToggled(Option<bool>),
    FilterCpuToggled(Option<bool>),
    List(list::Message),
}

pub struct State {
    pub data: StageDataState,
    pub is_sidebar_open: bool,
    pub selected_crown: u8,
    list: list::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            data: StageDataState::default(),
            is_sidebar_open: true,
            selected_crown: 0,
            list: list::State::default(),
        }
    }
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
            }
            Message::ToggleSidebar => self.is_sidebar_open = !self.is_sidebar_open,
            Message::ClearFilter => self.list.filter_state = StageFilterState::default(),
            Message::SelectCrown(crown) => self.selected_crown = crown,
            Message::FilterCategoryChanged(val) => self.list.filter_state.category_name = val,
            Message::FilterMapChanged(val) => self.list.filter_state.map_name = val,
            Message::FilterStageChanged(val) => self.list.filter_state.stage_name = val,
            Message::FilterContinuesToggled(val) => self.list.filter_state.continues = val,
            Message::FilterBossGuardToggled(val) => self.list.filter_state.boss_guard = val,
            Message::FilterCpuToggled(val) => self.list.filter_state.use_super_cpu = val,
            Message::List(msg) => self.list.update(msg, &mut self.data),
        }

        self.list.refresh();
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = self.view_main_panel();
        let sidebar_overlay = self.view_sidebar_overlay();

        if self.list.filter_state.is_open {
            stack![base, sidebar_overlay, self.view_filter_modal()]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            stack![base, sidebar_overlay]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }

    fn view_sidebar_overlay(&self) -> Element<'_, Message> {
        let arrow_text = if self.is_sidebar_open { "◀" } else { "▶" };
        let toggle_btn = button(text(arrow_text).size(20).align_x(alignment::Horizontal::Center))
            .width(40)
            .height(40)
            .on_press(Message::ToggleSidebar)
            .style(|theme: &Theme, status| button::primary(theme, status));

        let toggle_container = column![toggle_btn]
            .padding(iced::Padding { top: 2.5, right: 0.0, bottom: 0.0, left: 10.0 });

        let mut layer = row![].height(Length::Fill);

        if self.is_sidebar_open {
            let sidebar_panel = container(self.list.view(&self.data).map(Message::List))
                .style(|theme: &Theme| container::Style {
                    background: Some(theme.palette().background.into()),
                    ..Default::default()
                })
                .height(Length::Fill);

            layer = layer.push(sidebar_panel);
        }

        layer = layer.push(toggle_container);

        container(layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_left(Length::Fill)
            .into()
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
                text_input("Any", &self.list.filter_state.category_name)
                    .on_input(Message::FilterCategoryChanged)
            ].align_y(Alignment::Center),
            row![
                text("Map:").width(80),
                text_input("Any", &self.list.filter_state.map_name)
                    .on_input(Message::FilterMapChanged)
            ].align_y(Alignment::Center),
            row![
                text("Stage:").width(80),
                text_input("Any", &self.list.filter_state.stage_name)
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
                .on_press(Message::List(list::Message::ToggleFilter))
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