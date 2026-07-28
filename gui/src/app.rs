use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use iced::alignment;
use iced::widget::{button, column, container, progress_bar, row, scrollable, stack, text};
use iced::{window, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{debug, info, trace, warn};

use core::common::context::GlobalContext;
use core::common::game::{localizable, param};
use core::common::io::cache;
use core::common::resolver;
use core::modules::cat::paths as cat_paths;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::waiter::{skilldescriptions, skilllevel};
use core::modules::enemy::scanner::EnemyEntry;
use core::modules::settings::{Settings, UpdateMode};
use core::modules::stage::StageRegistry;

use crate::common::watcher::GuiWatcher;
use crate::modules::{cat, data, enemy, home, mods, settings as gui_settings, stage};

mod logging;
mod updater;

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize, Debug)]
pub enum Page {
    Home,
    Cats,
    Enemies,
    Stages,
    Mods,
    Data,
    Settings,
}

impl Page {
    pub fn tab_name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Cats => "Cats",
            Self::Enemies => "Enemies",
            Self::Stages => "Stages",
            Self::Mods => "Mods",
            Self::Data => "Data",
            Self::Settings => "Settings",
        }
    }
}

pub const ALL_PAGES: &[Page] = &[
    Page::Home,
    Page::Cats,
    Page::Enemies,
    Page::Stages,
    Page::Mods,
    Page::Data,
    Page::Settings,
];

#[derive(Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpdateFound(String, Release),
    Downloading(String),
    RestartPending(String),
    CheckFailed,
    UpToDate,
}

#[derive(Clone, Debug)]
pub enum UpdaterMsg {
    UpdateFound(Release),
    UpToDate,
    CheckFailed,
    DownloadStarted(String),
    DownloadFinished(String),
    SilentFail,
}

#[derive(Clone, Debug)]
pub enum UpdaterAction {
    StartDownload(Release),
    DismissUpdate,
    NeverUpdate,
    RestartApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivePopup {
    CatExport,
    CatFilter,
    EnemyExport,
    EnemyFilter,
    StageFilter,
    ModsImport,
    ModsExport,
    SettingsKeys,
    SettingsExceptions,
    SettingsPem,
}

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    Navigate(Page),
    ToggleSidebar,
    WindowResized(Size),
    UpdaterAction(UpdaterAction),
    Home(home::Message),
    Cat(cat::Message),
    Enemy(enemy::Message),
    Stage(stage::Message),
    Mod(mods::Message),
    Data(data::Message),
    Settings(gui_settings::Message),
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BattleCatsApp {
    #[serde(skip)]
    pub current_page: Page,
    #[serde(skip)]
    pub sidebar_open: bool,
    #[serde(skip)]
    pub window_size: Size,
    #[serde(skip)]
    pub active_popups: Vec<ActivePopup>,

    #[serde(skip)]
    pub home_state: home::State,
    #[serde(skip)]
    pub cat_state: cat::State,
    #[serde(skip)]
    pub enemy_state: enemy::EnemyState,
    #[serde(skip)]
    pub stage_state: stage::State,
    #[serde(skip)]
    pub mods_state: mods::State,
    #[serde(skip)]
    pub data_state: data::State,
    #[serde(skip)]
    pub settings_state: gui_settings::State,

    pub settings: Settings,

    #[serde(skip)]
    pub param: Param,
    #[serde(skip)]
    pub localizable: Localizable,
    #[serde(skip)]
    pub global_watcher: Option<GuiWatcher>,
    #[serde(skip)]
    pub last_saved_hash: u64,

    #[serde(skip)]
    pub hash_rx: Option<Receiver<bool>>,
    #[serde(skip)]
    pub updater_rx: Option<Receiver<UpdaterMsg>>,
    #[serde(skip)]
    pub updater_tx: Option<Sender<UpdaterMsg>>,
    #[serde(skip)]
    pub updater_status: UpdateStatus,
    #[serde(skip)]
    pub download_progress: f32,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::Home,
            sidebar_open: true,
            window_size: Size::new(800.0, 600.0),
            active_popups: Vec::new(),
            home_state: home::State::default(),
            cat_state: cat::State::default(),
            enemy_state: enemy::EnemyState::default(),
            stage_state: stage::State::default(),
            mods_state: mods::State::new(core::modules::mods::ModDataState::default()),
            data_state: data::State::default(),
            settings_state: gui_settings::State::default(),
            settings: Settings::default(),
            param: Param::default(),
            localizable: Localizable::default(),
            global_watcher: None,
            last_saved_hash: 0,
            hash_rx: None,
            updater_rx: None,
            updater_tx: None,
            updater_status: UpdateStatus::Idle,
            download_progress: 0.0,
        }
    }
}

impl BattleCatsApp {
    pub fn new() -> (Self, Task<Message>) {
        let mut app = Self::default();

        logging::init_logging(app.settings.general.enable_logging);

        info!("Loading core tables");
        let tables_dir = std::path::Path::new("game/tables");
        let loc_dir = std::path::Path::new("game/tables/localizable");
        let priority = &app.settings.general.language_priority;

        app.param = param(tables_dir, priority).unwrap_or_default();
        app.localizable = localizable(loc_dir, priority);

        let mut expected_hash = 0;
        let mut needs_validation = false;

        if let Some((hash, cached_cats)) = cache::load_with_hash::<Vec<CatEntry>>("cats_cache.bin") {
            info!("Found cats_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            let cats_dir = std::path::Path::new(cat_paths::DIR_CATS);
            let costs_arc = Arc::new(skilllevel(cats_dir, priority));
            let descriptions_arc = Arc::new(skilldescriptions(cats_dir, priority));

            app.cat_state.data.cats = cached_cats.into_iter().map(|mut cat| {
                cat.talent_costs = Arc::clone(&costs_arc);
                cat.skill_descriptions = Arc::clone(&descriptions_arc);
                cat
            }).collect();
        } else {
            info!("No cats_cache.bin found, triggering full cat scan");
            app.cat_state.data.restart_scan(app.settings.scanner_config());
        }
        app.cat_state.data.initialized = true;

        if let Some((hash, cached_enemies)) = cache::load_with_hash::<Vec<EnemyEntry>>("enemies_cache.bin") {
            info!("Found enemies_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;
            app.enemy_state.data.enemies = cached_enemies;
        } else {
            info!("No enemies_cache.bin found, triggering full enemy scan");
            app.enemy_state.data.restart_scan(app.settings.scanner_config());
        }
        app.enemy_state.data.initialized = true;

        if let Some((hash, cached_registry)) = cache::load_with_hash::<StageRegistry>("stages_cache.bin") {
            info!("Found stages_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            app.stage_state.data.registry = cached_registry;

            let config = app.settings.scanner_config();
            app.stage_state.data.load_dictionaries(&config);

            let enemies_ref = app.enemy_state.data.enemies.clone();
            app.stage_state.data.sync_enemies(&enemies_ref);
        } else {
            info!("No stages_cache.bin found, triggering full stage scan");
            app.stage_state.data.restart_scan(app.settings.scanner_config());
        }
        app.stage_state.data.initialized = true;

        if needs_validation {
            debug!("Spawning hash validation thread");
            let (tx, rx) = std::sync::mpsc::channel();
            app.hash_rx = Some(rx);
            let active_mod = resolver::get_active_mod();

            thread::spawn(move || {
                let current_hash = cache::get_game_hash(active_mod.as_deref());
                let is_valid = current_hash == expected_hash && active_mod.is_none();
                let _ = tx.send(is_valid);
            });
        }

        let (up_tx, up_rx) = std::sync::mpsc::channel();
        app.updater_tx = Some(up_tx);
        app.updater_rx = Some(up_rx);

        if app.settings.general.update_mode != UpdateMode::Ignore {
            info!("Checking for app updates at startup");
            app.check_for_updates(false);
        }

        let (home_state, home_task) = home::State::new();
        app.home_state = home_state;

        (app, home_task.map(Message::Home))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            window::resize_events().map(|(_, size)| Message::WindowResized(size)),
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
            self.home_state.subscription().map(Message::Home),
            self.cat_state.subscription().map(Message::Cat),
            self.enemy_state.subscription().map(Message::Enemy),
            self.stage_state.subscription().map(Message::Stage),
            self.mods_state.subscription().map(Message::Mod),
            self.data_state.subscription().map(Message::Data),
            self.settings_state.subscription().map(Message::Settings),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(size) => {
                self.window_size = size;
                Task::none()
            }
            Message::Tick => {
                self.check_auto_save();

                if self.settings.runtime.manual_check_requested {
                    info!("Manual update check requested by user");
                    self.settings.runtime.manual_check_requested = false;
                }

                if let Some(rx) = &self.hash_rx {
                    if let Ok(is_valid) = rx.try_recv() {
                        self.hash_rx = None;
                        if !is_valid {
                            warn!("Cache hash validation failed! Performing full data reload.");
                        } else {
                            info!("Cache hash validation passed.");
                        }
                    }
                }

                if let Some(rx) = &self.updater_rx {
                    while let Ok(msg) = rx.try_recv() {
                        match msg {
                            UpdaterMsg::UpdateFound(release) => {
                                self.updater_status = UpdateStatus::UpdateFound(release.version.clone(), release);
                            }
                            UpdaterMsg::UpToDate => {
                                self.updater_status = UpdateStatus::UpToDate;
                            }
                            UpdaterMsg::CheckFailed => {
                                self.updater_status = UpdateStatus::CheckFailed;
                            }
                            UpdaterMsg::DownloadStarted(version) => {
                                self.updater_status = UpdateStatus::Downloading(version);
                                self.download_progress = 0.0;
                            }
                            UpdaterMsg::DownloadFinished(version) => {
                                self.updater_status = UpdateStatus::RestartPending(version);
                            }
                            UpdaterMsg::SilentFail => {
                                self.updater_status = UpdateStatus::Idle;
                            }
                        }
                    }
                }

                if let UpdateStatus::Downloading(_) = self.updater_status {
                    self.download_progress += 0.01;
                    if self.download_progress > 1.0 {
                        self.download_progress = 0.0;
                    }
                }

                Task::none()
            }
            Message::Navigate(page) => {
                self.current_page = page;
                Task::none()
            }
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                Task::none()
            }
            Message::UpdaterAction(action) => {
                match action {
                    UpdaterAction::StartDownload(release) => {
                        self.download_and_install(release);
                    }
                    UpdaterAction::DismissUpdate => {
                        self.updater_status = UpdateStatus::Idle;
                    }
                    UpdaterAction::NeverUpdate => {
                        info!("User selected Never update, changing mode to Ignore");
                        self.settings.general.update_mode = UpdateMode::Ignore;
                        self.updater_status = UpdateStatus::Idle;
                    }
                    UpdaterAction::RestartApp => {
                        updater::restart_app();
                    }
                }
                Task::none()
            }
            Message::Home(msg) => {
                match msg {
                    home::Message::Navigate(page) => {
                        self.current_page = page;
                        Task::none()
                    }
                    home::Message::NavigateSettings(_tab_str) => {
                        self.current_page = Page::Settings;
                        Task::none()
                    }
                    _ => self.home_state.update(msg).map(Message::Home),
                }
            }
            Message::Cat(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let task = self.cat_state.update(msg, &mut self.settings, global_ctx).map(Message::Cat);
                self.sync_popup(ActivePopup::CatExport, self.cat_state.export_popup_open());
                self.sync_popup(ActivePopup::CatFilter, self.cat_state.filter_popup_open());
                task
            }
            Message::Enemy(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let task = self.enemy_state.update(msg, &mut self.settings, global_ctx).map(Message::Enemy);
                self.sync_popup(ActivePopup::EnemyFilter, self.enemy_state.filter_popup_open());
                self.sync_popup(ActivePopup::EnemyExport, self.enemy_state.export_popup_open());
                task
            }
            Message::Stage(msg) => {
                let task = self.stage_state.update(msg, &self.settings).map(Message::Stage);
                self.sync_popup(ActivePopup::StageFilter, self.stage_state.filter_popup_open());
                task
            }
            Message::Mod(msg) => {
                let task = self.mods_state.update(msg, &self.settings).map(Message::Mod);
                self.sync_popup(ActivePopup::ModsImport, self.mods_state.import_popup_open());
                self.sync_popup(ActivePopup::ModsExport, self.mods_state.export_popup_open());
                task
            }
            Message::Data(msg) => self.data_state.update(msg, &mut self.settings).map(Message::Data),
            Message::Settings(msg) => {
                let task = self.settings_state.update(msg, &mut self.settings).map(Message::Settings);
                self.sync_popup(ActivePopup::SettingsKeys, self.settings_state.keys_popup_open());
                self.sync_popup(ActivePopup::SettingsExceptions, self.settings_state.exceptions_popup_open());
                self.sync_popup(ActivePopup::SettingsPem, self.settings_state.pem_popup_open());
                task
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_page {
            Page::Home => self.home_state.view().map(Message::Home),
            Page::Cats => self.cat_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Cat),
            Page::Enemies => self.enemy_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Enemy),
            Page::Stages => self.stage_state.view(GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Stage),
            Page::Mods => self.mods_state.view().map(Message::Mod),
            Page::Data => self.data_state.view(&self.settings).map(Message::Data),
            Page::Settings => self.settings_state.view(&self.settings).map(Message::Settings),
        };

        let content_container = container(content)
            .width(Length::Fill)
            .height(Length::Fill);

        let sidebar_overlay = self.view_sidebar_overlay();

        let expanded: Option<Element<'_, Message>> = match self.current_page {
            Page::Cats => self.cat_state.expanded_animation_view().map(|view| view.map(Message::Cat)),
            Page::Enemies => self.enemy_state.expanded_animation_view().map(|view| view.map(Message::Enemy)),
            _ => None,
        };

        let mut layers = stack![content_container];

        if expanded.is_none() {
            for popup in self.build_popups() {
                layers = layers.push(popup);
            }
        }

        layers = layers.push(sidebar_overlay);

        if let Some(expanded) = expanded {
            layers = layers.push(expanded);

            for popup in self.build_popups() {
                layers = layers.push(popup);
            }
        }

        if let Some(modal) = self.build_modal() {
            layers = layers.push(modal);
        }

        layers.into()
    }

    fn sync_popup(&mut self, popup: ActivePopup, open: bool) {
        if open {
            if !self.active_popups.contains(&popup) {
                self.active_popups.push(popup);
            }
        } else {
            self.active_popups.retain(|active| *active != popup);
        }
    }

    fn build_popups(&self) -> Vec<Element<'_, Message>> {
        self.active_popups
            .iter()
            .filter_map(|popup| match popup {
                ActivePopup::CatExport => {
                    if !matches!(self.current_page, Page::Cats) || !self.cat_state.export_popup_visible() {
                        return None;
                    }

                    self.cat_state.export_popup_view(self.window_size).map(|view| view.map(Message::Cat))
                }
                ActivePopup::CatFilter => {
                    if !matches!(self.current_page, Page::Cats) {
                        return None;
                    }

                    self.cat_state.filter_popup_view(self.window_size).map(|view| view.map(Message::Cat))
                }
                ActivePopup::EnemyExport => {
                    if !matches!(self.current_page, Page::Enemies) || !self.enemy_state.export_popup_visible() {
                        return None;
                    }

                    self.enemy_state.export_popup_view(self.window_size).map(|view| view.map(Message::Enemy))
                }
                ActivePopup::EnemyFilter => {
                    if !matches!(self.current_page, Page::Enemies) {
                        return None;
                    }

                    self.enemy_state.filter_popup_view(self.window_size).map(|view| view.map(Message::Enemy))
                }
                ActivePopup::StageFilter => {
                    if !matches!(self.current_page, Page::Stages) {
                        return None;
                    }

                    self.stage_state.filter_popup_view(self.window_size).map(|view| view.map(Message::Stage))
                }
                ActivePopup::ModsImport => {
                    if !matches!(self.current_page, Page::Mods) {
                        return None;
                    }

                    self.mods_state.import_popup_view(self.window_size).map(|view| view.map(Message::Mod))
                }
                ActivePopup::ModsExport => {
                    if !matches!(self.current_page, Page::Mods) {
                        return None;
                    }

                    self.mods_state.export_popup_view(self.window_size).map(|view| view.map(Message::Mod))
                }
                ActivePopup::SettingsKeys => {
                    if !matches!(self.current_page, Page::Settings) {
                        return None;
                    }

                    self.settings_state.keys_popup_view(self.window_size).map(|view| view.map(Message::Settings))
                }
                ActivePopup::SettingsExceptions => {
                    if !matches!(self.current_page, Page::Settings) {
                        return None;
                    }

                    self.settings_state.exceptions_popup_view(self.window_size).map(|view| view.map(Message::Settings))
                }
                ActivePopup::SettingsPem => {
                    if !matches!(self.current_page, Page::Settings) {
                        return None;
                    }

                    self.settings_state.pem_popup_view(self.window_size).map(|view| view.map(Message::Settings))
                }
            })
            .collect()
    }

    fn view_sidebar_overlay(&self) -> Element<'_, Message> {
        let arrow_text = if self.sidebar_open { "▶" } else { "◀" };
        let toggle_btn = button(text(arrow_text).size(20).align_x(alignment::Horizontal::Center))
            .width(Length::Fixed(40.0))
            .height(Length::Fixed(40.0))
            .on_press(Message::ToggleSidebar)
            .style(|theme: &Theme, _status| button::primary(theme, _status));

        let toggle_container = column![toggle_btn]
            .padding(iced::Padding {
                top: 2.5,
                right: 10.0,
                bottom: 0.0,
                left: 0.0,
            });

        let mut layer = row![toggle_container].height(Length::Fill);

        if self.sidebar_open {
            let mut tabs: iced::widget::Column<'_, Message> = column![].spacing(10);
            for page in ALL_PAGES {
                let is_active = self.current_page == *page;
                let btn = button(text(page.tab_name()).size(16).align_x(alignment::Horizontal::Center))
                    .width(Length::Fill)
                    .padding(10)
                    .on_press(Message::Navigate(*page))
                    .style(move |theme: &Theme, _status| {
                        if is_active {
                            button::primary(theme, _status)
                        } else {
                            button::secondary(theme, _status)
                        }
                    });

                tabs = tabs.push(btn);
            }

            let sidebar_panel = container(tabs)
                .width(Length::Fixed(180.0))
                .height(Length::Fill)
                .padding(15)
                .style(|theme: &Theme| {
                    let palette = theme.palette();
                    container::Style {
                        background: Some(palette.background.into()),
                        border: iced::border::rounded(10).color(palette.text).width(1),
                        ..Default::default()
                    }
                });

            layer = layer.push(sidebar_panel);
        }

        container(layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_right(Length::Fill)
            .into()
    }

    fn build_modal(&self) -> Option<Element<'_, Message>> {
        let modal_content: Element<Message> = match &self.updater_status {
            UpdateStatus::UpdateFound(tag, release) => {
                let display_version = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Update Available").size(24),
                    text(format!("New Battle Cats Complete update found: {}", display_version)),
                    text("Would you like to download the update now?"),
                    row![
                        button("Yes").on_press(Message::UpdaterAction(UpdaterAction::StartDownload(release.clone()))),
                        button("No").on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
                        button("Never").on_press(Message::UpdaterAction(UpdaterAction::NeverUpdate)),
                    ].spacing(15)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            UpdateStatus::Downloading(tag) => {
                let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Downloading Update").size(24),
                    text(format!("Downloading {}...", display_tag)),
                    progress_bar(0.0..=1.0, self.download_progress)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            UpdateStatus::RestartPending(tag) => {
                let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Update Complete").size(24),
                    text(format!("{} update complete!", display_tag)),
                    text("Would you like to restart and apply the update now?"),
                    row![
                        button("Yes").on_press(Message::UpdaterAction(UpdaterAction::RestartApp)),
                        button("No").on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
                    ].spacing(15)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            _ => return None,
        };

        let modal_card = container(
            scrollable(modal_content)
                .width(Length::Fill)
                .height(Length::Shrink)
        )
            .padding(30)
            .width(Length::Fixed(400.0))
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(palette.background.into()),
                    border: iced::border::rounded(10).color(palette.text).width(2),
                    ..Default::default()
                }
            });

        let overlay = container(modal_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| {
                container::Style {
                    background: Some(Color::from_rgba8(0, 0, 0, 0.7).into()),
                    ..Default::default()
                }
            });

        Some(overlay.into())
    }

    fn check_auto_save(&mut self) {
        let Ok(json_string) = serde_json::to_string(self) else { return; };

        let mut hasher = FxHasher::default();
        json_string.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.last_saved_hash != current_hash {
            trace!("Settings changed. Saving to settings.json");
            self.last_saved_hash = current_hash;
        }
    }
}