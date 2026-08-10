use std::fs;

use iced::Task;
use smol::Timer;
use tracing::{debug, info, warn};

use core::common::dirs;
use core::common::game::{localizable, param};
use core::common::io::json;
use core::modules::settings::{desktop, lang, ExceptionList, UpdateMode};

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

        let active_mod = app.mods_state.active_mod();

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
