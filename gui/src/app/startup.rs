use std::path::Path;

use iced::Task;
use tracing::{debug, info};

use core::common::game::{localizable, param};
use core::common::io::json;
use core::modules::settings::{desktop, lang, ExceptionList, UpdateMode};

use crate::modules::home;

use super::{logging, updater, BattleCatsApp, Message};

impl BattleCatsApp {
    pub fn new() -> (Self, Task<Message>) {
        let mut app: Self = json::load("settings.json").unwrap_or_default();

        logging::init_logging(app.settings.general.enable_logging);
        info!("Starting initialization sequence...");

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
        let tables_dir = Path::new("game/tables");
        let loc_dir = Path::new("game/tables/localizable");
        let priority = &app.settings.general.language_priority;

        app.param = param(tables_dir, priority).unwrap_or_default();
        app.localizable = localizable(loc_dir, priority);

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

        info!("Initialization sequence complete");

        (app, Task::batch([home_task.map(Message::Home), updater_task, icon_streams]))
    }
}
