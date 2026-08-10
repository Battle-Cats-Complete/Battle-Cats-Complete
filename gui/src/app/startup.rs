use std::fs;
use std::path::Path;
use std::sync::Arc;

use iced::Task;
use smol::Timer;
use tracing::{debug, error, info, warn};

use core::common::dirs;
use core::common::game::{localizable, param};
use core::common::io::json;
use core::modules::mods;
use core::modules::settings::{desktop, lang, ExceptionList, Settings, UpdateMode};
use core::{ContentStore, Store};

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

        let active_mod = app.mods_state.active_mod();
        app.store = Arc::new(build_store(&app.settings, active_mod.as_deref()));

        info!("Loading core tables");
        app.param = param(&app.store.vfs).unwrap_or_default();
        app.localizable = localizable(&app.store.vfs);

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

        let boot_loads = Task::batch([
            app.cat_state.start_load(&app.settings, &app.store, active_mod.clone()).map(Message::Cat),
            app.enemy_state.start_load(&app.settings, &app.store, active_mod.clone()).map(Message::Enemy),
            app.stage_state.start_load(&app.settings, &app.store, active_mod).map(Message::Stage),
        ]);

        let reveal_fallback = Task::future(Timer::after(super::WINDOW_SHOW_FALLBACK)).map(|_| Message::ShowWindow);

        info!("Initialization sequence complete");

        (app, Task::batch([home_task.map(Message::Home), updater_task, icon_streams, boot_loads, reveal_fallback]))
    }
}

fn build_store(settings: &Settings, active_mod: Option<&str>) -> Store {
    let mut store = Store::new(settings);
    let hash = Store::hash(active_mod);

    if store.vfs.restore(hash) {
        debug!(hash, "Restored file index from vfs.bin");
    } else {
        mount_game(&store);

        if let Some(name) = active_mod {
            mount_mod(&store, name);
        }

        store.vfs.persist(hash);
    }

    if let Some(content) = ContentStore::load(hash) {
        debug!(hash, "Restored parsed tables from content.bin");
        content.apply(&mut store.vds);
    }

    store
}

fn mount_game(store: &Store) {
    info!("Indexing game data");

    match store.vfs.create(Path::new("game")) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in game data, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!("Failed to index game data: {}", err),
    }
}

fn mount_mod(store: &Store, name: &str) {
    info!(mod_name = name, "Mounting active mod");

    match mods::enable(store, name) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in mod, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!(mod_name = name, "Failed to mount active mod: {}", err),
    }
}
