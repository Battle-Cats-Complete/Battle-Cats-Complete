use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use iced::futures::channel::mpsc;
use iced::Task;
use smol::Timer;
use tracing::{debug, error, info, warn};

use core::common::dirs;
use core::common::game::{localizable, param};
use core::common::io::json;
use core::modules::data::architecture;
use core::modules::mods;
use core::modules::settings::{desktop, lang, ExceptionList, UpdateMode};
use core::{ContentStore, Vault};

use crate::modules::home;

use super::{logging, migrate, notice, updater, ActivePopup, BattleCatsApp, Message};

impl BattleCatsApp {
    pub fn new() -> (Self, Task<Message>) {
        let migration_notes = migrate::run();

        let mut app: Self = json::load("settings.json").unwrap_or_default();
        app.app_state = json::load_state("state.json").unwrap_or_default();

        logging::init_logging(app.settings.general.enable_logging);

        for note in migration_notes {
            match note {
                migrate::Note::Info(message) => info!("{}", message),
                migrate::Note::Warn(message) => warn!("{}", message),
            }
        }

        info!("Starting initialization sequence...");

        app.cat_state.restore_state(&app.app_state.cat);
        app.enemy_state.restore_state(&app.app_state.enemy);
        app.stage_state.restore_state(&app.app_state.stage);

        if notice::should_show(&app.app_state.notice.acknowledged) {
            info!("Notice {} not yet acknowledged, showing at startup", notice::hash());
            app.notice_open = true;
            app.sync_popup(ActivePopup::VersionNotice, true);
        }

        if let Some(state_dir) = dirs::state() {
            let _ = fs::remove_file(state_dir.join("meta.json"));
        }

        ExceptionList::sync_on_boot();

        #[cfg(target_os = "linux")]
        {
            debug!("Syncing Linux desktop data");
            let _ = desktop::sync_desktop_data();
        }

        lang::ensure_complete_list(&mut app.settings.general.language_priority);

        debug!("Cleaning up temp update files");
        updater::cleanup_temp_files();

        let store_task = app.spawn_vault_build();

        let updater_task = if app.settings.general.update_mode != UpdateMode::Ignore {
            info!("Checking for app updates at startup");
            app.check_for_updates(false)
        } else {
            Task::none()
        };

        let (home_state, home_task) = home::State::new();
        app.home_state = home_state;

        let icon_streams = Task::batch([
            app.cat_state.icon_stream().map(Message::Cat),
            app.enemy_state.icon_stream().map(Message::Enemy),
            app.mods_state.icon_stream().map(Message::Mod),
        ]);

        let reveal_fallback = Task::future(Timer::after(super::WINDOW_SHOW_FALLBACK)).map(|_| Message::ShowWindow);

        info!("Initialization sequence complete");

        (app, Task::batch([home_task.map(Message::Home), updater_task, icon_streams, store_task, reveal_fallback]))
    }

    fn spawn_vault_build(&mut self) -> Task<Message> {
        info!("Building the file index in the background");

        let mut vault = Vault::new(&self.settings);
        let active_mod = self.mods_state.active_mod();
        let (tx, rx) = mpsc::unbounded();

        self.cat_state.set_indexing();
        self.enemy_state.set_indexing();
        self.stage_state.set_indexing();

        thread::spawn(move || {
            populate_vault(&mut vault, active_mod.as_deref());
            let _ = tx.unbounded_send(Message::VaultReady(Arc::new(vault)));
        });

        Task::stream(rx)
    }

    pub(super) fn adopt_vault(&mut self, vault: Arc<Vault>) -> Task<Message> {
        self.vault = vault;
        self.vault_ready = true;

        info!("Loading core tables");
        self.param = param(&self.vault.vfs).unwrap_or_default();
        self.localizable = localizable(&self.vault.vfs);

        self.sync_home_status();

        let active_mod = self.mods_state.active_mod();

        Task::batch([
            self.cat_state.start_load(&self.settings, &self.vault, active_mod.clone()).map(Message::Cat),
            self.enemy_state.start_load(&self.settings, &self.vault, active_mod.clone()).map(Message::Enemy),
            self.stage_state.start_load(&self.settings, &self.vault, active_mod).map(Message::Stage),
        ])
    }
}

fn populate_vault(vault: &mut Vault, active_mod: Option<&str>) {
    let hash = Vault::hash(active_mod);

    if vault.vfs.restore(hash) {
        debug!(hash, "Restored file index from vfs.bin");
    } else {
        mount_game(vault);

        if let Some(name) = active_mod {
            mount_mod(vault, name);
        }

        vault.vfs.persist(hash);
    }

    if let Some(content) = ContentStore::load(hash) {
        debug!(hash, "Restored parsed tables from content.bin");
        content.apply(&mut vault.vds);
    }
}

fn mount_game(vault: &Vault) {
    info!("Indexing game data");

    match vault.vfs.create(Path::new(architecture::GAME)) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in game data, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!("Failed to index game data: {}", err),
    }
}

fn mount_mod(vault: &Vault, name: &str) {
    info!(mod_name = name, "Mounting active mod");

    match mods::enable(vault, name) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in mod, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!(mod_name = name, "Failed to mount active mod: {}", err),
    }
}
