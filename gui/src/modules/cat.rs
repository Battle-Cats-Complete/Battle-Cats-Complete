mod abilities;
mod filter;
mod list;
mod statblock;
mod talents;

use std::collections::{HashMap, VecDeque};
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use iced::alignment::{Horizontal, Vertical};
use iced::futures::channel::mpsc;
use iced::widget::{
    button, column, container, row, rule, scrollable,
    text, text_input, Id, Space,
};
use iced::{Background, Border, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::cat::unit::Battle;
use tracing::{error, info};

use core::common::context::GlobalContext;
use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::modules::cat::game::registry::{format_cat_stat, STAT_ATK_CYCLE, STAT_ATTACK, STAT_COOLDOWN, STAT_COST, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_RARITY, STAT_SPEED};
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::{self, CatEntry};
use core::modules::cat::waiter::unitid;
use core::modules::cat::CatDataState;
use core::modules::settings::Settings;

use crate::app::state::{AppState, CatListState};
use crate::app::theme;
use crate::common::feedback::Slot;
use crate::common::stat_grid;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::modules::animation;
use crate::modules::statblock::{feedback_color, feedback_label};

use super::statblock::{builder, JobResult};
use statblock::build_cat_statblock;

const HEADER_BUTTON_WIDTH: f32 = 65.0;
const HEADER_BUTTON_HEIGHT: f32 = 26.0;
const HEADER_BUTTON_TOP_PADDING: f32 = 5.0;
const TALENT_HISTORY_CAP: usize = 3;

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
    AnimationTick,
    SheetsCheck,
    ScanProgress(usize, usize),
    Loaded(Vec<CatEntry>),
    Img015Loaded(usize, Option<CoreSpriteSheet>),
    Img022Loaded(usize, Option<CoreSpriteSheet>),
    StatblockFinished(JobResult),
    CopyFeedbackExpired,
    SaveFeedbackExpired,
    SearchChanged(String),
    SelectCat(u32),
    SelectForm(usize),
    SelectTab(DetailTab),
    LevelInputChanged(String),
    ChangeTalentLevel(u8, u8),
    MaximizeTalents(bool),
    ExportStatblock(ExportAction),
    List(list::Message),
    Filter(filter::Message),
    Abilities(abilities::Message),
    Talents(talents::Message),
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
            Self::StatblockFinished(_) => write!(f, "StatblockFinished"),
            Self::CopyFeedbackExpired => write!(f, "CopyFeedbackExpired"),
            Self::SaveFeedbackExpired => write!(f, "SaveFeedbackExpired"),
            Self::SearchChanged(s) => write!(f, "SearchChanged({})", s),
            Self::SelectCat(id) => write!(f, "SelectCat({})", id),
            Self::SelectForm(i) => write!(f, "SelectForm({})", i),
            Self::SelectTab(t) => write!(f, "SelectTab({:?})", t),
            Self::LevelInputChanged(s) => write!(f, "LevelInputChanged({})", s),
            Self::ChangeTalentLevel(i, l) => write!(f, "ChangeTalentLevel({}, {})", i, l),
            Self::MaximizeTalents(b) => write!(f, "MaximizeTalents({})", b),
            Self::ExportStatblock(e) => write!(f, "ExportStatblock({:?})", e),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
            Self::Abilities(msg) => write!(f, "Abilities({:?})", msg),
            Self::Talents(msg) => write!(f, "Talents({:?})", msg),
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
    is_in_ultra_state: bool,
    saved_pre_ultra_level: Option<(i32, String)>,

    img015_sheets: Vec<SpriteSheet>,
    img022_sheets: Vec<SpriteSheet>,
    custom_assets: CustomAssets,

    scan_progress: Option<(usize, usize)>,

    statblock_pending: Option<ExportAction>,
    statblock_clipboard: Option<Clipboard>,
    statblock_copy_feedback: Slot<bool>,
    statblock_save_feedback: Slot<bool>,

    list: list::State,
    filter: filter::State,
    abilities: abilities::State,
    talents: talents::State,
    animation: animation::State,
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
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            talent_level_inputs: HashMap::new(),
            is_in_ultra_state: false,
            saved_pre_ultra_level: None,

            img015_sheets: Vec::new(),
            img022_sheets: Vec::new(),
            custom_assets: CustomAssets::new(),

            scan_progress: None,

            statblock_pending: None,
            statblock_clipboard: None,
            statblock_copy_feedback: Slot::default(),
            statblock_save_feedback: Slot::default(),

            list: list::State::default(),
            filter: filter::State::default(),
            abilities: abilities::State::default(),
            talents: talents::State::default(),
            animation: animation::State::default(),
        }
    }
}

impl State {
    pub(crate) fn list_scrollable_id() -> Id {
        list::scrollable_id()
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

    fn seeded_level(cat: &CatEntry, settings: &Settings) -> (i32, String) {
        if !settings.cat_data.auto_level_calculations {
            let default_level = settings.cat_data.default_level.max(1);
            return (default_level, default_level.to_string());
        }

        let base_max = cat.unitbuy.level_cap_catseye;
        let plus_max = cat.unitbuy.level_cap_plus;
        let is_legend_rare = cat.unitbuy.rarity == 5;
        let is_normal_rare = cat.unitbuy.rarity == 0;

        if is_legend_rare {
            (50, "50".to_string())
        } else if base_max == 1 || (5..=65).contains(&plus_max) || is_normal_rare {
            let input = if plus_max > 0 {
                format!("{}+{}", base_max, plus_max)
            } else {
                base_max.to_string()
            };
            (base_max + plus_max, input)
        } else if base_max > 50 {
            (50, "50".to_string())
        } else {
            (base_max, base_max.to_string())
        }
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

    fn current_ultra_state(&self) -> Option<bool> {
        let cat = self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id))?;

        let mut ultra = self.selected_form == 3;
        if self.selected_form >= 2 && !ultra
            && let Some(levels) = self.talent_levels.get(&cat.id) {
            if let Some(talent_data) = &cat.talent_data {
                ultra = talent_data.groups.iter().enumerate().any(|(idx, group)| {
                    group.limit == 1
                        && levels.get(&(idx as u8)).is_some_and(|&lvl| lvl > 0)
                });
            } else {
                ultra = levels.iter().any(|(&idx, &lvl)| idx >= 5 && lvl > 0);
            }
        }

        Some(ultra)
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

    fn sync_ultra_bump(&mut self, settings: &Settings) {
        let Some(current_ultra) = self.current_ultra_state() else {
            return;
        };

        if !settings.cat_data.bump_ultra_60 {
            self.is_in_ultra_state = current_ultra;
            self.saved_pre_ultra_level = None;
            return;
        }

        if !self.is_in_ultra_state && current_ultra {
            self.saved_pre_ultra_level = Some((self.current_level, self.level_input.clone()));
            if self.current_level < 60 {
                self.current_level = 60;
                self.level_input = "60".to_string();
            }
        } else if self.is_in_ultra_state && !current_ultra
            && let Some((saved_level, saved_input)) = self.saved_pre_ultra_level.take() {
            let expected_level = if saved_level < 60 { 60 } else { saved_level };
            if self.current_level == expected_level {
                self.current_level = saved_level;
                self.level_input = saved_input;
            }
        }
        self.is_in_ultra_state = current_ultra;
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

        self.sync_ultra_bump(settings);
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
            Message::StatblockFinished(job) => {
                self.statblock_pending = None;
                self.finish_statblock_job(job)
            }
            Message::CopyFeedbackExpired => {
                self.statblock_copy_feedback.expire();
                Task::none()
            }
            Message::SaveFeedbackExpired => {
                self.statblock_save_feedback.expire();
                Task::none()
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

                info!("Selected cat ID: {}", id);
                match self.data.cats.iter().find(|c| c.id == id) {
                    Some(cat) => {
                        let (level, input) = Self::seeded_level(cat, settings);
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
                self.talent_levels.entry(cat_id).or_default().insert(index, level);
                self.talent_level_inputs.remove(&index);
                Task::none()
            }
            Message::MaximizeTalents(is_ultra) => {
                if let Some(cat_id) = self.selected_cat
                    && let Some(cat) = self.data.cats.iter().find(|c| c.id == cat_id)
                    && let Some(talent_data) = &cat.talent_data {
                    let levels = self.talent_levels.entry(cat_id).or_default();
                    for (index, group) in talent_data.groups.iter().enumerate() {
                        let target_group = if is_ultra { group.limit == 1 } else { group.limit != 1 };
                        if target_group {
                            levels.insert(index as u8, group.max_level.max(1));
                            self.talent_level_inputs.remove(&(index as u8));
                        }
                    }
                }
                Task::none()
            }
            Message::ExportStatblock(action) => self.start_statblock_export(action, settings, global_ctx),
            Message::List(msg) => {
                if let list::Message::SelectCat(id) = msg {
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
                        let max_level = self.selected_cat
                            .and_then(|cat_id| self.data.cats.iter().find(|c| c.id == cat_id))
                            .and_then(|cat| cat.talent_data.as_ref())
                            .and_then(|talent_data| talent_data.groups.get(index as usize))
                            .map(|group| group.max_level.max(1))
                            .unwrap_or(u8::MAX);

                        if let Some(cat_id) = self.selected_cat {
                            let levels = self.talent_levels.entry(cat_id).or_default();
                            if let Ok(parsed) = input.trim().parse::<u8>() {
                                levels.insert(index, parsed.min(max_level));
                            } else if input.trim().is_empty() {
                                levels.insert(index, 0);
                            }
                        }

                        self.talent_level_inputs.insert(index, input);
                    }
                    other => self.talents.update(other),
                }
                Task::none()
            }
            Message::Animation(msg) => self.animation.update(msg, settings, &mut app_state.animation).map(Message::Animation),
        }
    }

    fn start_statblock_export(&mut self, action: ExportAction, settings: &Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        if self.statblock_pending.is_some() {
            return Task::none();
        }

        let Some(selected_id) = self.selected_cat else { return Task::none(); };
        let Some(cat) = self.data.cats.iter().find(|c| c.id == selected_id) else { return Task::none(); };

        let dynamic_stats = unitid(cat.id as i32, &settings.general.language_priority);
        let Some(base_stats) = dynamic_stats.as_ref().and_then(|v| v.get(self.selected_form)) else { return Task::none(); };

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

        let is_conjure_expanded = self.abilities.is_conjure_expanded(cat.id, settings);
        let data = build_cat_statblock(&cat_ctx, cat, self.selected_form, self.level_input.clone(), is_conjure_expanded, settings);

        let is_cat = data.is_cat;
        let id_str = data.id_str.clone();
        let top_value = data.top_value.clone();

        let mut cuts_map = HashMap::new();
        for sheet in self.img015_sheets.iter().rev() {
            cuts_map.extend(sheet.core.cuts_map.clone());
        }
        let priority = settings.general.language_priority.clone();

        self.statblock_pending = Some(action);

        Task::perform(async move {
            let build_result = builder::build_statblock_image(&priority, data, cuts_map);

            match action {
                ExportAction::Copy => JobResult::Copy(build_result),
                ExportAction::Save => {
                    let result = build_result.and_then(|image| builder::save_to_disk(&image, is_cat, &id_str, &top_value).map(|_| ()));
                    if let Err(err) = &result {
                        error!("Cat statblock save failed: {err}");
                    }
                    JobResult::Save(result)
                }
            }
        }, Message::StatblockFinished)
    }

    fn finish_statblock_job(&mut self, job: JobResult) -> Task<Message> {
        match job {
            JobResult::Copy(Ok(image)) => {
                let result = self
                    .ensure_clipboard()
                    .map_or_else(|| Err("Clipboard unavailable".to_string()), |clipboard| builder::copy_to_clipboard(clipboard, &image));
                if let Err(err) = &result {
                    error!("Cat statblock copy failed: {err}");
                }
                self.statblock_copy_feedback.set(result.is_ok(), Message::CopyFeedbackExpired)
            }
            JobResult::Copy(Err(err)) => {
                error!("Cat statblock export failed: {err}");
                self.statblock_copy_feedback.set(false, Message::CopyFeedbackExpired)
            }
            JobResult::Save(result) => {
                self.statblock_save_feedback.set(result.is_ok(), Message::SaveFeedbackExpired)
            }
        }
    }

    fn ensure_clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.statblock_clipboard.is_none() {
            match Clipboard::new() {
                Ok(clipboard) => self.statblock_clipboard = Some(clipboard),
                Err(err) => error!("Failed to open system clipboard: {err}"),
            }
        }
        self.statblock_clipboard.as_mut()
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
        const FILTER_LIST_GAP: f32 = 4.0;

        let search_input = text_input("Search Cat...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(4)
            .size(13)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let filter_button = button(
            text("Filter")
                .size(13)
                .align_x(Horizontal::Center)
                .width(Length::Fill)
        )
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
            .width(Length::Fixed(list::LIST_WIDTH + 16.0))
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
            return container(text("Loading Unit Data..."))
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
            DetailTab::Details => self.view_details(cat),
            DetailTab::Animation => self.animation.view(settings, &app_state.animation).map(Message::Animation),
        };

        column![
            header,
            Space::new().height(Length::Fixed(16.0)),
            content
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 4.0, right: 16.0, bottom: 16.0, left: 16.0 })
            .into()
    }

    fn view_header(&self, cat: &CatEntry) -> Element<'_, Message> {
        let mut form_row = row![].spacing(4);
        let form_labels = ["Normal", "Evolved", "True", "Ultra"];

        for (i, label) in form_labels.iter().enumerate() {
            let exists = cat.forms.get(i).copied().unwrap_or(false);
            let is_selected = self.selected_form == i;

            let btn = button(text(*label).size(12).align_x(Horizontal::Center).align_y(Vertical::Center))
                .width(Length::Fixed(HEADER_BUTTON_WIDTH))
                .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
                .on_press_maybe(exists.then_some(Message::SelectForm(i)))
                .style(move |theme: &Theme, status| button::Style {
                    border: Border { radius: 0.0.into(), ..theme::header_toggle_button(theme, status, is_selected, exists).border },
                    ..theme::header_toggle_button(theme, status, is_selected, exists)
                });

            form_row = form_row.push(btn);
        }

        let copy_busy = self.statblock_pending == Some(ExportAction::Copy);
        let copy_feedback = self.statblock_copy_feedback.get().copied();
        let copy_label = feedback_label(copy_busy, copy_feedback, "Copy Image", "Copying...", "Copied!", "Failed!");
        let copy_btn = button(text(copy_label).size(12))
            .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportStatblock(ExportAction::Copy)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, copy_busy, copy_feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            });

        let save_busy = self.statblock_pending == Some(ExportAction::Save);
        let save_feedback = self.statblock_save_feedback.get().copied();
        let save_label = feedback_label(save_busy, save_feedback, "Export Image", "Exporting...", "Exported!", "Failed!");
        let save_btn = button(text(save_label).size(12))
            .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportStatblock(ExportAction::Save)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, save_busy, save_feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            });

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

            let btn = button(text(label).size(12).align_x(Horizontal::Center).align_y(Vertical::Center))
                .width(Length::Fixed(HEADER_BUTTON_WIDTH))
                .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
                .on_press_maybe(available.then_some(Message::SelectTab(tab_enum)))
                .style(move |theme: &Theme, status| theme::header_toggle_button(theme, status, is_selected, available));

            tab_row = tab_row.push(btn);
        }

        let level_row = row![
            text("Level:").align_y(Vertical::Center),
            text_input("Level", &self.level_input)
                .on_input(Message::LevelInputChanged)
                .width(Length::Fixed(60.0))
        ].spacing(8).align_y(Vertical::Center);

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
            row![copy_btn, save_btn].spacing(8),
            Space::new().height(Length::Fixed(16.0)),
            row![
                text(format!("ID: {:03}-{}", cat.id, self.selected_form + 1)).size(12),
                Space::new().width(Length::Fill),
                level_row
            ],
        ].into()
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
            stat_grid::grid_cell(STAT_ATTACK.display_name, true),
            stat_grid::grid_cell(STAT_DPS.display_name, true),
            stat_grid::grid_cell(STAT_RANGE.display_name, true),
            stat_grid::grid_cell(STAT_ATK_CYCLE.display_name, true),
            stat_grid::grid_cell(STAT_RARITY.display_name, true),
        ].spacing(4);

        let value_row = row![
            stat_grid::grid_cell(&atk_str, false),
            stat_grid::grid_cell(&dps_str, false),
            stat_grid::grid_cell(&range_str, false),
            stat_grid::grid_cell_element(stat_grid::render_frames(cycle, 60.0), false),
            stat_grid::grid_cell(&rarity_str, false),
        ].spacing(4);

        let header_row2 = row![
            stat_grid::grid_cell(STAT_HITPOINTS.display_name, true),
            stat_grid::grid_cell(STAT_KNOCKBACKS.display_name, true),
            stat_grid::grid_cell(STAT_SPEED.display_name, true),
            stat_grid::grid_cell(STAT_COOLDOWN.display_name, true),
            stat_grid::grid_cell(STAT_COST.display_name, true),
        ].spacing(4);

        let value_row2 = row![
            stat_grid::grid_cell(&hp_str, false),
            stat_grid::grid_cell(&kb_str, false),
            stat_grid::grid_cell(&speed_str, false),
            stat_grid::grid_cell_element(stat_grid::render_frames(cd_val, 60.0), false),
            stat_grid::grid_cell(&cost_str, false),
        ].spacing(4);

        column![header_row, value_row, header_row2, value_row2].spacing(4).into()
    }

    fn view_talents<'a>(&'a self, cat: &'a CatEntry, settings: &'a Settings) -> Element<'a, Message> {
        let Some(talent_data) = &cat.talent_data else {
            return container(text("No Talents Available")).into();
        };

        let dynamic_stats = unitid(cat.id as i32, &settings.general.language_priority);
        let base_stats = dynamic_stats.as_ref().and_then(|v| v.get(self.selected_form));

        let normal_talents_btn = button("Max Normal Talents")
            .on_press(Message::MaximizeTalents(false));
        let ultra_talents_btn = button("Max Ultra Talents")
            .on_press(Message::MaximizeTalents(true));

        let talents_view = self.talents.view(talents::ViewCtx {
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
        }).map(Message::Talents);

        column![
            row![normal_talents_btn, ultra_talents_btn].spacing(8),
            Space::new().height(Length::Fixed(12.0)),
            talents_view
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_details(&self, cat: &CatEntry) -> Element<'_, Message> {
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
