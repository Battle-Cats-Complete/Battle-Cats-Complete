use std::hash::{Hash, Hasher};

use eframe::egui;
use nyanko::common::data::{Localizable, Param};
use rustc_hash::FxHasher;

use core::global::io::json;
use core::settings::logic::state::Settings;

use crate::modules::cat::state::CatListState;
use crate::modules::data::state::ImportState;
use crate::modules::enemy::state::EnemyListState;
use crate::modules::mods::state::ModListState;
use crate::modules::stage::state::StageListState;
use crate::global::shared::DragGuard;
use crate::global::watcher::GuiWatcher;
use updater::Updater;

pub mod startup;
pub mod frame;
pub mod reload;
pub mod events;
pub mod tracing;
mod updater;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BattleCatsApp {
    #[serde(skip)] pub(crate) current_page: frame::Page,
    #[serde(skip)] pub(crate) sidebar_open: bool,
    #[serde(skip)] pub(crate) import_state: ImportState,
    #[serde(skip)] pub(crate) updater: Updater,
    #[serde(skip)] pub(crate) drag_guard: DragGuard,
    #[serde(skip)] pub(crate) global_watcher: Option<GuiWatcher>,
    #[serde(skip)] pub param: Param,
    #[serde(skip)] pub localizable: Localizable,

    #[serde(skip)] pub hash_rx: Option<std::sync::mpsc::Receiver<bool>>,
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
            current_page: frame::Page::Home,
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
        if let Ok(json_string) = serde_json::to_string(self) {
            let mut hasher = FxHasher::default();
            json_string.hash(&mut hasher);
            let current_hash = hasher.finish();
            if self.last_saved_hash != current_hash {
                ::tracing::trace!("Settings changed. Saving to settings.json");
                json::save("settings.json", self);
                self.last_saved_hash = current_hash;
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.hash_rx
            && let Ok(is_valid) = rx.try_recv() {
            self.hash_rx = None;
            if !is_valid {
                ::tracing::warn!("Cache hash validation failed! Performing full data reload.");
                self.perform_full_data_reload();
                ctx.request_repaint();
            } else {
                ::tracing::info!("Cache hash validation passed.");
                self.cat_list_state.cat_list.force_search_rebuild();
                self.enemy_list_state.enemy_list.force_search_rebuild();
            }
        }

        self.updater.update_state(ctx);

        let status_str = match self.updater.status {
            updater::UpdateStatus::Checking => "Checking",
            updater::UpdateStatus::UpToDate => "UpToDate",
            updater::UpdateStatus::UpdateFound(..) => "UpdateFound",
            updater::UpdateStatus::CheckFailed => "CheckFailed",
            updater::UpdateStatus::Downloading(_) => "Downloading",
            updater::UpdateStatus::RestartPending(_) => "RestartPending",
            updater::UpdateStatus::Idle => "Idle",
        };
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("updater_status"), status_str));

        if self.settings.runtime.manual_check_requested {
            ::tracing::info!("Manual update check requested by user");
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

        if self.cat_list_state.data.scan_receiver.is_some() || self.enemy_list_state.data.scan_receiver.is_some() || self.stage_list_state.data.scan_receiver.is_some() {
            ctx.request_repaint();
        }

        let import_finished = self.import_state.update(ctx);
        if import_finished {
            ::tracing::info!("Import job finished, performing full data reload");
            self.perform_full_data_reload();
            ctx.request_repaint();
        }

        frame::draw(self, ctx);
    }
}