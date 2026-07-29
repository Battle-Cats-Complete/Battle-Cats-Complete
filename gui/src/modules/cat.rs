mod abilities;
mod filter;
mod list;
mod statblock;
mod talents;

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use iced::alignment::{Horizontal, Vertical};
use iced::futures::channel::mpsc;
use iced::widget::{
    button, column, container, row, scrollable,
    text, text_input, Space,
};
use iced::{task, Background, Border, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::cat::unit::Battle;
use tracing::{error, info};

use core::common::context::GlobalContext;
use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::modules::cat::game::registry::{format_cat_stat, get_cat_stat};
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::{self, CatEntry};
use core::modules::cat::waiter::unitid;
use core::modules::cat::CatDataState;
use core::modules::settings::Settings;

use crate::common::stat_grid;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::modules::animation;
use crate::modules::statblock::{feedback_color, feedback_label};

use super::statblock::{builder, JobResult};
use statblock::build_cat_statblock;

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
    AnimationTick,
    ScanProgress(usize, usize),
    Loaded(Vec<CatEntry>),
    Img015Loaded(usize, Option<CoreSpriteSheet>),
    Img022Loaded(usize, Option<CoreSpriteSheet>),
    StatblockFinished(JobResult),
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
            Self::Tick => write!(f, "Tick"),
            Self::AnimationTick => write!(f, "AnimationTick"),
            Self::ScanProgress(done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(cats) => write!(f, "Loaded({})", cats.len()),
            Self::Img015Loaded(i, _) => write!(f, "Img015Loaded({})", i),
            Self::Img022Loaded(i, _) => write!(f, "Img022Loaded({})", i),
            Self::StatblockFinished(_) => write!(f, "StatblockFinished"),
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
    pub talent_levels: HashMap<u8, u8>,
    pub talent_level_inputs: HashMap<u8, String>,

    img015_sheets: Vec<SpriteSheet>,
    img022_sheets: Vec<SpriteSheet>,
    custom_assets: CustomAssets,

    load_handle: Option<task::Handle>,
    scan_progress: Option<(usize, usize)>,

    statblock_pending: Option<ExportAction>,
    statblock_clipboard: Option<Clipboard>,
    statblock_copy_feedback: Option<(bool, Instant)>,
    statblock_save_feedback: Option<(bool, Instant)>,

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
            talent_level_inputs: HashMap::new(),

            img015_sheets: Vec::new(),
            img022_sheets: Vec::new(),
            custom_assets: CustomAssets::new(),

            load_handle: None,
            scan_progress: None,

            statblock_pending: None,
            statblock_clipboard: None,
            statblock_copy_feedback: None,
            statblock_save_feedback: None,

            list: list::State::default(),
            filter: filter::State::default(),
            abilities: abilities::State::default(),
            talents: talents::State::default(),
            animation: animation::State::default(),
        }
    }
}

impl State {
    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)];

        if self.selected_tab == DetailTab::Animation {
            subscriptions.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick));
        }

        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        let task = self.update_inner(message, settings, global_ctx);

        self.list.refresh(&self.data.cats, &self.search_query, &self.filter.filter_state, settings.cat_data.high_banner_quality);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &mut Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        match message {
            Message::Tick => {
                let load_task = if self.load_handle.is_none() {
                    info!("Triggering initial cat load");
                    let config = settings.scanner_config();
                    let (tx, rx) = mpsc::unbounded();

                    thread::spawn(move || {
                        let cats = scanner::load(config, |done, total| {
                            let _ = tx.unbounded_send(Message::ScanProgress(done, total));
                        });
                        let _ = tx.unbounded_send(Message::Loaded(cats));
                    });

                    let (load_task, handle) = Task::stream(rx).abortable();
                    self.load_handle = Some(handle);
                    load_task
                } else {
                    Task::none()
                };

                let img015_task = crate::common::img015::ensure_loaded(&mut self.img015_sheets, settings)
                    .map(|(index, sheet)| Message::Img015Loaded(index, sheet));
                let img022_task = crate::common::img022::ensure_loaded(&mut self.img022_sheets, settings)
                    .map(|(index, sheet)| Message::Img022Loaded(index, sheet));

                Task::batch([load_task, img015_task, img022_task])
            }
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
                self.data.cats = cats;
                Task::none()
            }
            Message::StatblockFinished(job) => {
                self.statblock_pending = None;
                self.finish_statblock_job(job);
                Task::none()
            }
            Message::AnimationTick => {
                if let Some(cat) = self.selected_cat.and_then(|id| self.data.cats.iter().find(|c| c.id == id)) {
                    self.animation.sync(cat, self.selected_form, settings);
                }
                self.animation.tick();
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::SelectCat(id) => {
                self.selected_cat = Some(id);
                self.selected_form = 0;
                self.selected_tab = DetailTab::Abilities;
                self.talent_levels.clear();
                self.talent_level_inputs.clear();

                if let Some(_cat) = self.data.cats.iter().find(|c| c.id == id) {
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
            Message::ChangeTalentLevel(index, level) => {
                self.talent_levels.insert(index, level);
                self.talent_level_inputs.remove(&index);
                Task::none()
            }
            Message::MaximizeTalents(is_ultra) => {
                if let Some(cat_id) = self.selected_cat {
                    if let Some(cat) = self.data.cats.iter().find(|c| c.id == cat_id) {
                        if let Some(talent_data) = &cat.talent_data {
                            for (index, group) in talent_data.groups.iter().enumerate() {
                                let target_group = if is_ultra { group.limit == 1 } else { group.limit != 1 };
                                if target_group {
                                    self.talent_levels.insert(index as u8, group.max_level.max(1));
                                    self.talent_level_inputs.remove(&(index as u8));
                                }
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ExportStatblock(action) => self.start_statblock_export(action, settings, global_ctx),
            Message::List(msg) => {
                if let list::Message::SelectCat(id) = msg {
                    return self.update(Message::SelectCat(id), settings, global_ctx);
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
                        return self.update(Message::ChangeTalentLevel(index, level), settings, global_ctx);
                    }
                    talents::Message::LevelInputChanged(index, input) => {
                        let max_level = self.selected_cat
                            .and_then(|cat_id| self.data.cats.iter().find(|c| c.id == cat_id))
                            .and_then(|cat| cat.talent_data.as_ref())
                            .and_then(|talent_data| talent_data.groups.get(index as usize))
                            .map(|group| group.max_level.max(1))
                            .unwrap_or(u8::MAX);

                        if let Ok(parsed) = input.trim().parse::<u8>() {
                            self.talent_levels.insert(index, parsed.min(max_level));
                        } else if input.trim().is_empty() {
                            self.talent_levels.insert(index, 0);
                        }

                        self.talent_level_inputs.insert(index, input);
                    }
                    other => self.talents.update(other),
                }
                Task::none()
            }
            Message::Animation(msg) => self.animation.update(msg, settings).map(Message::Animation),
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
        let talent_levels = if form_allows_talents { Some(&self.talent_levels) } else { None };
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

    fn finish_statblock_job(&mut self, job: JobResult) {
        match job {
            JobResult::Copy(Ok(image)) => {
                let result = match self.ensure_clipboard() {
                    Some(clipboard) => builder::copy_to_clipboard(clipboard, &image),
                    None => Err("Clipboard unavailable".to_string()),
                };
                if let Err(err) = &result {
                    error!("Cat statblock copy failed: {err}");
                }
                self.statblock_copy_feedback = Some((result.is_ok(), Instant::now()));
            }
            JobResult::Copy(Err(err)) => {
                error!("Cat statblock export failed: {err}");
                self.statblock_copy_feedback = Some((false, Instant::now()));
            }
            JobResult::Save(result) => {
                self.statblock_save_feedback = Some((result.is_ok(), Instant::now()));
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

    pub fn expanded_animation_view(&self) -> Option<Element<'_, Message>> {
        self.animation.expanded_view().map(|view| view.map(Message::Animation))
    }

    pub fn export_popup_open(&self) -> bool {
        self.animation.export_popup_open()
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

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.animation.export_popup_view(window).map(|view| view.map(Message::Animation))
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content(settings, global_ctx);

        let base_layout = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        base_layout.into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let search_input = text_input("Search Cat...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(8)
            .width(Length::Fill);

        let filter_button = button(text("Filter").align_x(Horizontal::Center))
            .on_press(Message::Filter(filter::Message::Toggle))
            .width(Length::Fill);

        let cat_list = self.list.view(&self.data.cats, self.selected_cat).map(Message::List);

        let mut sidebar = column![
            row![search_input, filter_button].spacing(4),
            Space::new().height(Length::Fixed(8.0)),
        ];

        if let Some((done, total)) = self.scan_progress {
            sidebar = sidebar.push(text(format!("Scanning cats... {}/{}", done, total)).size(12));
        }

        sidebar = sidebar.push(cat_list);

        container(
            sidebar
                .spacing(4)
                .height(Length::Fill)
        )
            .width(Length::Fixed(220.0))
            .height(Length::Fill)
            .padding(8)
            .style(container::bordered_box)
            .into()
    }

    fn view_main_content<'a>(&'a self, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
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
            DetailTab::Animation => self.animation.view().map(Message::Animation),
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

    fn view_header(&self, cat: &CatEntry) -> Element<'_, Message> {
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

        let copy_busy = self.statblock_pending == Some(ExportAction::Copy);
        let copy_feedback = self.statblock_copy_feedback;
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
        let save_feedback = self.statblock_save_feedback;
        let save_label = feedback_label(save_busy, save_feedback, "Export Image", "Exporting...", "Exported!", "Failed!");
        let save_btn = button(text(save_label).size(12))
            .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportStatblock(ExportAction::Save)))
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(Background::Color(feedback_color(theme, save_busy, save_feedback))),
                text_color: Color::WHITE,
                border: Border::default().rounded(4.0),
                ..Default::default()
            });

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

    fn view_abilities<'a>(&'a self, cat: &'a CatEntry, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let dynamic_stats = unitid(cat.id as i32, &settings.general.language_priority);
        let Some(base_stats) = dynamic_stats.as_ref().and_then(|v| v.get(self.selected_form)) else {
            return container(text("Stats data not found")).into();
        };

        let form_allows_talents = self.selected_form >= 2;
        let talent_data = if form_allows_talents { cat.talent_data.as_ref() } else { None };
        let talent_levels = if form_allows_talents { Some(&self.talent_levels) } else { None };

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

        let atk_str = format_cat_stat("Attack", final_stats, anim_frames, unitbuy_opt);
        let dps_str = format_cat_stat("Dps", final_stats, anim_frames, unitbuy_opt);
        let range_str = format_cat_stat("Range", final_stats, anim_frames, unitbuy_opt);
        let rarity_str = format_cat_stat("Rarity", final_stats, anim_frames, unitbuy_opt);
        let hp_str = format_cat_stat("Hitpoints", final_stats, anim_frames, unitbuy_opt);
        let kb_str = format_cat_stat("Knockbacks", final_stats, anim_frames, unitbuy_opt);
        let speed_str = format_cat_stat("Speed", final_stats, anim_frames, unitbuy_opt);
        let cost_str = format_cat_stat("Cost", final_stats, anim_frames, unitbuy_opt);

        let cycle = (get_cat_stat("Atk Cycle").get_value)(final_stats, anim_frames, unitbuy_opt);
        let cd_val = (get_cat_stat("Cooldown").get_value)(final_stats, anim_frames, unitbuy_opt);

        let header_row = row![
            stat_grid::grid_cell(get_cat_stat("Attack").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Dps").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Range").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Atk Cycle").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Rarity").display_name, true),
        ].spacing(4);

        let value_row = row![
            stat_grid::grid_cell(&atk_str, false),
            stat_grid::grid_cell(&dps_str, false),
            stat_grid::grid_cell(&range_str, false),
            stat_grid::grid_cell_element(stat_grid::render_frames(cycle, 60.0), false),
            stat_grid::grid_cell(&rarity_str, false),
        ].spacing(4);

        let header_row2 = row![
            stat_grid::grid_cell(get_cat_stat("Hitpoints").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Knockbacks").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Speed").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Cooldown").display_name, true),
            stat_grid::grid_cell(get_cat_stat("Cost").display_name, true),
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

        let talents_view = self.talents.view(
            cat.id,
            talent_data,
            &self.talent_levels,
            &self.talent_level_inputs,
            &cat.talent_costs,
            &cat.skill_descriptions,
            base_stats,
            cat.curve.as_ref(),
            self.current_level,
            &self.img015_sheets,
            &self.img022_sheets,
            &self.custom_assets,
            settings,
        ).map(Message::Talents);

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
