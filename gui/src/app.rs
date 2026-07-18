pub(crate) mod events;
pub(crate) mod canvas;
pub(crate) mod logging;
pub(crate) mod reload;
pub(crate) mod startup;
pub(crate) mod updater;

use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;
use self_update::update::Release;
use tracing::{info, trace, warn};

use core::common::io::json;
use core::modules::settings::Settings;

use crate::common::DragGuard;
use crate::common::GuiWatcher;
use crate::modules::cat::state::CatListState;
use crate::modules::data::state::ImportState;
use crate::modules::enemy::state::EnemyListState;
use crate::modules::mods::state::ModListState;
use crate::modules::stage::state::StageListState;

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub(crate) enum Page {
    Home,
    Cats,
    Enemies,
    Stages,
    Mods,
    Data,
    Settings,
}

impl Page {
    pub(crate) fn tab_name(self) -> &'static str {
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

pub(crate) const ALL_PAGES: &[Page] = &[
    Page::Home,
    Page::Cats,
    Page::Enemies,
    Page::Stages,
    Page::Mods,
    Page::Data,
    Page::Settings,
];

#[derive(Clone)]
pub(crate) enum UpdateStatus {
    Idle,
    Checking,
    UpdateFound(String, Release),
    Downloading(String),
    RestartPending(String),
    CheckFailed,
    UpToDate,
}

pub(super) enum UpdaterMsg {
    UpdateFound(Release),
    UpToDate,
    CheckFailed,
    DownloadStarted(String),
    DownloadFinished(String),
    SilentFail,
}

pub(crate) struct Updater {
    rx: Receiver<UpdaterMsg>,
    tx: Sender<UpdaterMsg>,
    pub status: UpdateStatus,
    pub clear_time: Option<f64>,
}

impl Default for Updater {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            rx,
            tx,
            status: UpdateStatus::Idle,
            clear_time: None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct BattleCatsApp {
    #[serde(skip)] pub(crate) current_page: Page,
    #[serde(skip)] pub(crate) sidebar_open: bool,
    #[serde(skip)] pub(crate) import_state: ImportState,
    #[serde(skip)] pub(crate) updater: Updater,
    #[serde(skip)] pub(crate) drag_guard: DragGuard,
    #[serde(skip)] pub(crate) global_watcher: Option<GuiWatcher>,
    #[serde(skip)] pub param: Param,
    #[serde(skip)] pub localizable: Localizable,

    #[serde(skip)] pub hash_rx: Option<Receiver<bool>>,
    #[serde(skip)] pub last_saved_hash: u64,

    pub(crate) cat_list_state: CatListState,
    pub(crate) enemy_list_state: EnemyListState,
    pub(crate) stage_list_state: StageListState,
    pub(crate) mod_state: ModListState,
    pub settings: Settings,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::Home,
            sidebar_open: false,
            import_state: ImportState::default(),
            cat_list_state: CatListState::default(),
            enemy_list_state: EnemyListState::default(),
            stage_list_state: StageListState::default(),
            mod_state: ModListState::default(),
            settings: Settings::default(),
            updater: Updater::default(),
            drag_guard: DragGuard::default(),
            global_watcher: None,
            hash_rx: None,
            last_saved_hash: 0,
            param: Param::default(),
            localizable: Localizable::default(),
        }
    }
}

impl eframe::App for BattleCatsApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        let Ok(json_string) = serde_json::to_string(self) else { return; };

        let mut hasher = FxHasher::default();
        json_string.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.last_saved_hash != current_hash {
            trace!("Settings changed. Saving to settings.json");
            json::save("settings.json", self);
            self.last_saved_hash = current_hash;
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.hash_rx {
            if let Ok(is_valid) = rx.try_recv() {
                self.hash_rx = None;
                if !is_valid {
                    warn!("Cache hash validation failed! Performing full data reload.");
                    self.perform_full_data_reload();
                    ctx.request_repaint();
                } else {
                    info!("Cache hash validation passed.");
                    self.cat_list_state.cat_list.force_search_rebuild();
                    self.enemy_list_state.enemy_list.force_search_rebuild();
                }
            }
        }

        self.updater.update_state(ctx);

        let status_string = match self.updater.status {
            UpdateStatus::Checking => "Checking",
            UpdateStatus::UpToDate => "UpToDate",
            UpdateStatus::UpdateFound(..) => "UpdateFound",
            UpdateStatus::CheckFailed => "CheckFailed",
            UpdateStatus::Downloading(_) => "Downloading",
            UpdateStatus::RestartPending(_) => "RestartPending",
            UpdateStatus::Idle => "Idle",
        };
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("updater_status"), status_string));

        if self.settings.runtime.manual_check_requested {
            info!("Manual update check requested by user");
            self.settings.runtime.manual_check_requested = false;
            self.updater.check_for_updates(ctx.clone(), true);
        }

        self.updater.show_ui(ctx, &mut self.settings, &mut self.drag_guard);

        self.process_file_events(ctx);
        self.process_ui_events(ctx);

        self.cat_list_state.data.update_data();
        self.enemy_list_state.data.update_data();
        self.stage_list_state.update_data();

        self.stage_list_state.sync_enemies(&self.enemy_list_state.data.enemies);

        let needs_repaint = self.cat_list_state.data.scan_receiver.is_some()
            || self.enemy_list_state.data.scan_receiver.is_some()
            || self.stage_list_state.data.scan_receiver.is_some();

        if needs_repaint {
            ctx.request_repaint();
        }

        let import_finished = self.import_state.update(ctx);
        if import_finished {
            info!("Import job finished, performing full data reload");
            self.perform_full_data_reload();
            ctx.request_repaint();
        }

        canvas::draw(self, ctx);
    }
}