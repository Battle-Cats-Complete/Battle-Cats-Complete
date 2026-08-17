use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::{button, column, container, markdown, opaque, operation, row, scrollable, stack};
use iced::{task, window, Element, Length, Size, Subscription, Task, Theme};
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{info, trace, warn};

use core::common::context::GlobalContext;
use core::common::game::{localizable, param};
use core::common::io::json;
use core::modules::import::architecture;
use core::modules::settings::{Settings, UpdateMode};
use core::{ContentStore, Vault};

use crate::common::fonts;
use crate::common::watcher::{self, Asset, Change};
use crate::modules::{cat, enemy, files, home, import, mods, settings as gui_settings, stage};
use crate::widget::{nightly_label, popup, slide, Slide};

use state::AppState;

mod errors;
mod logging;
mod migrate;
mod notice;
pub(crate) mod startup;
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
    Files,
    Import,
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
            Self::Files => "Files",
            Self::Import => "Import",
            Self::Settings => "Settings",
        }
    }

    pub(crate) fn nightly(self) -> bool {
        false
    }
}

pub(crate) const WINDOW_SHOW_FALLBACK: Duration = Duration::from_millis(400);

const FRAMES_BEFORE_SHOW: u8 = 2;

const TAB_TEXT_SIZE: f32 = 16.0;

const ALL_PAGES: &[Page] = &[
    Page::Home,
    Page::Cats,
    Page::Enemies,
    Page::Stages,
    Page::Mods,
    Page::Files,
    Page::Import,
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
    InitErrors,
    Updater,
    VersionNotice,
    HomeChangelog,
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
    FramePainted,
    ShowWindow,
    Navigate(Page),
    CloseRequested(window::Id),
    ToggleSidebar,
    WindowResized(Size),
    Updater(UpdaterMsg),
    UpdaterAction(UpdaterAction),
    UpdaterPopup(popup::Message),
    UpdaterStatusExpired,
    Notice(popup::Message),
    AcknowledgeNotice,
    InitErrorPopup(popup::Message),
    AcknowledgeInitError,
    OpenUrl(String),
    VaultHydrated(Arc<Vault>),
    TablesLoaded(Box<(Param, Localizable)>),
    VaultValidated { vault: Option<Arc<Vault>>, key: Option<u64>, mounted: Option<String> },
    IndexPersisted,
    FilesChanged(Change),
    Home(home::Message),
    Cat(cat::Message),
    Enemy(enemy::Message),
    Stage(stage::Message),
    Mod(mods::Message),
    Files(files::Message),
    Import(import::Message),
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
    notice_items: Vec<markdown::Item>,
    #[serde(skip)]
    init_errors: errors::State,
    #[serde(skip)]
    rebuild_running: bool,
    #[serde(skip)]
    rebuild_queued: bool,
    #[serde(skip)]
    index_persisting: bool,
    #[serde(skip)]
    validated_key: Option<u64>,
    #[serde(skip)]
    frames_painted: u8,
    #[serde(skip)]
    window_shown: bool,
    #[serde(skip)]
    pub(crate) boot: Option<Instant>,

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
    pub files_state: files::State,
    #[serde(skip)]
    pub import_state: import::State,
    #[serde(skip)]
    pub settings_state: gui_settings::State,

    pub settings: Settings,

    #[serde(skip)]
    pub vault: Arc<Vault>,
    #[serde(skip)]
    pub vault_ready: bool,

    #[serde(skip)]
    pub app_state: AppState,

    #[serde(skip)]
    pub param: Param,
    #[serde(skip)]
    pub localizable: Localizable,
    #[serde(skip)]
    pub last_saved_hash: u64,
    #[serde(skip)]
    pub last_saved_state_hash: u64,

    #[serde(skip)]
    pub updater_handle: Option<task::Handle>,
    #[serde(skip)]
    pub updater_status: UpdateStatus,
    #[serde(skip)]
    pub updater_status_handle: Option<task::Handle>,
    #[serde(skip)]
    updater_popup: popup::State,
    #[serde(skip)]
    updater_popup_open: bool,
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
            notice_items: notice::parse_content(),
            init_errors: errors::State::default(),
            rebuild_running: false,
            rebuild_queued: false,
            index_persisting: false,
            validated_key: None,
            frames_painted: 0,
            window_shown: false,
            boot: None,
            home_state: home::State::default(),
            cat_state: cat::State::default(),
            enemy_state: enemy::EnemyState::default(),
            stage_state: stage::State::default(),
            mods_state: mods::State::new(core::modules::mods::ModDataState::default()),
            files_state: files::State::default(),
            import_state: import::State::default(),
            settings_state: gui_settings::State::default(),
            settings: Settings::default(),
            vault: Arc::new(Vault::new(&Settings::default())),
            vault_ready: false,
            app_state: AppState::default(),
            param: Param::default(),
            localizable: Localizable::default(),
            last_saved_hash: 0,
            last_saved_state_hash: 0,
            updater_handle: None,
            updater_status: UpdateStatus::Idle,
            updater_status_handle: None,
            updater_popup: popup::State::default(),
            updater_popup_open: false,
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
            self.import_state.subscription().map(Message::Import),
            Subscription::run(watcher::changes).map(Message::FilesChanged),
            window::close_requests().map(Message::CloseRequested),
        ];

        if self.updater_popup_open && matches!(self.updater_status, UpdateStatus::Downloading(_)) {
            subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::DownloadTick));
        }

        if !self.window_shown {
            subs.push(window::frames().map(|_| Message::FramePainted));
        }

        Subscription::batch(subs)
    }

    fn show_window(&mut self) -> Task<Message> {
        self.window_shown = true;

        if let Some(boot) = self.boot {
            info!(ms = boot.elapsed().as_millis(), "Window revealed");
        }

        window::latest().and_then(|id| window::set_mode(id, window::Mode::Windowed))
    }

    fn navigate(&mut self, page: Page) -> Task<Message> {
        if self.current_page == Page::Files && page != Page::Files {
            self.files_state.leave(&self.vault.vfs);
        }

        self.current_page = page;

        match page {
            Page::Home => {
                self.sync_home_status();
                Task::none()
            }
            Page::Files => {
                let task = self.files_state.sync(&self.vault.vfs, &self.settings.files, false);
                self.files_state.sync_state(&mut self.app_state.files);

                task.map(Message::Files)
            }
            Page::Cats => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault };
                let sheets_task = self.cat_state.update(cat::Message::SheetsCheck, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Cat);
                let scroll_task = operation::scroll_to(
                    cat::State::list_scrollable_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: self.cat_state.list_scroll_offset() },
                );
                Task::batch([sheets_task, scroll_task])
            }
            Page::Enemies => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault };
                let sheets_task = self.enemy_state.update(enemy::Message::SheetsCheck, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Enemy);
                let scroll_task = operation::scroll_to(
                    enemy::EnemyState::list_scrollable_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: self.enemy_state.list_scroll_offset() },
                );
                Task::batch([sheets_task, scroll_task])
            }
            Page::Stages => {
                self.stage_state.enter();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn sync_home_status(&mut self) {
        if !self.vault_ready || self.rebuild_running {
            return;
        }

        self.home_state.set_game_empty(self.vault.vfs.count(architecture::GAME) == 0);
    }

    fn apply_changes(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        if !self.vault_ready {
            return Task::none();
        }

        let mut units = HashSet::new();
        let mut enemies = HashSet::new();
        let mut items = HashSet::new();
        let mut stage_coarse = false;
        let mut touched_mods = HashSet::new();
        let mut pruned = false;

        for path in &paths {
            let Some(mount) = watcher::mount_of(path) else { continue; };
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue; };

            if mount != architecture::GAME {
                self.vault.evict(name);
                touched_mods.insert(mount);
            } else if path.is_file() {
                if let Err(err) = self.vault.vfs.create((mount.as_str(), path.as_path())) {
                    warn!(path = %path.display(), "Failed to index a changed file: {}", err);
                }

                self.vault.evict(name);
            } else if !path.exists() {
                let dropped = self.vault.vfs.prune(mount.as_str(), path.as_path());

                trace!(path = %path.display(), files = dropped.len(), "Dropped deleted paths from the index");
                self.vault.purge(&dropped);
                self.vault.vds.evict(name);

                pruned |= !dropped.is_empty();
            }

            let is_image = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("png"));

            match watcher::asset(name) {
                Some(Asset::Unit(id)) => { units.insert(id); }
                Some(Asset::Enemy(id)) => { enemies.insert(id); }
                Some(Asset::Item(id)) => { items.insert(id); }
                None if is_image => stage_coarse = true,
                None => {}
            }

        }

        let wiped = !architecture::game_present() && self.vault.vfs.count(architecture::GAME) > 0;

        if wiped {
            let dropped = self.vault.vfs.prune(architecture::GAME, Path::new(architecture::GAME));
            warn!(files = dropped.len(), "The game folder is gone; dropped every indexed file");
            self.vault.purge(&dropped);
            pruned = true;
        }

        if pruned {
            self.sync_home_status();
        }

        if !touched_mods.is_empty() {
            self.mods_state.resync(&self.vault, &touched_mods);
            self.mods_state.invalidate_assets(&touched_mods);
        }

        let files_task = if self.current_page == Page::Files {
            let task = self.files_state.apply_changes(&self.vault.vfs, &paths, &self.settings.files);
            self.files_state.sync_state(&mut self.app_state.files);

            task.map(Message::Files)
        } else {
            Task::none()
        };

        if wiped {
            return self.rebuild_content();
        }

        self.cat_state.invalidate_assets(&units, &items);
        self.enemy_state.invalidate_assets(&enemies);
        self.stage_state.invalidate_assets(&items, &enemies, stage_coarse);

        self.cat_state.reload_selected(&self.vault, &self.settings.scanner_config(None));
        self.enemy_state.reload_selected(&self.vault, self.settings.show_invalid_enemies());
        self.stage_state.reload_selected(&self.vault);

        Task::batch([files_task, self.persist_index()])
    }

    fn finish_validation(&mut self, vault: Option<Arc<Vault>>, key: Option<u64>, mounted: Option<String>) -> Task<Message> {
        self.rebuild_running = false;
        let queued = std::mem::take(&mut self.rebuild_queued);

        if mounted != self.mods_state.intended_mod() {
            warn!(?mounted, "The active mod changed while indexing, discarding the stale index");
            return self.rebuild_content();
        }

        if let Some(rebuilt) = vault {
            info!("Swapping in the rebuilt file index");
            self.cat_state.clear_caches();
            self.enemy_state.clear_caches();
            self.stage_state.clear_caches();
            self.vault.vds.clear();

            let adopted = self.adopt_vault(rebuilt, false);

            if !queued {
                return adopted;
            }

            info!("More changes landed mid-validation, indexing again");
            return Task::batch([adopted, self.rebuild_content()]);
        }

        if queued {
            info!("More changes landed mid-validation, indexing again");
            return self.rebuild_content();
        }

        let Some(key) = key else {
            self.clear_indexing();
            return Task::none();
        };

        self.validated_key = Some(key);
        self.reconcile_caches()
    }

    pub(crate) fn reconcile_caches(&mut self) -> Task<Message> {
        let Some(key) = self.validated_key else {
            return Task::none();
        };

        let cached = [self.cat_state.cached_key(), self.enemy_state.cached_key(), self.stage_state.cached_key()];

        if cached.iter().any(Option::is_none) {
            return Task::none();
        }

        self.validated_key = None;

        if cached.iter().all(|entry| *entry == Some(key)) {
            self.clear_indexing();
            return Task::none();
        }

        info!(key, "Scanner caches drifted from the current settings, rescanning");
        self.vault.vds.clear();
        self.rescan_units()
    }

    fn clear_indexing(&mut self) {
        self.cat_state.clear_indexing();
        self.enemy_state.clear_indexing();
        self.stage_state.clear_indexing();
    }

    pub(crate) fn rebuild_content(&mut self) -> Task<Message> {
        if self.rebuild_running || !self.vault_ready || self.mods_state.mounting() {
            self.rebuild_queued = true;
            return Task::none();
        }

        info!("Rebuilding the file index and rescanning every module");
        self.rebuild_queued = false;

        self.spawn_vault_build(false)
    }

    fn persist_index(&mut self) -> Task<Message> {
        if self.index_persisting || self.rebuild_running || self.rebuild_queued {
            return Task::none();
        }

        if !architecture::game_present() {
            return Task::none();
        }

        self.index_persisting = true;
        let vault = Arc::clone(&self.vault);
        let active_mod = self.mods_state.active_mod();

        Task::perform(
            smol::unblock(move || {
                let hash = Vault::hash(active_mod.as_deref());
                vault.vfs.persist(hash);
                trace!(hash, "Re-persisted the file index after a live change");
            }),
            |()| Message::IndexPersisted,
        )
    }

    fn persist_content(&self) {
        let vault = Arc::clone(&self.vault);
        let config = self.settings.scanner_config(self.mods_state.active_mod());

        thread::spawn(move || {
            ContentStore::capture(&vault.vds).save(Vault::key(&config));
        });
    }

    pub(super) fn load_tables(&self) -> Task<Message> {
        let vault = Arc::clone(&self.vault);
        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            let tables = (param(&vault.vfs).unwrap_or_default(), localizable(&vault.vfs));
            let _ = tx.unbounded_send(Message::TablesLoaded(Box::new(tables)));
        });

        Task::stream(rx)
    }

    fn relocalize(&mut self) -> Task<Message> {
        info!(priority = ?self.settings.general.language_priority, "Language priority settled, dropping every resolved file");
        self.vault.priority(&self.settings.general.language_priority);
        self.rescan_units()
    }

    fn rescan_units(&mut self) -> Task<Message> {
        let active_mod = self.mods_state.active_mod();

        Task::batch([
            self.load_tables(),
            self.cat_state.rescan(&self.settings, &self.vault, active_mod.clone()).map(Message::Cat),
            self.enemy_state.rescan(&self.settings, &self.vault, active_mod.clone()).map(Message::Enemy),
            self.stage_state.rescan(&self.settings, &self.vault, active_mod).map(Message::Stage),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(size) => {
                self.window_size = size;
                self.settings.window.width = size.width;
                self.settings.window.height = size.height;
                Task::none()
            }
            Message::AutoSave => {
                self.check_auto_save();
                self.check_auto_save_state();
                Task::none()
            }
            Message::FramePainted => {
                if self.window_shown {
                    return Task::none();
                }

                self.frames_painted = self.frames_painted.saturating_add(1);
                if self.frames_painted < FRAMES_BEFORE_SHOW {
                    return Task::none();
                }

                self.show_window()
            }
            Message::ShowWindow => {
                if self.window_shown {
                    return Task::none();
                }

                warn!("First frame never reported; revealing the window on the startup fallback");
                self.show_window()
            }
            Message::DownloadTick => {
                self.download_progress += 0.01;
                if self.download_progress > 1.0 {
                    self.download_progress = 0.0;
                }
                Task::none()
            }
            Message::Updater(msg) => {
                let mut task = Task::none();
                match msg {
                    UpdaterMsg::UpdateFound(release) => {
                        self.updater_status = UpdateStatus::UpdateFound(release.version.clone(), release);
                        self.set_updater_popup(true);
                    }
                    UpdaterMsg::UpToDate => {
                        self.updater_status = UpdateStatus::UpToDate;
                        task = self.schedule_updater_status_expiry();
                    }
                    UpdaterMsg::CheckFailed => {
                        self.updater_status = UpdateStatus::CheckFailed;
                        task = self.schedule_updater_status_expiry();
                    }
                    UpdaterMsg::DownloadStarted(version) => {
                        self.updater_status = UpdateStatus::Downloading(version);
                        self.download_progress = 0.0;
                        self.set_updater_popup(true);
                    }
                    UpdaterMsg::DownloadFinished(version) => {
                        self.updater_status = UpdateStatus::RestartPending(version);
                        self.set_updater_popup(true);
                    }
                    UpdaterMsg::SilentFail => {
                        self.updater_status = UpdateStatus::Idle;
                        self.set_updater_popup(false);
                    }
                }
                task
            }
            Message::UpdaterPopup(msg) => {
                if updater::update_popup(&mut self.updater_popup, msg) {
                    self.dismiss_updater_popup();
                }
                Task::none()
            }
            Message::UpdaterStatusExpired => {
                if matches!(self.updater_status, UpdateStatus::UpToDate | UpdateStatus::CheckFailed) {
                    self.updater_status = UpdateStatus::Idle;
                }
                self.updater_status_handle = None;
                Task::none()
            }
            Message::CloseRequested(id) => {
                self.files_state.leave(&self.vault.vfs);
                self.check_auto_save();
                self.check_auto_save_state();

                window::close(id)
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
                        self.dismiss_updater_popup();
                        Task::none()
                    }
                    UpdaterAction::NeverUpdate => {
                        info!("User selected Never update, changing mode to Ignore");
                        self.settings.general.update_mode = UpdateMode::Ignore;
                        self.updater_status = UpdateStatus::Idle;
                        self.set_updater_popup(false);
                        Task::none()
                    }
                    UpdaterAction::RestartApp => {
                        updater::restart_app();
                        Task::none()
                    }
                }
            }
            Message::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    warn!("Failed to open URL {}: {}", url, e);
                }
                Task::none()
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
            Message::InitErrorPopup(msg) => {
                self.init_errors.update(msg);
                self.sync_popup(ActivePopup::InitErrors, self.init_errors.is_open());
                Task::none()
            }
            Message::AcknowledgeInitError => {
                self.init_errors.acknowledge();
                self.sync_popup(ActivePopup::InitErrors, self.init_errors.is_open());
                Task::none()
            }
            Message::VaultHydrated(vault) => self.adopt_vault(vault, true),
            Message::TablesLoaded(tables) => {
                let (param, localizable) = *tables;
                info!("Core tables loaded");
                self.param = param;
                self.localizable = localizable;
                Task::none()
            }
            Message::VaultValidated { vault, key, mounted } => self.finish_validation(vault, key, mounted),
            Message::FilesChanged(Change::Unavailable) => {
                warn!("File watcher could not be initialized; reporting it as an initialization error");
                self.init_errors.report_watcher_failure(self.settings.general.ignore_watcher_failure);
                self.sync_popup(ActivePopup::InitErrors, self.init_errors.is_open());
                Task::none()
            }
            Message::FilesChanged(Change::Bulk) => {
                info!("File watcher reported a bulk change, re-indexing everything");
                self.rebuild_content()
            }
            Message::FilesChanged(Change::Batch(paths)) => self.apply_changes(paths),
            Message::IndexPersisted => {
                self.index_persisting = false;
                Task::none()
            }
            Message::Home(msg) => {
                let task = match msg {
                    home::Message::Navigate(page) => self.navigate(page),
                    home::Message::NavigateSettingsAddOns => Task::batch([
                        self.navigate(Page::Settings),
                        self.update(Message::Settings(gui_settings::Message::TabSelected(gui_settings::Tab::AddOns))),
                    ]),
                    home::Message::NavigateSettingsKeys => Task::batch([
                        self.navigate(Page::Settings),
                        self.update(Message::Settings(gui_settings::Message::TabSelected(gui_settings::Tab::General))),
                        self.update(Message::Settings(gui_settings::Message::OpenKeysPopup)),
                    ]),
                    _ => self.home_state.update(msg).map(Message::Home),
                };
                self.sync_popup(ActivePopup::HomeChangelog, self.home_state.changelog_popup_open());
                task
            }
            Message::Cat(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault };
                let loaded = matches!(msg, cat::Message::Loaded(..));
                let task = self.cat_state.update(msg, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Cat);
                self.cat_state.sync_state(&mut self.app_state.cat);
                self.sync_popup(ActivePopup::CatExport, self.cat_state.export_popup_open(&self.app_state));
                self.sync_popup(ActivePopup::CatFilter, self.cat_state.filter_popup_open());

                if loaded {
                    return Task::batch([task, self.reconcile_caches()]);
                }

                task
            }
            Message::Enemy(enemy::Message::NavigateAppearances(id)) => Task::batch([
                self.navigate(Page::Stages),
                self.update(Message::Stage(stage::Message::ShowEnemyAppearances(id))),
            ]),
            Message::Enemy(msg) => {
                let global_ctx = GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault };
                let enemies_loaded = matches!(msg, enemy::Message::Loaded(..));
                let task = self.enemy_state.update(msg, &mut self.settings, &mut self.app_state, global_ctx).map(Message::Enemy);
                if enemies_loaded {
                    self.stage_state.sync_enemies(&self.enemy_state.data.enemies);
                }
                self.enemy_state.sync_state(&mut self.app_state.enemy);
                self.sync_popup(ActivePopup::EnemyFilter, self.enemy_state.filter_popup_open());
                self.sync_popup(ActivePopup::EnemyExport, self.enemy_state.export_popup_open(&self.app_state));

                if enemies_loaded {
                    return Task::batch([task, self.reconcile_caches()]);
                }

                task
            }
            Message::Stage(msg) => {
                let stages_loaded = matches!(msg, stage::Message::Loaded(..));
                let task = self.stage_state.update(msg).map(Message::Stage);
                if stages_loaded {
                    self.persist_content();
                }
                self.stage_state.sync_state(&mut self.app_state.stage);
                self.sync_popup(ActivePopup::StageFilter, self.stage_state.filter_popup_open());

                if stages_loaded {
                    return Task::batch([task, self.reconcile_caches()]);
                }

                task
            }
            Message::Mod(msg) => {
                let mount_settled = matches!(msg, mods::Message::MountFinished { .. });
                let task = self.mods_state.update(msg, &self.settings, &self.vault).map(Message::Mod);
                self.mods_state.sync_state(&mut self.app_state.mods);
                self.sync_popup(ActivePopup::ModsImport, self.mods_state.import_popup_open());
                self.sync_popup(ActivePopup::ModsExport, self.mods_state.export_popup_open());

                if !mount_settled {
                    return task;
                }

                if self.rebuild_queued {
                    info!("Changes landed while mounting, indexing again");
                    return Task::batch([task, self.rebuild_content()]);
                }

                Task::batch([task, self.rescan_units()])
            }
            Message::Import(msg) => {
                let task = self.import_state.update(msg, &mut self.settings, &mut self.app_state).map(Message::Import);

                if !self.import_state.take_import_success() {
                    return task;
                }

                Task::batch([task, self.rebuild_content()])
            }
            Message::Files(msg) => {
                let task = self.files_state.update(msg, &self.vault.vfs, &self.settings.files).map(Message::Files);
                self.files_state.sync_state(&mut self.app_state.files);
                task
            }
            Message::Settings(msg) => {
                if matches!(msg, gui_settings::Message::General(gui_settings::general::Message::ManualUpdateCheck)) {
                    info!("Manual update check requested from Settings");
                    return self.check_for_updates(true);
                }
                if matches!(msg, gui_settings::Message::General(gui_settings::general::Message::ShowUpdatePopup)) {
                    self.set_updater_popup(true);
                    return Task::none();
                }
                let task = self.settings_state.update(msg, &mut self.settings).map(Message::Settings);
                let relocalize = self.settings_state.take_language_change().then(|| self.relocalize());
                let left_nightly = (self.current_page.nightly() && !self.settings.general.enable_nightly)
                    .then(|| self.navigate(Page::Home));
                self.sync_popup(ActivePopup::SettingsKeys, self.settings_state.keys_popup_open());
                self.sync_popup(ActivePopup::SettingsExceptions, self.settings_state.exceptions_popup_open());
                self.sync_popup(ActivePopup::SettingsPem, self.settings_state.pem_popup_open());

                Task::batch(iter::once(task).chain(relocalize).chain(left_nightly))
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.current_page {
            Page::Home => self.home_state.view(self.settings.general.enable_nightly).map(Message::Home),
            Page::Cats => self.cat_state.view(&self.settings, &self.app_state, GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault }).map(Message::Cat),
            Page::Enemies => self.enemy_state.view(&self.settings, &self.app_state, GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault }).map(Message::Enemy),
            Page::Stages => self.stage_state.view(&self.settings, GlobalContext { param: &self.param, localizable: &self.localizable, vault: &self.vault }).map(Message::Stage),
            Page::Mods => self.mods_state.view().map(Message::Mod),
            Page::Files => self.files_state.view().map(Message::Files),
            Page::Import => self.import_state.view(&self.app_state).map(Message::Import),
            Page::Settings => self.settings_state.view(&self.settings, &self.updater_status).map(Message::Settings),
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

    fn set_updater_popup(&mut self, open: bool) {
        self.updater_popup_open = open;
        self.sync_popup(ActivePopup::Updater, open);
    }

    fn dismiss_updater_popup(&mut self) {
        if !matches!(self.updater_status, UpdateStatus::Downloading(_)) {
            self.updater_status = UpdateStatus::Idle;
        }

        self.set_updater_popup(false);
    }

    fn build_popups(&self) -> Vec<Element<'_, Message>> {
        self.active_popups
            .iter()
            .filter_map(|popup| match popup {
                ActivePopup::InitErrors => self.init_errors.view(self.window_size),
                ActivePopup::Updater => updater::view(&self.updater_popup, &self.updater_status, self.window_size, self.download_progress),
                ActivePopup::VersionNotice => Some(notice::view(&self.notice_popup, self.window_size, self.theme(), &self.notice_items)),
                ActivePopup::HomeChangelog => {
                    if !matches!(self.current_page, Page::Home) {
                        return None;
                    }

                    self.home_state.changelog_popup_view(self.window_size, self.theme()).map(|view| view.map(Message::Home))
                }
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
        let toggle_btn = button(
            theme::centered_text(arrow_text)
                .font(fonts::MISC_SYMBOLS)
                .size(20)
                .width(Length::Fill)
                .height(Length::Fill),
        )
            .width(Length::Fixed(theme::NAV_TOGGLE_SIZE))
            .height(Length::Fixed(theme::NAV_TOGGLE_SIZE))
            .padding(0)
            .on_press(Message::ToggleSidebar)
            .style(theme::primary_button);

        let toggle_container = column![toggle_btn]
            .padding(iced::Padding {
                top: theme::NAV_TOGGLE_TOP,
                right: theme::NAV_TOGGLE_RIGHT,
                bottom: 0.0,
                left: 0.0,
            });

        let mut tabs: iced::widget::Column<'_, Message> = column![].spacing(10);
        for page in ALL_PAGES {
            if page.nightly() && !self.settings.general.enable_nightly {
                continue;
            }

            let is_active = self.current_page == *page;
            let label: Element<'_, Message> = if page.nightly() {
                nightly_label(page.tab_name(), TAB_TEXT_SIZE)
            } else {
                theme::button_label(page.tab_name()).size(TAB_TEXT_SIZE).into()
            };
            let btn = button(label)
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

        let layer = row![toggle_container, opaque(slide(sidebar_panel, self.sidebar_open, Slide::Right))]
            .height(Length::Fill);

        container(layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_right(Length::Fill)
            .into()
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
        self.app_state.popups = popup::snapshot();

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