use std::hash::{Hash, Hasher};

use iced::alignment;
use iced::widget::{button, column, container, operation, progress_bar, row, scrollable, stack, text};
use iced::{task, window, Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{info, trace, warn};

use core::common::context::GlobalContext;
use core::common::io::json;
use core::modules::settings::{Settings, UpdateMode};

use crate::common::{popup, watcher::GuiWatcher};
use crate::modules::{cat, data, enemy, home, mods, settings as gui_settings, stage};

use state::AppState;

mod logging;
mod migrate;
mod notice;
mod startup;
pub mod state;
pub(crate) mod theme;
mod updater;

pub use theme::AppTheme;

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

const ALL_PAGES: &[Page] = &[
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
enum ActivePopup {
    VersionNotice,
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
    AutoSave,
    DownloadTick,
    Navigate(Page),
    ToggleSidebar,
    WindowResized(Size),
    Updater(UpdaterMsg),
    UpdaterAction(UpdaterAction),
    Notice(popup::Message),
    AcknowledgeNotice,
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
    active_popups: Vec<ActivePopup>,
    #[serde(skip)]
    notice_popup: popup::State,
    #[serde(skip)]
    notice_open: bool,

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
    pub app_state: AppState,

    #[serde(skip)]
    pub param: Param,
    #[serde(skip)]
    pub localizable: Localizable,
    #[serde(skip)]
    pub global_watcher: Option<GuiWatcher>,
    #[serde(skip)]
    pub last_saved_hash: u64,
    #[serde(skip)]
    pub last_saved_state_hash: u64,

    #[serde(skip)]
    pub updater_handle: Option<task::Handle>,
    #[serde(skip)]
    pub updater_status: UpdateStatus,
    #[serde(skip)]
    pub download_progress: f32,

    pub app_theme: AppTheme,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::Home,
            sidebar_open: true,
            window_size: Size::new(800.0, 600.0),
            active_popups: Vec::new(),
            notice_popup: popup::State::default(),
            notice_open: false,
            home_state: home::State::default(),
            cat_state: cat::State::default(),
            enemy_state: enemy::EnemyState::default(),
            stage_state: stage::State::default(),
            mods_state: mods::State::new(core::modules::mods::ModDataState::default()),
            data_state: data::State::default(),
            settings_state: gui_settings::State::default(),
            settings: Settings::default(),
            app_state: AppState::default(),
            param: Param::default(),
            localizable: Localizable::default(),
            global_watcher: None,
            last_saved_hash: 0,
            last_saved_state_hash: 0,
            updater_handle: None,
            updater_status: UpdateStatus::Idle,
            download_progress: 0.0,
            app_theme: AppTheme::default(),
        }
    }
}

impl BattleCatsApp {
    pub fn theme(&self) -> Theme {
        self.app_theme.to_iced_theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            window::resize_events().map(|(_, size)| Message::WindowResized(size)),
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::AutoSave),
            self.cat_state.subscription().map(Message::Cat),
            self.enemy_state.subscription().map(Message::Enemy),
            self.mods_state.subscription().map(Message::Mod),
        ];

        if let UpdateStatus::Downloading(_) = self.updater_status {
            subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::DownloadTick));
        }

        Subscription::batch(subs)
    }

    fn navigate(&mut self, page: Page) -> Task<Message> {
        self.current_page = page;

        match page {
            Page::Home => self.home_state.update(home::Message::CheckInit).map(Message::Home),
            Page::Cats => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let sheets_task = self.cat_state.update(cat::Message::SheetsCheck, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Cat);
                let scroll_task = operation::scroll_to(
                    cat::State::list_scrollable_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: self.cat_state.list_scroll_offset() },
                );
                Task::batch([sheets_task, scroll_task])
            }
            Page::Enemies => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let sheets_task = self.enemy_state.update(enemy::Message::SheetsCheck, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Enemy);
                let scroll_task = operation::scroll_to(
                    enemy::EnemyState::list_scrollable_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: self.enemy_state.list_scroll_offset() },
                );
                Task::batch([sheets_task, scroll_task])
            }
            _ => Task::none(),
        }
    }

    fn rescan_units(&mut self) -> Task<Message> {
        Task::batch([
            self.cat_state.rescan(&self.settings).map(Message::Cat),
            self.enemy_state.rescan(&self.settings).map(Message::Enemy),
            self.stage_state.rescan(&self.settings).map(Message::Stage),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(size) => {
                self.window_size = size;
                Task::none()
            }
            Message::AutoSave => {
                self.check_auto_save();
                self.check_auto_save_state();
                Task::none()
            }
            Message::DownloadTick => {
                self.download_progress += 0.01;
                if self.download_progress > 1.0 {
                    self.download_progress = 0.0;
                }
                Task::none()
            }
            Message::Updater(msg) => {
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
                Task::none()
            }
            Message::Navigate(page) => self.navigate(page),
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                Task::none()
            }
            Message::UpdaterAction(action) => {
                match action {
                    UpdaterAction::StartDownload(release) => self.download_and_install(release),
                    UpdaterAction::DismissUpdate => {
                        self.updater_status = UpdateStatus::Idle;
                        Task::none()
                    }
                    UpdaterAction::NeverUpdate => {
                        info!("User selected Never update, changing mode to Ignore");
                        self.settings.general.update_mode = UpdateMode::Ignore;
                        self.updater_status = UpdateStatus::Idle;
                        Task::none()
                    }
                    UpdaterAction::RestartApp => {
                        updater::restart_app();
                        Task::none()
                    }
                }
            }
            Message::Notice(msg) => {
                if notice::update(&mut self.notice_popup, msg) {
                    self.notice_open = false;
                }
                self.sync_popup(ActivePopup::VersionNotice, self.notice_open);
                Task::none()
            }
            Message::AcknowledgeNotice => {
                let hash = notice::hash();
                if !self.app_state.notice.acknowledged.contains(&hash) {
                    info!("Notice {} acknowledged", hash);
                    self.app_state.notice.acknowledged.push(hash);
                }
                self.notice_open = false;
                self.sync_popup(ActivePopup::VersionNotice, false);
                Task::none()
            }
            Message::Home(msg) => {
                match msg {
                    home::Message::Navigate(page) => self.navigate(page),
                    home::Message::NavigateSettingsAddOns => Task::batch([
                        self.navigate(Page::Settings),
                        self.update(Message::Settings(gui_settings::Message::TabSelected(gui_settings::Tab::AddOns))),
                    ]),
                    home::Message::NavigateSettingsKeys => Task::batch([
                        self.navigate(Page::Settings),
                        self.update(Message::Settings(gui_settings::Message::TabSelected(gui_settings::Tab::Data))),
                        self.update(Message::Settings(gui_settings::Message::OpenKeysPopup)),
                    ]),
                    _ => self.home_state.update(msg).map(Message::Home),
                }
            }
            Message::Cat(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let task = self.cat_state.update(msg, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Cat);
                self.cat_state.sync_state(&mut self.app_state.cat_data);
                self.sync_popup(ActivePopup::CatExport, self.cat_state.export_popup_open(&self.app_state));
                self.sync_popup(ActivePopup::CatFilter, self.cat_state.filter_popup_open());
                task
            }
            Message::Enemy(enemy::Message::NavigateAppearances(id)) => Task::batch([
                self.navigate(Page::Stages),
                self.update(Message::Stage(stage::Message::ShowEnemyAppearances(id))),
            ]),
            Message::Enemy(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable };
                let enemies_loaded = matches!(msg, enemy::Message::Loaded(_));
                let task = self.enemy_state.update(msg, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Enemy);
                if enemies_loaded {
                    self.stage_state.sync_enemies(&self.enemy_state.data.enemies);
                }
                self.enemy_state.sync_state(&mut self.app_state.enemy_data);
                self.sync_popup(ActivePopup::EnemyFilter, self.enemy_state.filter_popup_open());
                self.sync_popup(ActivePopup::EnemyExport, self.enemy_state.export_popup_open(&self.app_state));
                task
            }
            Message::Stage(msg) => {
                let task = self.stage_state.update(msg).map(Message::Stage);
                self.sync_popup(ActivePopup::StageFilter, self.stage_state.filter_popup_open());
                task
            }
            Message::Mod(msg) => {
                let active_before = self.mods_state.active_mod();
                let task = self.mods_state.update(msg, &self.settings).map(Message::Mod);
                self.sync_popup(ActivePopup::ModsImport, self.mods_state.import_popup_open());
                self.sync_popup(ActivePopup::ModsExport, self.mods_state.export_popup_open());
                if self.mods_state.active_mod() != active_before {
                    return Task::batch([task, self.rescan_units()]);
                }
                task
            }
            Message::Data(msg) => self.data_state.update(msg, &mut self.settings, &mut self.app_state).map(Message::Data),
            Message::Settings(msg) => {
                if matches!(msg, gui_settings::Message::ManualUpdateCheck) {
                    info!("Manual update check requested from Settings");
                    return self.check_for_updates(true);
                }
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
            Page::Cats => self.cat_state.view(&self.settings, &self.app_state, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Cat),
            Page::Enemies => self.enemy_state.view(&self.settings, &self.app_state, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Enemy),
            Page::Stages => self.stage_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Stage),
            Page::Mods => self.mods_state.view().map(Message::Mod),
            Page::Data => self.data_state.view(&self.app_state).map(Message::Data),
            Page::Settings => self.settings_state.view(&self.settings).map(Message::Settings),
        };

        let content_container = container(content)
            .width(Length::Fill)
            .height(Length::Fill);

        let sidebar_overlay = self.view_sidebar_overlay();

        let expanded: Option<Element<'_, Message>> = match self.current_page {
            Page::Cats => self.cat_state.expanded_animation_view(&self.settings, &self.app_state).map(|view| view.map(Message::Cat)),
            Page::Enemies => self.enemy_state.expanded_animation_view(&self.settings, &self.app_state).map(|view| view.map(Message::Enemy)),
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
                ActivePopup::VersionNotice => Some(notice::view(&self.notice_popup, self.window_size)),
                ActivePopup::CatExport => {
                    if !matches!(self.current_page, Page::Cats) || !self.cat_state.export_popup_visible() {
                        return None;
                    }

                    self.cat_state.export_popup_view(self.window_size, &self.app_state).map(|view| view.map(Message::Cat))
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

                    self.enemy_state.export_popup_view(self.window_size, &self.app_state).map(|view| view.map(Message::Enemy))
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
        let toggle_btn = button(text(arrow_text).size(20).align_x(alignment::Horizontal::Center).align_y(alignment::Vertical::Center))
            .width(Length::Fixed(37.0))
            .height(Length::Fixed(37.0))
            .on_press(Message::ToggleSidebar)
            .style(theme::primary_button);

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
                let btn = button(text(page.tab_name()).size(16).width(Length::Fill).align_x(alignment::Horizontal::Center))
                    .width(Length::Fill)
                    .padding(10)
                    .on_press(Message::Navigate(*page))
                    .style(move |theme: &Theme, status| theme::toggle_button(theme, status, is_active));

                tabs = tabs.push(btn);
            }

            let sidebar_panel = container(tabs)
                .width(Length::Fixed(180.0))
                .height(Length::Fill)
                .padding(15)
                .style(theme::sidebar_container);

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
            if let Err(err) = json::save("settings.json", self) {
                warn!("Failed to save settings.json: {}", err);
                return;
            }
            self.last_saved_hash = current_hash;
        }
    }

    fn check_auto_save_state(&mut self) {
        let Ok(json_string) = serde_json::to_string(&self.app_state) else { return; };

        let mut hasher = FxHasher::default();
        json_string.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.last_saved_state_hash != current_hash {
            trace!("App state changed. Saving to state.json");
            if let Err(err) = json::save_state("state.json", &self.app_state) {
                warn!("Failed to save state.json: {}", err);
                return;
            }
            self.last_saved_state_hash = current_hash;
        }
    }
}