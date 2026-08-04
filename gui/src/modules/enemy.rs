mod abilities;
mod filter;
mod list;
mod statblock;

use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use iced::futures::channel::mpsc;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Id, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::enemy::unit::Battle;
use tracing::{error, info};

use core::common::context::GlobalContext;
use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::modules::enemy::game::registry::{format_enemy_stat, Magnification, STAT_ATK_CYCLE, STAT_ATTACK, STAT_CASH_DROP, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_SPEED};
use core::modules::enemy::game::EnemyRenderContext;
use core::modules::enemy::scanner::{self, EnemyEntry};
use core::modules::enemy::{EnemyDataState, EnemyDetailTab};
use core::modules::settings::Settings;

use crate::app::state::{AppState, EnemyListState};
use crate::app::theme;
use crate::common::feedback::Slot;
use crate::common::stat_grid;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::modules::animation;
use crate::modules::statblock::{feedback_color, feedback_label};

use super::statblock::{builder, JobResult};
use statblock::build_enemy_statblock;

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
    Loaded(Vec<EnemyEntry>),
    Img015Loaded(usize, Option<CoreSpriteSheet>),
    StatblockFinished(JobResult),
    CopyFeedbackExpired,
    SaveFeedbackExpired,
    SearchQueryChanged(String),
    EnemySelected(u32),
    TabSelected(EnemyDetailTab),
    MagnificationChanged(String),
    ExportClicked(ExportAction),
    NavigateAppearances(u32),
    Filter(filter::Message),
    List(list::Message),
    Animation(animation::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnimationTick => write!(f, "AnimationTick"),
            Self::SheetsCheck => write!(f, "SheetsCheck"),
            Self::ScanProgress(done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(enemies) => write!(f, "Loaded({})", enemies.len()),
            Self::Img015Loaded(i, _) => write!(f, "Img015Loaded({})", i),
            Self::StatblockFinished(_) => write!(f, "StatblockFinished"),
            Self::CopyFeedbackExpired => write!(f, "CopyFeedbackExpired"),
            Self::SaveFeedbackExpired => write!(f, "SaveFeedbackExpired"),
            Self::SearchQueryChanged(s) => write!(f, "SearchQueryChanged({})", s),
            Self::EnemySelected(id) => write!(f, "EnemySelected({})", id),
            Self::TabSelected(tab) => write!(f, "TabSelected({:?})", tab),
            Self::MagnificationChanged(s) => write!(f, "MagnificationChanged({})", s),
            Self::ExportClicked(_) => write!(f, "ExportClicked"),
            Self::NavigateAppearances(id) => write!(f, "NavigateAppearances({})", id),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Animation(msg) => write!(f, "Animation({:?})", msg),
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

    scan_progress: Option<(usize, usize)>,

    statblock_pending: Option<ExportAction>,
    statblock_clipboard: Option<Clipboard>,
    statblock_copy_feedback: Slot<bool>,
    statblock_save_feedback: Slot<bool>,

    filter: filter::State,
    list: list::State,
    abilities: abilities::State,
    animation: animation::State,
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

            scan_progress: None,

            statblock_pending: None,
            statblock_clipboard: None,
            statblock_copy_feedback: Slot::default(),
            statblock_save_feedback: Slot::default(),

            filter: filter::State::default(),
            list: list::State::default(),
            abilities: abilities::State::default(),
            animation: animation::State::default(),
        }
    }
}

impl EnemyState {
    pub(crate) fn list_scrollable_id() -> Id {
        list::scrollable_id()
    }

    pub(crate) fn list_scroll_offset(&self) -> f32 {
        self.list.scroll_offset()
    }

    pub(crate) fn restore_state(&mut self, state: &EnemyListState) {
        self.list.set_scroll_offset(state.list_scroll_offset);
        self.data.selected_enemy = state.selected_enemy;
        self.search_query = state.search_query.clone();
    }

    pub(crate) fn sync_state(&self, state: &mut EnemyListState) {
        state.list_scroll_offset = self.list.scroll_offset();
        state.selected_enemy = self.data.selected_enemy;
        if state.search_query != self.search_query {
            state.search_query = self.search_query.clone();
        }
    }

    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.selected_tab == EnemyDetailTab::Animation {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        }
    }

    pub fn start_load(&mut self, settings: &Settings) -> Task<Message> {
        info!("Triggering initial enemy load");
        let config = settings.scanner_config();
        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            let enemies = scanner::load(config, |done, total| {
                let _ = tx.unbounded_send(Message::ScanProgress(done, total));
            });
            let _ = tx.unbounded_send(Message::Loaded(enemies));
        });

        Task::batch([Task::stream(rx), self.check_sheets(settings)])
    }

    pub fn rescan(&mut self, settings: &Settings) -> Task<Message> {
        info!("Rescanning enemies for active-mod change");
        self.animation.invalidate_paths();
        self.start_load(settings)
    }

    fn check_sheets(&mut self, settings: &Settings) -> Task<Message> {
        crate::common::img015::ensure_loaded(&mut self.img015_sheets, settings)
            .map(|(index, sheet)| Message::Img015Loaded(index, sheet))
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState, global_ctx: GlobalContext<'_>) -> Task<Message> {
        let task = self.update_inner(message, settings, app_state, global_ctx);

        self.list.refresh(&self.data.enemies, &self.search_query, &self.filter.filter_state);

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
            Message::ScanProgress(done, total) => {
                if self.scan_progress.is_none_or(|(prev, _)| done > prev) {
                    self.scan_progress = Some((done, total));
                }
                Task::none()
            }
            Message::Loaded(enemies) => {
                info!("Enemy load finished with {} entries", enemies.len());
                self.scan_progress = None;
                self.list.invalidate();
                self.data.enemies = enemies;
                match self.data.selected_enemy.and_then(|id| self.data.enemies.iter().find(|e| e.id == id)) {
                    Some(enemy) => self.animation.preload_enemy(enemy, settings).map(Message::Animation),
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
                if let Some(enemy) = self.data.selected_enemy.and_then(|id| self.data.enemies.iter().find(|e| e.id == id)) {
                    self.animation.sync_enemy(enemy, settings, &app_state.animation);
                }
                self.animation.tick();
                Task::none()
            }
            Message::SearchQueryChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::EnemySelected(id) => {
                if self.data.selected_enemy != Some(id) {
                    self.data.selected_enemy = Some(id);
                    self.mag_input = "100".to_string();
                    self.magnification = Magnification { hitpoints: 100, attack: 100 };
                    self.animation.reset_playhead();
                    return match self.data.enemies.iter().find(|e| e.id == id) {
                        Some(enemy) => self.animation.preload_enemy(enemy, settings).map(Message::Animation),
                        None => Task::none(),
                    };
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
            Message::ExportClicked(action) => self.start_statblock_export(action, settings, global_ctx),
            Message::NavigateAppearances(_) => Task::none(),
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
            Message::List(msg) => {
                if let list::Message::SelectEnemy(id) = msg {
                    return self.update(Message::EnemySelected(id), settings, app_state, global_ctx);
                }

                self.list.update(msg);
                Task::none()
            }
            Message::Animation(msg) => self.animation.update(msg, settings, &mut app_state.animation).map(Message::Animation),
        }
    }

    fn start_statblock_export(&mut self, action: ExportAction, settings: &Settings, global_ctx: GlobalContext<'_>) -> Task<Message> {
        if self.statblock_pending.is_some() {
            return Task::none();
        }

        let Some(selected_id) = self.data.selected_enemy else { return Task::none(); };
        let Some(enemy_entry) = self.data.enemies.iter().find(|e| e.id == selected_id) else { return Task::none(); };

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

        self.statblock_pending = Some(action);

        Task::perform(async move {
            let build_result = builder::build_statblock_image(&priority, data, cuts_map);

            match action {
                ExportAction::Copy => JobResult::Copy(build_result),
                ExportAction::Save => {
                    let result = build_result.and_then(|image| builder::save_to_disk(&image, is_cat, &id_str, &top_value).map(|_| ()));
                    if let Err(err) = &result {
                        error!("Enemy statblock save failed: {err}");
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
                    error!("Enemy statblock copy failed: {err}");
                }
                self.statblock_copy_feedback.set(result.is_ok(), Message::CopyFeedbackExpired)
            }
            JobResult::Copy(Err(err)) => {
                error!("Enemy statblock export failed: {err}");
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

    pub fn view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        row![
            self.view_sidebar(),
            self.view_main_panel(settings, app_state, global_ctx),
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

    pub fn expanded_animation_view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Option<Element<'a, Message>> {
        self.animation.expanded_view(settings, &app_state.animation).map(|view| view.map(Message::Animation))
    }

    pub fn export_popup_open(&self, app_state: &AppState) -> bool {
        self.animation.export_popup_open(&app_state.animation)
    }

    pub fn export_popup_visible(&self) -> bool {
        self.selected_tab == EnemyDetailTab::Animation
    }

    pub fn export_popup_view(&self, window: Size, app_state: &AppState) -> Option<Element<'_, Message>> {
        self.animation.export_popup_view(window, &app_state.animation).map(|view| view.map(Message::Animation))
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let search_bar = row![
            text_input("Search Enemy...", &self.search_query)
                .on_input(Message::SearchQueryChanged)
                .width(Length::Fill),
            button(text("Filter"))
                .on_press(Message::Filter(filter::Message::Toggle))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, self.filter.filter_state.is_active()))
        ]
            .spacing(5)
            .padding(5);

        let enemy_list = self.list.view(&self.data.enemies, self.data.selected_enemy).map(Message::List);

        let mut sidebar = column![search_bar];

        if let Some((done, total)) = self.scan_progress {
            sidebar = sidebar.push(text(format!("Scanning enemies... {}/{}", done, total)).size(12));
        }

        sidebar = sidebar.push(enemy_list);

        container(
            sidebar
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

    fn view_main_panel<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
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
                self.animation.view(settings, &app_state.animation).map(Message::Animation)
            }
        };

        let body: Element<'_, Message> = if self.selected_tab == EnemyDetailTab::Animation {
            content
        } else {
            scrollable(content).height(Length::Fill).into()
        };

        container(
            column![
                header,
                Space::new().height(Length::Fixed(10.0)),
                body
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
                .style(move |t: &Theme, status| theme::toggle_button(t, status, self.selected_tab == EnemyDetailTab::Abilities)),
            button("Details").on_press(Message::TabSelected(EnemyDetailTab::Details))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, self.selected_tab == EnemyDetailTab::Details)),
            button("Animation").on_press(Message::TabSelected(EnemyDetailTab::Animation))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, self.selected_tab == EnemyDetailTab::Animation)),
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
            let copy_busy = self.statblock_pending == Some(ExportAction::Copy);
            let copy_feedback = self.statblock_copy_feedback.get().copied();
            let copy_label = feedback_label(copy_busy, copy_feedback, "Copy Image", "Copying...", "Copied!", "Failed!");
            let copy_btn = button(text(copy_label).size(12))
                .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportClicked(ExportAction::Copy)))
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
                .on_press_maybe(self.statblock_pending.is_none().then_some(Message::ExportClicked(ExportAction::Save)))
                .style(move |theme: &Theme, _status| button::Style {
                    background: Some(Background::Color(feedback_color(theme, save_busy, save_feedback))),
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

        let atk_str = format_enemy_stat(&STAT_ATTACK, stats, frames, mag);
        let dps_str = format_enemy_stat(&STAT_DPS, stats, frames, mag);
        let range_str = format_enemy_stat(&STAT_RANGE, stats, frames, mag);
        let cash_str = format_enemy_stat(&STAT_CASH_DROP, stats, frames, mag);

        let hp_str = format_enemy_stat(&STAT_HITPOINTS, stats, frames, mag);
        let kb_str = format_enemy_stat(&STAT_KNOCKBACKS, stats, frames, mag);
        let speed_str = format_enemy_stat(&STAT_SPEED, stats, frames, mag);

        let cycle = (STAT_ATK_CYCLE.get_value)(stats, frames, mag);

        let header_row = row![
            stat_grid::grid_header(STAT_ATTACK.display_name),
            stat_grid::grid_header(STAT_DPS.display_name),
            stat_grid::grid_header(STAT_RANGE.display_name),
            stat_grid::grid_header(STAT_ATK_CYCLE.display_name),
        ].spacing(4);

        let value_row = row![
            stat_grid::grid_value(STAT_ATTACK.display_name, &atk_str),
            stat_grid::grid_value(STAT_DPS.display_name, &dps_str),
            stat_grid::grid_value(STAT_RANGE.display_name, &range_str),
            stat_grid::grid_frames(STAT_ATK_CYCLE.display_name, cycle),
        ].spacing(4);

        let header_row2 = row![
            stat_grid::grid_header(STAT_HITPOINTS.display_name),
            stat_grid::grid_header(STAT_KNOCKBACKS.display_name),
            stat_grid::grid_header(STAT_SPEED.display_name),
            stat_grid::grid_header(STAT_CASH_DROP.display_name),
        ].spacing(4);

        let value_row2 = row![
            stat_grid::grid_value(STAT_HITPOINTS.display_name, &hp_str),
            stat_grid::grid_value(STAT_KNOCKBACKS.display_name, &kb_str),
            stat_grid::grid_value(STAT_SPEED.display_name, &speed_str),
            stat_grid::grid_value(STAT_CASH_DROP.display_name, &cash_str),
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
