use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{info, trace, warn};

use core::common::io::json;
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
            Message::Enemy(msg) => self.enemy_state.update(msg).map(Message::Enemy),
            Message::Stage(msg) => self.stage_state.update(msg).map(Message::Stage),
            Message::Mod(msg) => self.mods_state.update(msg, &self.settings).map(Message::Mod),
            Message::Data(msg) => self.data_state.update(msg, &mut self.settings).map(Message::Data),
            Message::Animation(msg) => self.animation_state.update(msg).map(Message::Animation),
            Message::Settings(msg) => self.settings_state.update(msg, &mut self.settings).map(Message::Settings),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = if self.sidebar_open {
            self.view_sidebar()
        } else {
            Space::new().width(Length::Fixed(0.0)).into()
        };

        let content = match self.current_page {
            Page::Home => self.home_state.view().map(Message::Home),
            Page::Cats => self.cat_state.view().map(Message::Cat),
            Page::Enemies => self.enemy_state.view().map(Message::Enemy),
            Page::Stages => self.stage_state.view().map(Message::Stage),
            Page::Mods => self.mods_state.view().map(Message::Mod),
            Page::Data => self.data_state.view(&self.settings).map(Message::Data),
            Page::Animation => self.animation_state.view().map(Message::Animation),
            Page::Settings => self.settings_state.view(&self.settings).map(Message::Settings),
        };

        row![
            sidebar,
            container(content).width(Length::Fill).height(Length::Fill)
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_sidebar(&self) -> Element<Message> {
        let mut nav_col = column![].spacing(8).width(Length::Fill);

        for page in ALL_PAGES {
            let is_active = self.current_page == *page;

            let btn = button(text(page.tab_name()).size(16).align_x(Alignment::Center))
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

            nav_col = nav_col.push(btn);
        }

        let toggle_btn = button(text("<<").align_x(Alignment::Center))
            .width(Length::Fill)
            .padding(10)
            .on_press(Message::ToggleSidebar)
            .style(button::secondary);

        container(
            column![
                nav_col,
                Space::new().height(Length::Fill),
                toggle_btn
            ]
                .spacing(10)
        )
            .width(Length::Fixed(180.0))
            .height(Length::Fill)
            .padding(10)
            .style(container::bordered_box)
            .into()
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