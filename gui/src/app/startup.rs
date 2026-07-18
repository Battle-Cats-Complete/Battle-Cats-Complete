use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;

use iced::Task;
use tracing::{debug, info};

use core::common::game::{localizable, param};
use core::common::io::{cache, json};
use core::common::resolver;
use core::modules::cat::paths as cat_paths;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::waiter::{skilldescriptions, skilllevel};
use core::modules::enemy::scanner::EnemyEntry;
use core::modules::settings::{desktop, lang, ExceptionList, UpdateMode};
use core::modules::stage::StageRegistry;

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

        debug!("Refreshing mod state and cleaning up temp update files");
        app.mod_state.data.refresh_mods();
        updater::cleanup_temp_files();

        info!("Loading core tables");
        let tables_dir = Path::new("game/tables");
        let loc_dir = Path::new("game/tables/localizable");
        let priority = &app.settings.general.language_priority;

        app.param = param(tables_dir, priority).unwrap_or_default();
        app.localizable = localizable(loc_dir, priority);

        let mut expected_hash = 0;
        let mut needs_validation = false;

        if let Some((hash, cached_cats)) = cache::load_with_hash::<Vec<CatEntry>>("cats_cache.bin") {
            info!("Found cats_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            let cats_dir = Path::new(cat_paths::DIR_CATS);
            let costs_arc = Arc::new(skilllevel(cats_dir, priority));
            let descriptions_arc = Arc::new(skilldescriptions(cats_dir, priority));

            app.cat_list_state.data.cats = cached_cats.into_iter().map(|mut cat| {
                cat.talent_costs = Arc::clone(&costs_arc);
                cat.skill_descriptions = Arc::clone(&descriptions_arc);
                cat
            }).collect();

            app.cat_list_state.data.initialized = true;
        } else {
            info!("No cats_cache.bin found, triggering full cat scan");
            app.cat_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if let Some((hash, cached_enemies)) = cache::load_with_hash::<Vec<EnemyEntry>>("enemies_cache.bin") {
            info!("Found enemies_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;
            app.enemy_list_state.data.enemies = cached_enemies;
            app.enemy_list_state.data.initialized = true;
        } else {
            info!("No enemies_cache.bin found, triggering full enemy scan");
            app.enemy_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if let Some((hash, cached_registry)) = cache::load_with_hash::<StageRegistry>("stages_cache.bin") {
            info!("Found stages_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            app.stage_list_state.data.registry = cached_registry;

            let config = app.settings.scanner_config();
            app.stage_list_state.data.load_dictionaries(&config);

            let enemies_ref = app.enemy_list_state.data.enemies.clone();
            app.stage_list_state.data.sync_enemies(&enemies_ref);

            app.stage_list_state.data.initialized = true;

            info!("Triggering silent background validation scan for stages...");
            app.stage_list_state.data.restart_scan(app.settings.scanner_config());
        } else {
            info!("No stages_cache.bin found, triggering full stage scan");
            app.stage_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if needs_validation {
            debug!("Spawning hash validation thread");
            let (tx, rx) = mpsc::channel();
            app.hash_rx = Some(rx);
            let active_mod = resolver::get_active_mod();

            thread::spawn(move || {
                let current_hash = cache::get_game_hash(active_mod.as_deref());
                let is_valid = current_hash == expected_hash && active_mod.is_none();
                let _ = tx.send(is_valid);
            });
        }

        let (up_tx, up_rx) = mpsc::channel();
        app.updater_tx = Some(up_tx);
        app.updater_rx = Some(up_rx);

        if app.settings.general.update_mode != UpdateMode::Ignore {
            info!("Checking for app updates at startup");
            app.check_for_updates(false);
        }

        info!("Initialization sequence complete");

        (app, Task::none())
    }
}