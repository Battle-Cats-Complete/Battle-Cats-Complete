use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};

use iced::alignment;
use iced::widget::{button, column, container, progress_bar, row, scrollable, stack, text, Space};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{info, trace, warn};

use core::common::context::GlobalContext;
use core::modules::settings::{Settings, UpdateMode};

use crate::common::watcher::GuiWatcher;
use crate::modules::{animation, cat, data, enemy, home, mods, settings as gui_settings, stage};

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize, Debug)]
pub enum Page {
    Home,
    Cats,
    Enemies,
    Stages,
    Mods,
    Data,
    Animation,
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
            Self::Animation => "Animation",
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
    Page::Animation,
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

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    Navigate(Page),
    ToggleSidebar,
    UpdaterAction(UpdaterAction),
    Home(home::Message),
    Cat(cat::Message),
    Enemy(enemy::Message),
    Stage(stage::Message),
    Mod(mods::Message),
    Data(data::Message),
    Animation(animation::Message),
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
    pub animation_state: animation::State,
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
            home_state: home::State::default(),
            cat_state: cat::State::default(),
            enemy_state: enemy::EnemyState::default(),
            stage_state: stage::State::default(),
            mods_state: mods::State::new(core::modules::mods::ModDataState::default()),
            data_state: data::State::default(),
            animation_state: animation::State::default(),
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
        let (home_state, home_task) = home::State::new();
        app.home_state = home_state;

        (app, home_task.map(Message::Home))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
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
                    UpdaterAction::StartDownload(_release) => {
                        info!("Triggering StartDownload");
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
                        info!("Triggering RestartApp");
                        std::process::exit(0);
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
            Message::Cat(msg) => self.cat_state.update(msg, &self.settings).map(Message::Cat),
            Message::Enemy(msg) => self.enemy_state.update(msg, &self.settings).map(Message::Enemy),
            Message::Stage(msg) => self.stage_state.update(msg, &self.settings).map(Message::Stage),
            Message::Mod(msg) => self.mods_state.update(msg, &self.settings).map(Message::Mod),
            Message::Data(msg) => self.data_state.update(msg, &mut self.settings).map(Message::Data),
            Message::Animation(msg) => self.animation_state.update(msg).map(Message::Animation),
            Message::Settings(msg) => self.settings_state.update(msg, &mut self.settings).map(Message::Settings),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_page {
            Page::Home => self.home_state.view().map(Message::Home),
            Page::Cats => self.cat_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Cat),
            Page::Enemies => self.enemy_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable }).map(Message::Enemy),
            Page::Stages => self.stage_state.view().map(Message::Stage),
            Page::Mods => self.mods_state.view().map(Message::Mod),
            Page::Data => self.data_state.view(&self.settings).map(Message::Data),
            Page::Animation => self.animation_state.view().map(Message::Animation),
            Page::Settings => self.settings_state.view(&self.settings).map(Message::Settings),
        };

        let content_container = container(content)
            .width(Length::Fill)
            .height(Length::Fill);
        
        let sidebar_list: Element<Message> = if self.sidebar_open {
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

            container(tabs)
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
                })
                .into()
        } else {
            Space::new().width(Length::Fixed(0.0)).into()
        };
        
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
        
        let right_panel = row![
            toggle_container,
            sidebar_list
        ]
            .height(Length::Fill);
        
        let base_ui = row![
            content_container,
            right_panel
        ]
            .width(Length::Fill)
            .height(Length::Fill);
        
        if let Some(modal) = self.build_modal() {
            stack![base_ui, modal].into()
        } else {
            base_ui.into()
        }
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