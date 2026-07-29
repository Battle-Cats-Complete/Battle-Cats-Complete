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

use std::thread;

use iced::alignment;
use iced::futures::channel::mpsc;
use iced::widget::{button, column, container, row, scrollable, space, stack, text};
use iced::{task, Element, Length, Size, Subscription, Task, Theme};
use tracing::{info, warn};

use core::common::context::GlobalContext;
use core::modules::settings::{Settings, SidebarBehavior};
const SIDEBAR_PUSH_GAP: f32 = 10.0;

use core::modules::stage::scanner::{self, StageBundle};
use core::modules::stage::{fixedlineup as core_fixedlineup, GlobalMapId, StageDataState};

#[derive(Clone)]
pub enum Message {
    Tick,
    ScanProgress(usize, usize),
    Loaded(StageBundle),
    ToggleSidebar,
    SelectCrown(u8),
    List(list::Message),
    Filter(filter::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tick => write!(f, "Tick"),
            Self::ScanProgress(done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(bundle) => write!(f, "Loaded({} maps, {} stages)", bundle.registry.maps.len(), bundle.registry.stages.len()),
            Self::ToggleSidebar => write!(f, "ToggleSidebar"),
            Self::SelectCrown(crown) => write!(f, "SelectCrown({})", crown),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
        }
    }
}

pub struct State {
    pub data: StageDataState,
    pub is_sidebar_open: bool,
    pub selected_crown: u8,
    load_handle: Option<task::Handle>,
    scan_progress: Option<(usize, usize)>,
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
            load_handle: None,
            scan_progress: None,
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
        let task = match message {
            Message::Tick => {
                if self.load_handle.is_none() {
                    info!("Triggering initial stage load");
                    let config = settings.scanner_config();
                    let (tx, rx) = mpsc::unbounded();

                    thread::spawn(move || {
                        let bundle = scanner::load(config, |done, total| {
                            let _ = tx.unbounded_send(Message::ScanProgress(done, total));
                        });
                        let _ = tx.unbounded_send(Message::Loaded(bundle));
                    });

                    let (load_task, handle) = Task::stream(rx).abortable();
                    self.load_handle = Some(handle);
                    load_task
                } else {
                    Task::none()
                }
            }
            Message::ScanProgress(done, total) => {
                if self.scan_progress.is_none_or(|(prev, _)| done > prev) {
                    self.scan_progress = Some((done, total));
                }
                Task::none()
            }
            Message::Loaded(bundle) => {
                info!("Stage load finished with {} maps and {} stages", bundle.registry.maps.len(), bundle.registry.stages.len());
                self.scan_progress = None;
                self.data.registry = bundle.registry;

                let dictionaries = bundle.dictionaries;
                self.data.enemy_name_registry = dictionaries.enemy_name_registry;
                self.data.item_buy_registry = dictionaries.item_buy_registry;
                self.data.item_name_registry = dictionaries.item_name_registry;
                self.data.drop_chara_registry = dictionaries.drop_chara_registry;
                self.data.unit_buy_registry = dictionaries.unit_buy_registry;
                self.data.cat_name_registry = dictionaries.cat_name_registry;
                self.data.lock_skip_registry = dictionaries.lock_skip_registry;
                self.data.scat_cpu_setting = dictionaries.scat_cpu_setting;
                self.data.active_language_priority = dictionaries.active_language_priority;
                Task::none()
            }
            Message::ToggleSidebar => {
                self.is_sidebar_open = !self.is_sidebar_open;
                Task::none()
            }
            Message::SelectCrown(crown) => {
                self.selected_crown = crown;
                Task::none()
            }
            Message::List(list::Message::ToggleFilter) => {
                self.filter.update(filter::Message::Toggle);
                Task::none()
            }
            Message::List(msg) => {
                self.list.update(msg, &mut self.data);
                Task::none()
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
        };

        self.list.refresh(&self.filter.filter_state);
        task
    }

    pub fn view<'a>(&'a self, settings: &Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let mut base = self.view_main_panel(global_ctx);

        if settings.stages.sidebar_behavior == SidebarBehavior::Push && self.is_sidebar_open {
            let push_offset = list::sidebar_width(&self.data) + SIDEBAR_PUSH_GAP;
            base = container(base)
                .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: push_offset })
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

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
            let mut sidebar_content = column![];

            if let Some((done, total)) = self.scan_progress {
                sidebar_content = sidebar_content.push(text(format!("Scanning stages... {}/{}", done, total)).size(12));
            }

            sidebar_content = sidebar_content.push(self.list.view(&self.data, &self.filter.filter_state).map(Message::List));

            let sidebar_panel = container(sidebar_content)
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
