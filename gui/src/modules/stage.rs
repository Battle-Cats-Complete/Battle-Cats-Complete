mod battleground;
mod category;
mod crowns;
mod filter;
mod fixedlineup;
mod info;
mod list;
mod materials;
mod treasure;

use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use iced::futures::channel::mpsc;
use iced::widget::{button, column, container, row, scrollable, space, stack};
use iced::{Alignment, Element, Length, Padding, Size, Task};
use tracing::{debug, info, warn};

use core::common::context::GlobalContext;
use core::modules::enemy::scanner::EnemyEntry;
use core::modules::settings::{Settings, SidebarBehavior};
use core::modules::stage::filter::enemy::EnemyFilter;
use core::modules::stage::filter::StageFilterState;
use core::modules::stage::scanner::{self, StageBundle};
use core::modules::stage::{fixedlineup as core_fixedlineup, GlobalMapId, StageDataState};
use core::Vault;

use crate::app::state::StageListState;
use crate::app::theme;
use crate::widget::{slide, smooth_scroll, status, Slide};

const SIDEBAR_PUSH_GAP: f32 = 10.0;
const SIDEBAR_PADDING: f32 = 15.0;
const TOGGLE_BUTTON_SIZE: f32 = 30.0;
const TOGGLE_BUTTON_GAP: f32 = 5.0;
const MIN_WINDOW_WIDTH: f32 = 800.0;
const CONTENT_PADDING: f32 = 40.0;
const CONTENT_TOP_PADDING: f32 = 3.0;
const CONTENT_SPACING: f32 = 20.0;
const DROP_TABLE_GAP: f32 = 15.0;

pub(super) const CONTENT_WIDTH: f32 = MIN_WINDOW_WIDTH - CONTENT_PADDING * 2.0;

#[derive(Clone)]
pub enum Message {
    ScanProgress(usize, usize),
    Loaded(Box<StageBundle>),
    ToggleSidebar,
    SelectCrown(u8),
    ShowEnemyAppearances(u32),
    List(list::Message),
    Filter(filter::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanProgress(done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(bundle) => write!(f, "Loaded({} maps, {} stages)", bundle.registry.maps.len(), bundle.registry.stages.len()),
            Self::ToggleSidebar => write!(f, "ToggleSidebar"),
            Self::SelectCrown(crown) => write!(f, "SelectCrown({})", crown),
            Self::ShowEnemyAppearances(id) => write!(f, "ShowEnemyAppearances({})", id),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
        }
    }
}

pub struct State {
    pub data: StageDataState,
    pub is_sidebar_open: bool,
    pub selected_crown: u8,
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
    pub(crate) fn invalidate_assets(&self, items: &HashSet<u32>, enemies: &HashSet<u32>, coarse: bool) {
        for id in items {
            self.treasure.forget(*id);
            self.materials.forget(*id);
        }

        for id in enemies {
            self.battleground.forget(*id);
        }

        if coarse {
            self.info.clear_icons();
            self.fixedlineup.clear_icons();
        }
    }

    pub(crate) fn reload_selected(&mut self, vault: &Vault) {
        let Some(map_id) = self.data.selected_map.clone() else { return; };

        let rebuilt = scanner::scan_single(vault, &map_id.category, map_id.map);

        if rebuilt.maps.is_empty() {
            return;
        }

        self.data.registry.stages.retain(|id, _| id.category != map_id.category || id.map != map_id.map);
        self.data.registry.maps.extend(rebuilt.maps);
        self.data.registry.stages.extend(rebuilt.stages);
    }

    pub fn set_indexing(&mut self) {
        self.scan_progress = Some((0, 0));
    }

    pub fn start_load(&mut self, settings: &Settings, vault: &Arc<Vault>, active_mod: Option<String>) -> Task<Message> {
        info!("Triggering initial stage load");
        let config = settings.scanner_config(active_mod);
        let vault = Arc::clone(vault);
        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            let bundle = scanner::load(config, vault, |done, total| {
                let _ = tx.unbounded_send(Message::ScanProgress(done, total));
            });
            let _ = tx.unbounded_send(Message::Loaded(Box::new(bundle)));
        });

        Task::stream(rx)
    }

    pub fn rescan(&mut self, settings: &Settings, vault: &Arc<Vault>, active_mod: Option<String>) -> Task<Message> {
        info!("Rescanning stages for active-mod change");
        self.start_load(settings, vault, active_mod)
    }

    pub(crate) fn restore_state(&mut self, state: &StageListState) {
        self.data.selected_category = state.selected_category.clone();
        self.data.selected_map = state.selected_map.clone();
        self.data.selected_stage = state.selected_stage.clone();
        self.selected_crown = state.selected_crown;
    }

    pub(crate) fn sync_state(&self, state: &mut StageListState) {
        state.selected_crown = self.selected_crown;

        if state.selected_category != self.data.selected_category {
            state.selected_category = self.data.selected_category.clone();
        }
        if state.selected_map != self.data.selected_map {
            state.selected_map = self.data.selected_map.clone();
        }
        if state.selected_stage != self.data.selected_stage {
            state.selected_stage = self.data.selected_stage.clone();
        }
    }

    pub fn sync_enemies(&mut self, enemies: &[EnemyEntry]) {
        self.data.sync_enemies(enemies);
        self.battleground.clear_icons();
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = match message {
            Message::ScanProgress(done, total) => {
                if self.scan_progress.is_none_or(|(prev, _)| done > prev) {
                    self.scan_progress = Some((done, total));
                }
                Task::none()
            }
            Message::Loaded(bundle) => {
                info!("Stage load finished with {} maps and {} stages", bundle.registry.maps.len(), bundle.registry.stages.len());
                self.scan_progress = None;
                self.info.clear_icons();
                self.treasure.clear_icons();
                self.materials.clear_icons();
                self.fixedlineup.clear_icons();
                self.list.invalidate();
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
                self.prune_selection();
                self.clamp_crown();
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
            Message::ShowEnemyAppearances(id) => {
                self.is_sidebar_open = true;
                let filter = EnemyFilter { name_or_id: id.to_string(), ..Default::default() };
                self.filter.filter_state = StageFilterState { enemies: vec![filter], ..Default::default() };
                Task::none()
            }
            Message::List(list::Message::ToggleFilter) => {
                self.filter.update(filter::Message::Toggle);
                Task::none()
            }
            Message::List(msg) => {
                self.list.update(msg, &mut self.data);
                self.clamp_crown();
                Task::none()
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
        };

        self.list.refresh(&self.filter.filter_state, &self.data);
        task
    }

    pub fn view<'a>(&'a self, settings: &Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let mut base = self.view_main_panel(global_ctx);

        if settings.stages.sidebar_behavior == SidebarBehavior::Push {
            let push_spacer = space()
                .width(Length::Fixed(self.sidebar_span() + SIDEBAR_PUSH_GAP))
                .height(Length::Fill);

            base = row![slide(push_spacer, self.is_sidebar_open, Slide::Left), base]
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

    fn prune_selection(&mut self) {
        if self.data.selected_stage.as_ref().is_some_and(|id| !self.data.registry.stages.contains_key(id)) {
            warn!("Dropping restored stage selection missing from the rebuilt registry");
            self.data.selected_stage = None;
        }
        if self.data.selected_map.as_ref().is_some_and(|id| !self.data.registry.maps.contains_key(id)) {
            self.data.selected_map = None;
            self.data.selected_stage = None;
        }
        if self.data.selected_category.as_ref().is_some_and(|category| {
            !self.data.registry.maps.keys().any(|key| key.category == *category)
        }) {
            self.data.selected_category = None;
            self.data.selected_map = None;
            self.data.selected_stage = None;
        }
    }

    fn clamp_crown(&mut self) {
        let Some(stage) = self.data.selected_stage.as_ref().and_then(|id| self.data.registry.stages.get(id)) else {
            return;
        };

        if self.selected_crown >= stage.max_crowns {
            debug!(current = self.selected_crown, max = stage.max_crowns, "Resetting selected crown out of bounds");
            self.selected_crown = 0;
        }
    }

    fn sidebar_span(&self) -> f32 {
        list::sidebar_width(&self.data) + SIDEBAR_PADDING * 2.0
    }

    fn view_sidebar_overlay(&self) -> Element<'_, Message> {
        let arrow_text = if self.is_sidebar_open { "◀" } else { "▶" };
        let toggle_btn = button(theme::centered_text(arrow_text).size(16).width(Length::Fill).height(Length::Fill))
            .width(TOGGLE_BUTTON_SIZE)
            .height(TOGGLE_BUTTON_SIZE)
            .padding(0)
            .on_press(Message::ToggleSidebar)
            .style(theme::neutral_button);

        let toggle_container = container(toggle_btn)
            .padding(Padding { top: TOGGLE_BUTTON_GAP, left: TOGGLE_BUTTON_GAP, ..Padding::ZERO });

        let mut sidebar_content = column![].height(Length::Fill);

        sidebar_content = sidebar_content.push(self.list.view(&self.data, &self.filter.filter_state, self.scan_progress.is_some()).map(Message::List));

        let sidebar_panel = container(sidebar_content)
            .width(Length::Fixed(self.sidebar_span()))
            .height(Length::Fill)
            .padding(SIDEBAR_PADDING)
            .style(theme::left_sidebar_container);

        let layer = row![slide(sidebar_panel, self.is_sidebar_open, Slide::Left), toggle_container]
            .height(Length::Fill)
            .align_y(Alignment::Start);

        container(layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_left(Length::Fill)
            .into()
    }

    fn scan_status(&self) -> Option<Element<'_, Message>> {
        let (done, total) = self.scan_progress?;

        if total == 0 {
            return Some(status("Indexing File System...", None));
        }

        Some(status("Scanning Stages...", Some(format!("{} / {}", done, total))))
    }

    fn view_main_panel<'a>(&'a self, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        if let Some(progress) = self.scan_status() {
            return progress;
        }

        let Some(stage_id) = &self.data.selected_stage else {
            return container(theme::centered_text("Select a stage to view details").size(16))
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

        let vfs = &global_ctx.vault.vfs;

        let mut content = column![]
            .spacing(CONTENT_SPACING)
            .padding(Padding {
                top: CONTENT_TOP_PADDING,
                right: CONTENT_PADDING,
                bottom: CONTENT_PADDING,
                left: CONTENT_PADDING,
            });

        content = content.push(self.info.view(stage, map, vfs, &self.data.lock_skip_registry, &self.data.scat_cpu_setting, self.selected_crown));

        if materials::has_drops(stage, map) {
            content = content.push(
                row![
                    self.materials.view(stage, map, self.selected_crown, &self.data.item_buy_registry, &self.data.item_name_registry, vfs),
                    self.treasure.view(stage, &self.data.item_buy_registry, &self.data.item_name_registry, &self.data.drop_chara_registry, &self.data.unit_buy_registry, vfs),
                ]
                    .spacing(DROP_TABLE_GAP)
                    .align_y(Alignment::Start)
            );
        } else {
            content = content.push(self.treasure.view(stage, &self.data.item_buy_registry, &self.data.item_name_registry, &self.data.drop_chara_registry, &self.data.unit_buy_registry, vfs));
        }

        if let Some(preset) = stage.fixed_lineups.get(&self.selected_crown) {
            let resolved = core_fixedlineup::resolve_lineup(vfs, preset);
            content = content.push(self.fixedlineup.view(&resolved, preset, vfs));
        }

        content = content.push(self.battleground.view(stage, map, self.selected_crown, &self.data.enemy_registry, &self.data.enemy_name_registry, global_ctx));

        smooth_scroll(
            scrollable(content)
                .width(Length::Fill)
                .height(Length::Fill)
        )
            .into()
    }
}
