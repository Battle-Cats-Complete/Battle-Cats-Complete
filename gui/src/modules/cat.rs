mod abilities;
mod conjure;
mod details;
mod export;
mod filter;
mod list;
mod statblock;
mod talents;
mod ultra;

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use iced::alignment::Vertical;
use iced::futures::channel::mpsc;
use iced::widget::image::Handle;
use iced::widget::{
    button, column, container, image as iced_image, row, rule,
    text, text_input, Id, Space,
};
use iced::{Border, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::cat::unit::Battle;
use tracing::info;

use core::common::context::GlobalContext;
use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::common::gfx::autocrop;
use core::modules::cat::game::registry::{format_cat_stat, STAT_ATK_CYCLE, STAT_ATTACK, STAT_COOLDOWN, STAT_COST, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_RARITY, STAT_SPEED};
use core::modules::cat::game::stats::{get_final_stats, seeded_level};
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::paths::DIR_CATS;
use core::modules::cat::scanner::{self, CatEntry};
use core::modules::cat::waiter::unitid;
use core::modules::cat::CatDataState;
use core::modules::settings::Settings;

use crate::animation;
use crate::app::state::{AppState, CatListState};
use crate::app::theme;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::widget::{grid_frames, grid_header, grid_value, name_box, roster_list, statblock_export};

const HEADER_BUTTON_WIDTH: f32 = 65.0;
const HEADER_BUTTON_HEIGHT: f32 = 26.0;
const HEADER_BUTTON_TOP_PADDING: f32 = 5.0;
const EXPORT_BUTTON_RULE_GAP: f32 = 2.0;
const TALENT_HISTORY_CAP: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Abilities,
    Talents,
    Details,
    Animation,
}

#[derive(Clone)]
pub enum Message {
    AnimationTick,
    SheetsCheck,
    ScanProgress(usize, usize),
    Loaded(Vec<CatEntry>),
    Img015Loaded(usize, Option<CoreSpriteSheet>),
    Img022Loaded(usize, Option<CoreSpriteSheet>),
    SearchChanged(String),
    SelectCat(u32),
    SelectForm(usize),
    SelectTab(DetailTab),
    LevelInputChanged(String),
    ChangeTalentLevel(u8, u8),
    ToggleTalents(bool),
    List(list::Message),
    Filter(filter::Message),
    Abilities(abilities::Message),
    Talents(talents::Message),
    Export(statblock_export::Message),
    Animation(animation::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnimationTick => write!(f, "AnimationTick"),
            Self::SheetsCheck => write!(f, "SheetsCheck"),
            Self::ScanProgress(done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(cats) => write!(f, "Loaded({})", cats.len()),
            Self::Img015Loaded(i, _) => write!(f, "Img015Loaded({})", i),
            Self::Img022Loaded(i, _) => write!(f, "Img022Loaded({})", i),
            Self::SearchChanged(s) => write!(f, "SearchChanged({})", s),
            Self::SelectCat(id) => write!(f, "SelectCat({})", id),
            Self::SelectForm(i) => write!(f, "SelectForm({})", i),
            Self::SelectTab(t) => write!(f, "SelectTab({:?})", t),
            Self::LevelInputChanged(s) => write!(f, "LevelInputChanged({})", s),
            Self::ChangeTalentLevel(i, l) => write!(f, "ChangeTalentLevel({}, {})", i, l),
            Self::ToggleTalents(b) => write!(f, "ToggleTalents({})", b),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
            Self::Abilities(msg) => write!(f, "Abilities({:?})", msg),
            Self::Talents(msg) => write!(f, "Talents({:?})", msg),
            Self::Export(msg) => write!(f, "Export({:?})", msg),
            Self::Animation(msg) => write!(f, "Animation({:?})", msg),
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
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>,
    pub talent_level_inputs: HashMap<u8, String>,

    img015_sheets: Vec<SpriteSheet>,
    img022_sheets: Vec<SpriteSheet>,
    custom_assets: CustomAssets,

    header_icon_cache: RefCell<HashMap<PathBuf, Handle>>,
    header_icon_dummy: Handle,

    scan_progress: Option<(usize, usize)>,

    list: list::State,
    filter: filter::State,
    abilities: abilities::State,
    details: details::State,
    talents: talents::State,
    export: statblock_export::State,
    ultra: ultra::State,
    animation: animation::State,
}

impl Default for State {
    fn default() -> Self {
        let header_icon_dummy = Handle::from_rgba(1, 1, vec![80, 80, 80, 255]);

        Self {
            data: CatDataState::default(),
            selected_cat: None,
            selected_form: 0,
            selected_tab: DetailTab::Abilities,
            search_query: String::new(),
            current_level: 1,
            level_input: String::from("1"),
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            talent_level_inputs: HashMap::new(),

            img015_sheets: Vec::new(),
            img022_sheets: Vec::new(),
            custom_assets: CustomAssets::new(),

            header_icon_cache: RefCell::new(HashMap::new()),
            header_icon_dummy,

            scan_progress: None,

            list: list::State::default(),
            filter: filter::State::default(),
            abilities: abilities::State::default(),
            details: details::State::default(),
            talents: talents::State::default(),
            export: statblock_export::State::new("Cat"),
            ultra: ultra::State::default(),
            animation: animation::State::default(),
        }
    }
}

impl State {
    pub(crate) fn list_scrollable_id() -> Id {
        list::State::scrollable_id()
    }

    pub(crate) fn list_scroll_offset(&self) -> f32 {
        self.list.scroll_offset()
    }

    pub(crate) fn restore_state(&mut self, state: &CatListState) {
        self.list.set_scroll_offset(state.list_scroll_offset);
        self.selected_cat = state.selected_cat;
        self.selected_form = state.selected_form;
        self.search_query = state.search_query.clone();
        self.talent_levels = state.talent_levels.clone();
        self.talent_history = state.talent_history.clone();
        self.current_level = state.current_level;
        self.level_input = state.level_input.clone();
    }

    pub(crate) fn sync_state(&self, state: &mut CatListState) {
        state.list_scroll_offset = self.list.scroll_offset();
        state.selected_cat = self.selected_cat;
        state.selected_form = self.selected_form;
        state.current_level = self.current_level;

        if state.search_query != self.search_query {
            state.search_query = self.search_query.clone();
        }
        if state.level_input != self.level_input {
            state.level_input = self.level_input.clone();
        }
        if state.talent_history != self.talent_history {
            state.talent_history = self.talent_history.clone();
        }
        if state.talent_levels != self.talent_levels {
            state.talent_levels = self.talent_levels.clone();
        }
    }

    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.selected_tab == DetailTab::Animation {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        }
    }

    pub fn start_load(&mut self, settings: &Settings) -> Task<Message> {
        info!("Triggering initial cat load");
        let config = settings.scanner_config();
        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            let cats = scanner::load(config, |done, total| {
                let _ = tx.unbounded_send(Message::ScanProgress(done, total));
            });
            let _ = tx.unbounded_send(Message::Loaded(cats));
        });

        Task::batch([Task::stream(rx), self.check_sheets(settings)])
    }

    pub fn rescan(&mut self, settings: &Settings) -> Task<Message> {
        info!("Rescanning cats for active-mod change");
        self.animation.invalidate_paths();
        self.start_load(settings)
    }

    fn clamped_selection(&self, cat: &CatEntry) -> (usize, DetailTab) {
        let max_form = cat.forms.iter().rposition(|&exists| exists).unwrap_or(0);
        let current_exists = cat.forms.get(self.selected_form).copied().unwrap_or(false);
        let form = if self.selected_form > max_form || !current_exists {
            max_form
        } else {
            self.selected_form
        };

        let tab = if self.selected_tab == DetailTab::Talents && (form < 2 || cat.talent_data.is_none()) {
            DetailTab::Abilities
        } else {
            self.selected_tab
        };

        (form, tab)
    }

    fn push_talent_history(&mut self, id: u32) {
        if let Some(pos) = self.talent_history.iter().position(|&existing| existing == id) {
            self.talent_history.remove(pos);
        }
        self.talent_history.push_back(id);

        while self.talent_history.len() > TALENT_HISTORY_CAP {
            if let Some(evicted) = self.talent_history.pop_front() {
                self.talent_levels.remove(&evicted);
            }
        }
    }

    fn check_sheets(&mut self, settings: &Settings) -> Task<Message> {
        let img015_task = crate::common::img015::ensure_loaded(&mut self.img015_sheets, settings)
            .map(|(index, sheet)| Message::Img015Loaded(index, sheet));
        let img022_task = crate::common::img022::ensure_loaded(&mut self.img022_sheets, settings)
            .map(|(index, sheet)| Message::Img022Loaded(index, sheet));

        Task::batch([img015_task, img022_task])
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState, global_ctx: GlobalContext<'_>) -> Task<Message> {
        let task = self.update_inner(message, settings, app_state, global_ctx);

        let cat = self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id));
        let talent_levels = cat.and_then(|entry| self.talent_levels.get(&entry.id));
        self.ultra.sync(
            ultra::Ctx {
                cat,
                form: self.selected_form,
                talent_levels,
                bump_enabled: settings.cat_data.bump_ultra_60,
            },
            &mut self.current_level,
            &mut self.level_input,
        );

        self.list.refresh(&self.data.cats, &self.search_query, &self.filter.filter_state);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState, global_ctx: GlobalContext<'_>) -> Task<Message> {
        match message {
            Message::SheetsCheck => self.check_sheets(settings),
            Message::Img015Loaded(index, sheet) => {
                if let Some(slot) = self.img015_sheets.get_mut(index) {
                    slot.apply(sheet);
                }
                Task::none()
            }
            Message::Img022Loaded(index, sheet) => {
                if let Some(slot) = self.img022_sheets.get_mut(index) {
                    slot.apply(sheet);
                }
                Task::none()
            }
            Message::ScanProgress(done, total) => {
                if self.scan_progress.is_none_or(|(prev, _)| done > prev) {
                    self.scan_progress = Some((done, total));
                }
                Task::none()
            }
            Message::Loaded(cats) => {
                info!("Cat load finished with {} entries", cats.len());
                self.scan_progress = None;
                self.list.invalidate();
                self.details.clear_icons();
                self.data.cats = cats;
                match self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id)) {
                    Some(cat) => {
                        let (form, tab) = self.clamped_selection(cat);
                        self.selected_form = form;
                        self.selected_tab = tab;
                        self.animation.preload(cat, form, settings).map(Message::Animation)
                    }
                    None => Task::none(),
                }
            }
            Message::AnimationTick => {
                if let Some(cat) = self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id)) {
                    self.animation.sync(cat, self.selected_form, settings, &app_state.animation);
                }
                self.animation.tick();
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::SelectCat(id) => {
                if self.selected_cat == Some(id) {
                    return Task::none();
                }

                self.selected_cat = Some(id);
                self.talent_level_inputs.clear();
                self.push_talent_history(id);
                self.animation.reset_playhead();

                info!("Selected cat ID: {}", id);
                match self.data.cats.iter().find(|c| c.id == id) {
                    Some(cat) => {
                        let (level, input) = seeded_level(cat, settings);
                        self.current_level = level;
                        self.level_input = input;
                        let (form, tab) = self.clamped_selection(cat);
                        self.selected_form = form;
                        self.selected_tab = tab;
                        self.animation.preload(cat, form, settings).map(Message::Animation)
                    }
                    None => Task::none(),
                }
            }
            Message::SelectForm(form_idx) => {
                self.selected_form = form_idx;

                if self.selected_tab == DetailTab::Talents && form_idx < 2 {
                    self.selected_tab = DetailTab::Abilities;
                }
                match self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id)) {
                    Some(cat) => self.animation.preload(cat, form_idx, settings).map(Message::Animation),
                    None => Task::none(),
                }
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
            Message::ChangeTalentLevel(index, level) => {
                let Some(cat_id) = self.selected_cat else { return Task::none(); };
                self.talents.set_level(index, level, self.talent_levels.entry(cat_id).or_default(), &mut self.talent_level_inputs);
                Task::none()
            }
            Message::ToggleTalents(is_ultra) => {
                let talent_data = self.selected_cat
                    .and_then(|id| self.data.cats.iter().find(|c| c.id == id))
                    .and_then(|cat| cat.talent_data.as_ref());

                if let Some(cat_id) = self.selected_cat
                    && let Some(talent_data) = talent_data {
                    self.talents.toggle(is_ultra, talent_data, self.talent_levels.entry(cat_id).or_default(), &mut self.talent_level_inputs);
                }
                Task::none()
            }
            Message::List(msg) => {
                if let list::Message::Select(id) = msg {
                    return self.update(Message::SelectCat(id), settings, app_state, global_ctx);
                }

                self.list.update(msg);
                Task::none()
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
            Message::Abilities(msg) => {
                self.abilities.update(msg);
                Task::none()
            }
            Message::Talents(msg) => {
                match msg {
                    talents::Message::LevelChanged(index, level) => {
                        return self.update(Message::ChangeTalentLevel(index, level), settings, app_state, global_ctx);
                    }
                    talents::Message::LevelInputChanged(index, input) => {
                        let talent_data = self.selected_cat
                            .and_then(|id| self.data.cats.iter().find(|c| c.id == id))
                            .and_then(|cat| cat.talent_data.as_ref());
                        let levels = self.selected_cat.map(|id| self.talent_levels.entry(id).or_default());

                        self.talents.set_level_input(index, input, levels, &mut self.talent_level_inputs, talent_data);
                    }
                    talents::Message::ToggleNormal => {
                        return self.update(Message::ToggleTalents(false), settings, app_state, global_ctx);
                    }
                    talents::Message::ToggleUltra => {
                        return self.update(Message::ToggleTalents(true), settings, app_state, global_ctx);
                    }
                    other => self.talents.update(other),
                }
                Task::none()
            }
            Message::Export(msg) => {
                let cat = self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id));
                let ctx = cat.map(|cat| export::Ctx {
                    cat,
                    form: self.selected_form,
                    current_level: self.current_level,
                    level_input: &self.level_input,
                    talent_levels: self.talent_levels.get(&cat.id),
                    is_conjure_expanded: self.abilities.is_conjure_expanded(cat.id, settings),
                    sheets: &self.img015_sheets,
                    global: global_ctx,
                    settings,
                });

                self.export.update(msg, || ctx.and_then(export::request)).map(Message::Export)
            }
            Message::Animation(msg) => self.animation.update(msg, settings, &mut app_state.animation).map(Message::Animation),
        }
    }

    pub fn expanded_animation_view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Option<Element<'a, Message>> {
        self.animation.expanded_view(settings, &app_state.animation).map(|view| view.map(Message::Animation))
    }

    pub fn export_popup_open(&self, app_state: &AppState) -> bool {
        self.animation.export_popup_open(&app_state.animation)
    }

    pub fn export_popup_visible(&self) -> bool {
        self.selected_tab == DetailTab::Animation
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

    pub fn export_popup_view(&self, window: Size, app_state: &AppState) -> Option<Element<'_, Message>> {
        self.animation.export_popup_view(window, &app_state.animation).map(|view| view.map(Message::Animation))
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content(settings, app_state, global_ctx);

        let base_layout = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        base_layout.into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        const SEARCH_FILTER_GAP: f32 = 4.0;
        const FILTER_LIST_GAP: f32 = 8.0;

        let search_input = text_input("Search Cat...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(4)
            .size(13)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let filter_button = button(theme::button_label("Filter").size(13))
            .on_press(Message::Filter(filter::Message::Toggle))
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |t: &Theme, status| theme::toggle_button(t, status, self.filter.filter_state.is_active()));

        let cat_list = self.list.view(&self.data.cats, self.selected_cat).map(Message::List);

        let mut sidebar = column![
            search_input,
            Space::new().height(Length::Fixed(SEARCH_FILTER_GAP)),
            filter_button,
            Space::new().height(Length::Fixed(FILTER_LIST_GAP)),
        ];

        if let Some((done, total)) = self.scan_progress {
            sidebar = sidebar.push(text(format!("Scanning cats... {}/{}", done, total)).size(12));
            sidebar = sidebar.push(Space::new().height(Length::Fixed(FILTER_LIST_GAP)));
        }

        sidebar = sidebar.push(cat_list);

        container(
            sidebar
                .spacing(0)
                .height(Length::Fill)
        )
            .width(Length::Fixed(roster_list::LIST_WIDTH + 16.0))
            .height(Length::Fill)
            .padding(8)
            .style(theme::list_panel_container)
            .into()
    }

    fn view_main_content<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let Some(selected_id) = self.selected_cat else {
            return container(text("Select a Unit").size(24))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let Some(cat) = self.data.cats.iter().find(|c| c.id == selected_id) else {
            return container(text("No Cat Data"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let header = self.view_header(cat);

        let content = match self.selected_tab {
            DetailTab::Abilities => self.view_abilities(cat, settings, global_ctx),
            DetailTab::Talents => self.view_talents(cat, settings),
            DetailTab::Details => self.view_details(cat, settings),
            DetailTab::Animation => self.animation.view(settings, &app_state.animation).map(Message::Animation),
        };

        column![
            header,
            Space::new().height(Length::Fixed(8.0)),
            rule::horizontal(1),
            Space::new().height(Length::Fixed(8.0)),
            content
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 4.0, right: 16.0, bottom: 16.0, left: 16.0 })
            .into()
    }

    fn view_header<'a>(&'a self, cat: &'a CatEntry) -> Element<'a, Message> {
        let mut form_row = row![].spacing(4);
        let form_labels = ["Normal", "Evolved", "True", "Ultra"];

        for (i, label) in form_labels.iter().enumerate() {
            let exists = cat.forms.get(i).copied().unwrap_or(false);
            let is_selected = self.selected_form == i;

            let btn = button(theme::centered_text(*label).size(12))
                .width(Length::Fixed(HEADER_BUTTON_WIDTH))
                .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
                .on_press_maybe(exists.then_some(Message::SelectForm(i)))
                .style(move |theme: &Theme, status| button::Style {
                    border: Border { radius: 0.0.into(), ..theme::header_toggle_button(theme, status, is_selected, exists).border },
                    ..theme::header_toggle_button(theme, status, is_selected, exists)
                });

            form_row = form_row.push(btn);
        }

        let mut tab_row = row![].spacing(4);
        let tabs = [
            (DetailTab::Abilities, "Abilities"),
            (DetailTab::Talents, "Talents"),
            (DetailTab::Details, "Details"),
            (DetailTab::Animation, "Animation"),
        ];

        for (tab_enum, label) in tabs {
            let is_talents = tab_enum == DetailTab::Talents;
            let available = if is_talents {
                self.selected_form >= 2 && cat.talent_data.is_some()
            } else {
                true
            };
            let is_selected = self.selected_tab == tab_enum;

            let btn = button(theme::centered_text(label).size(12))
                .width(Length::Fixed(HEADER_BUTTON_WIDTH))
                .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
                .on_press_maybe(available.then_some(Message::SelectTab(tab_enum)))
                .style(move |theme: &Theme, status| theme::header_toggle_button(theme, status, is_selected, available));

            tab_row = tab_row.push(btn);
        }

        let mut detail_row = row![
            self.view_cat_icon(cat),
            self.view_info_box(cat),
        ].spacing(12).align_y(Vertical::Center);

        if self.selected_tab == DetailTab::Abilities {
            detail_row = detail_row.push(Space::new().width(Length::Fixed(15.0)));
            detail_row = detail_row.push(container(rule::vertical(1)).height(Length::Fixed(96.0)));
            detail_row = detail_row.push(Space::new().width(Length::Fixed(EXPORT_BUTTON_RULE_GAP)));
            detail_row = detail_row.push(self.export.view().map(Message::Export));
        } else if self.selected_tab == DetailTab::Talents
            && let Some(talent_data) = cat.talent_data.as_ref() {
            detail_row = detail_row.push(Space::new().width(Length::Fixed(15.0)));
            detail_row = detail_row.push(container(rule::vertical(1)).height(Length::Fixed(96.0)));
            detail_row = detail_row.push(Space::new().width(Length::Fixed(EXPORT_BUTTON_RULE_GAP)));
            detail_row = detail_row.push(
                self.talents
                    .header_view(talent_data, self.talent_levels.get(&cat.id), &cat.talent_costs, &self.img022_sheets)
                    .map(Message::Talents),
            );
        }

        column![
            Space::new().height(Length::Fixed(HEADER_BUTTON_TOP_PADDING)),
            row![
                form_row,
                container(rule::vertical(1)).height(Length::Fixed(HEADER_BUTTON_HEIGHT - 4.0)),
                tab_row,
            ].spacing(12).align_y(Vertical::Center),
            Space::new().height(Length::Fixed(8.0)),
            rule::horizontal(1),
            Space::new().height(Length::Fixed(8.0)),
            detail_row,
        ].into()
    }

    fn view_cat_icon(&self, cat: &CatEntry) -> Element<'_, Message> {
        let path = cat.deploy_icon_paths[self.selected_form].as_ref();
        let handle = self.cat_icon_handle(path);

        iced_image(handle).height(Length::Fixed(96.0)).into()
    }

    fn cat_icon_handle(&self, path: Option<&PathBuf>) -> Handle {
        if let Some(path) = path
            && let Some(handle) = self.load_icon(path)
        {
            return handle;
        }

        let fallback = PathBuf::from(DIR_CATS).join("uni.png");
        self.load_icon(&fallback).unwrap_or_else(|| self.header_icon_dummy.clone())
    }

    fn load_icon(&self, path: &PathBuf) -> Option<Handle> {
        if let Some(cached) = self.header_icon_cache.borrow().get(path) {
            return Some(cached.clone());
        }

        if !path.exists() {
            return None;
        }

        let img = image::open(path).ok()?;
        let rgba = autocrop(img.to_rgba8());
        let handle = Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw());
        self.header_icon_cache.borrow_mut().insert(path.clone(), handle.clone());
        Some(handle)
    }

    fn view_info_box<'a>(&'a self, cat: &'a CatEntry) -> Element<'a, Message> {
        let disp_name = cat.display_name(self.selected_form);

        let id_text = text(format!("ID: {:03}-{}", cat.id, self.selected_form + 1))
            .size(11)
            .style(|theme: &Theme| text::Style { color: Some(Color { a: 0.4, ..theme.palette().text }) });

        let level_row = row![
            text("Level:").size(11).align_y(Vertical::Center),
            text_input("Level", &self.level_input)
                .on_input(Message::LevelInputChanged)
                .size(11)
                .padding(3)
                .width(Length::Fixed(45.0))
                .style(theme::rounded_input)
        ].spacing(6).align_y(Vertical::Center);

        column![
            name_box(disp_name, 123.0, 56.0, 145.0),
            id_text,
            level_row,
        ].spacing(0).into()
    }

    fn view_abilities<'a>(&'a self, cat: &'a CatEntry, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let dynamic_stats = unitid(cat.id as i32, &settings.general.language_priority);
        let Some(base_stats) = dynamic_stats.as_ref().and_then(|v| v.get(self.selected_form)) else {
            return container(text("Stats data not found")).into();
        };

        let form_allows_talents = self.selected_form >= 2;
        let talent_data = if form_allows_talents { cat.talent_data.as_ref() } else { None };
        let talent_levels = if form_allows_talents { self.talent_levels.get(&cat.id) } else { None };

        let final_stats = get_final_stats(base_stats, cat.curve.as_ref(), self.current_level, talent_data, talent_levels);

        let cat_ctx = CatRenderContext {
            global: global_ctx,
            base_stats,
            final_stats: &final_stats,
            current_level: self.current_level,
            level_curve: cat.curve.as_ref(),
            talent_data,
            talent_levels,
            is_conjure_unit: false,
        };

        column![
            self.view_stats(cat, &final_stats, self.selected_form),
            Space::new().height(Length::Fixed(8.0)),
            self.abilities.view(&cat_ctx, cat, global_ctx, &self.img015_sheets, &self.custom_assets, settings).map(Message::Abilities)
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_stats(&self, cat: &CatEntry, final_stats: &Battle, form: usize) -> Element<'_, Message> {
        let anim_frames = cat.atk_anim_frames[form];
        let unitbuy_opt = Some(&cat.unitbuy);

        let atk_str = format_cat_stat(&STAT_ATTACK, final_stats, anim_frames, unitbuy_opt);
        let dps_str = format_cat_stat(&STAT_DPS, final_stats, anim_frames, unitbuy_opt);
        let range_str = format_cat_stat(&STAT_RANGE, final_stats, anim_frames, unitbuy_opt);
        let rarity_str = format_cat_stat(&STAT_RARITY, final_stats, anim_frames, unitbuy_opt);
        let hp_str = format_cat_stat(&STAT_HITPOINTS, final_stats, anim_frames, unitbuy_opt);
        let kb_str = format_cat_stat(&STAT_KNOCKBACKS, final_stats, anim_frames, unitbuy_opt);
        let speed_str = format_cat_stat(&STAT_SPEED, final_stats, anim_frames, unitbuy_opt);
        let cost_str = format_cat_stat(&STAT_COST, final_stats, anim_frames, unitbuy_opt);

        let cycle = (STAT_ATK_CYCLE.get_value)(final_stats, anim_frames, unitbuy_opt);
        let cd_val = (STAT_COOLDOWN.get_value)(final_stats, anim_frames, unitbuy_opt);

        let header_row = row![
            grid_header(STAT_ATTACK.display_name),
            grid_header(STAT_DPS.display_name),
            grid_header(STAT_RANGE.display_name),
            grid_header(STAT_ATK_CYCLE.display_name),
            grid_header(STAT_RARITY.display_name),
        ].spacing(4);

        let value_row = row![
            grid_value(STAT_ATTACK.display_name, &atk_str),
            grid_value(STAT_DPS.display_name, &dps_str),
            grid_value(STAT_RANGE.display_name, &range_str),
            grid_frames(STAT_ATK_CYCLE.display_name, cycle),
            grid_value(STAT_RARITY.display_name, &rarity_str),
        ].spacing(4);

        let header_row2 = row![
            grid_header(STAT_HITPOINTS.display_name),
            grid_header(STAT_KNOCKBACKS.display_name),
            grid_header(STAT_SPEED.display_name),
            grid_header(STAT_COOLDOWN.display_name),
            grid_header(STAT_COST.display_name),
        ].spacing(4);

        let value_row2 = row![
            grid_value(STAT_HITPOINTS.display_name, &hp_str),
            grid_value(STAT_KNOCKBACKS.display_name, &kb_str),
            grid_value(STAT_SPEED.display_name, &speed_str),
            grid_frames(STAT_COOLDOWN.display_name, cd_val),
            grid_value(STAT_COST.display_name, &cost_str),
        ].spacing(4);

        column![header_row, value_row, header_row2, value_row2].spacing(4).into()
    }

    fn view_talents<'a>(&'a self, cat: &'a CatEntry, settings: &'a Settings) -> Element<'a, Message> {
        let Some(talent_data) = &cat.talent_data else {
            return container(text("No Talents Available")).into();
        };

        let dynamic_stats = unitid(cat.id as i32, &settings.general.language_priority);
        let base_stats = dynamic_stats.as_ref().and_then(|v| v.get(self.selected_form));

        self.talents.view(talents::ViewCtx {
            cat_id: cat.id,
            talent_data,
            talent_levels: self.talent_levels.get(&cat.id),
            level_inputs: &self.talent_level_inputs,
            talent_costs: &cat.talent_costs,
            descriptions: &cat.skill_descriptions,
            current_stats: base_stats,
            curve: cat.curve.as_ref(),
            unit_level: self.current_level,
            sheets: &self.img015_sheets,
            img022_sheets: &self.img022_sheets,
            assets: &self.custom_assets,
            settings,
        }).map(Message::Talents)
    }

    fn view_details<'a>(&'a self, cat: &'a CatEntry, settings: &'a Settings) -> Element<'a, Message> {
        self.details.view(cat, self.selected_form, &settings.general.language_priority)
    }
}
