mod battleground;
mod category;
mod crowns;
mod filter;
mod fixedlineup;
mod icons;
mod info;
mod list;
mod materials;
mod treasure;

use iced::alignment;
use iced::widget::{button, column, container, row, scrollable, space, stack, text};
use iced::{Element, Length, Size, Subscription, Task, Theme};
use tracing::{debug, warn};

use core::common::context::GlobalContext;
use core::modules::settings::Settings;
use core::modules::stage::{fixedlineup as core_fixedlineup, GlobalMapId, StageDataState};

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    ToggleSidebar,
    SelectCrown(u8),
    List(list::Message),
    Filter(filter::Message),
}

pub struct State {
    pub data: StageDataState,
    pub is_sidebar_open: bool,
    pub selected_crown: u8,
    filter: filter::State,
    list: list::State,
    info: info::State,
    materials: materials::State,
    treasure: treasure::State,
    fixedlineup: fixedlineup::State,
    battleground: battleground::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            data: StageDataState::default(),
            is_sidebar_open: true,
            selected_crown: 0,
            filter: filter::State::default(),
            list: list::State::default(),
            info: info::State::default(),
            materials: materials::State::default(),
            treasure: treasure::State::default(),
            fixedlineup: fixedlineup::State::default(),
            battleground: battleground::State::default(),
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
            Message::SelectCrown(crown) => self.selected_crown = crown,
            Message::List(list::Message::ToggleFilter) => self.filter.update(filter::Message::Toggle),
            Message::List(msg) => self.list.update(msg, &mut self.data),
            Message::Filter(msg) => self.filter.update(msg),
        }

        self.list.refresh(&self.filter.filter_state);
        Task::none()
    }

    pub fn view<'a>(&'a self, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let base = self.view_main_panel(global_ctx);
        let sidebar_overlay = self.view_sidebar_overlay();

        stack![base, sidebar_overlay]
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
            .then(|| self.filter.view(window).map(Message::Filter))
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
            let sidebar_panel = container(self.list.view(&self.data, &self.filter.filter_state).map(Message::List))
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

    fn view_main_panel<'a>(&'a self, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
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

        let map_key = GlobalMapId { category: stage.category.clone(), map: stage.map_id };
        let Some(map) = self.data.registry.maps.get(&map_key) else {
            warn!("Failed to locate parent map for stage view");
            return space().into();
        };

        let langs = &self.data.active_language_priority;

        let mut content = column![].spacing(20).padding(40);

        content = content.push(self.info.view(stage, map, langs, &self.data.lock_skip_registry, &self.data.scat_cpu_setting, self.selected_crown));

        if materials::has_drops(stage, map) {
            content = content.push(
                row![
                    self.materials.view(stage, map, self.selected_crown, &self.data.item_buy_registry, &self.data.item_name_registry, langs),
                    space().width(Length::Fixed(15.0)),
                    self.treasure.view(stage, &self.data.item_buy_registry, &self.data.item_name_registry, &self.data.drop_chara_registry, &self.data.unit_buy_registry, langs),
                ]
                    .align_y(iced::Alignment::Start)
            );
        } else {
            content = content.push(self.treasure.view(stage, &self.data.item_buy_registry, &self.data.item_name_registry, &self.data.drop_chara_registry, &self.data.unit_buy_registry, langs));
        }

        if let Some(preset) = stage.fixed_lineups.get(&self.selected_crown) {
            let resolved = core_fixedlineup::resolve_lineup(preset, langs);
            content = content.push(self.fixedlineup.view(&resolved, preset, langs));
        }

        content = content.push(self.battleground.view(stage, map, self.selected_crown, &self.data.enemy_registry, &self.data.enemy_name_registry, global_ctx));

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
