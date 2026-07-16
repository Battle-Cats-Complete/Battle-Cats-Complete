use std::path::Path;

use eframe::egui;

use core::modules::cat::paths as cat_paths;
use core::modules::cat::waiter::{skilldescriptions, skilllevel};
use core::common::assets;
use core::common::game::waiter::{localizable, param};
use core::common::io::json;
use core::modules::settings::logic::{lang, upd::UpdateMode};

use crate::app::updater;

use super::BattleCatsApp;

impl BattleCatsApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let mut app: Self = json::load("settings.json").unwrap_or_default();

        crate::app::tracing::init(app.settings.general.enable_logging);
        tracing::info!("Starting initialization sequence...");

        core::modules::settings::logic::exceptions::ExceptionList::sync_on_boot();

        #[cfg(target_os = "linux")]
        {
            tracing::debug!("Syncing Linux desktop data");
            let _ = core::modules::settings::logic::desktop::sync_desktop_data();
        }

        lang::ensure_complete_list(&mut app.settings.general.language_priority);

        tracing::debug!("Setting up custom fonts");
        setup_custom_fonts(&creation_context.egui_ctx);

        tracing::debug!("Refreshing mod state and cleaning up temp update files");
        app.mod_state.data.refresh_mods();
        updater::cleanup_temp_files();

        tracing::info!("Loading core tables");
        let tables_dir = Path::new("game/tables");
        let loc_dir = Path::new("game/tables/localizable");
        let priority = &app.settings.general.language_priority;

        app.param = param(tables_dir, priority).unwrap_or_else(|| Default::default());
        app.localizable = localizable(loc_dir, priority);

        let mut expected_hash = 0;
        let mut needs_validation = false;

        if let Some((hash, cached_cats)) = core::common::io::cache::load_with_hash::<Vec<core::modules::cat::logic::scanner::CatEntry>>("cats_cache.bin") {
            tracing::info!("Found cats_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            let cats_dir = Path::new(cat_paths::DIR_CATS);
            let costs_arc = std::sync::Arc::new(skilllevel(cats_dir, priority));
            let descs_arc = std::sync::Arc::new(skilldescriptions(cats_dir, priority));

            app.cat_list_state.data.cats = cached_cats.into_iter().map(|mut cat| {
                cat.talent_costs = std::sync::Arc::clone(&costs_arc);
                cat.skill_descriptions = std::sync::Arc::clone(&descs_arc);
                cat
            }).collect();

            app.cat_list_state.data.initialized = true;
        } else {
            tracing::info!("No cats_cache.bin found, triggering full cat scan");
            app.cat_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if let Some((hash, cached_enemies)) = core::common::io::cache::load_with_hash::<Vec<core::modules::enemy::logic::scanner::EnemyEntry>>("enemies_cache.bin") {
            tracing::info!("Found enemies_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;
            app.enemy_list_state.data.enemies = cached_enemies;
            app.enemy_list_state.data.initialized = true;
        } else {
            tracing::info!("No enemies_cache.bin found, triggering full enemy scan");
            app.enemy_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if let Some((hash, cached_registry)) = core::common::io::cache::load_with_hash::<core::modules::stage::registry::StageRegistry>("stages_cache.bin") {
            tracing::info!("Found stages_cache.bin (Hash: {})", hash);
            expected_hash = hash;
            needs_validation = true;

            app.stage_list_state.data.registry = cached_registry;

            let config = app.settings.scanner_config();
            app.stage_list_state.data.load_dictionaries(&config);

            let enemies_ref = app.enemy_list_state.data.enemies.clone();
            app.stage_list_state.data.sync_enemies(&enemies_ref);

            app.stage_list_state.data.initialized = true;

            tracing::info!("Triggering silent background validation scan for stages...");
            app.stage_list_state.data.restart_scan(app.settings.scanner_config());
        } else {
            tracing::info!("No stages_cache.bin found, triggering full stage scan");
            app.stage_list_state.data.restart_scan(app.settings.scanner_config());
        }

        if needs_validation {
            tracing::debug!("Spawning hash validation thread");
            let (tx, rx) = std::sync::mpsc::channel();
            app.hash_rx = Some(rx);
            let active_mod = core::common::resolver::get_active_mod();

            std::thread::spawn(move || {
                let cur_hash = core::common::io::cache::get_game_hash(active_mod.as_deref());
                let is_valid = cur_hash == expected_hash && active_mod.is_none();
                let _ = tx.send(is_valid);
            });
        }

        if app.settings.general.update_mode != UpdateMode::Ignore {
            tracing::info!("Checking for app updates at startup");
            app.updater.check_for_updates(creation_context.egui_ctx.clone(), false);
        }

        tracing::info!("Initialization sequence complete");
        app
    }
}

fn setup_custom_fonts(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("jp_font".to_owned(), egui::FontData::from_static(assets::FONT_JP));
    fonts.font_data.insert("kr_font".to_owned(), egui::FontData::from_static(assets::FONT_KR));
    fonts.font_data.insert("tc_font".to_owned(), egui::FontData::from_static(assets::FONT_TC));
    fonts.font_data.insert("thai_font".to_owned(), egui::FontData::from_static(assets::FONT_TH));

    let families = [egui::FontFamily::Proportional, egui::FontFamily::Monospace];
    for family in families {
        let Some(list_ref) = fonts.families.get_mut(&family) else { continue; };

        list_ref.push("jp_font".to_owned());
        list_ref.push("kr_font".to_owned());
        list_ref.push("tc_font".to_owned());
        list_ref.push("thai_font".to_owned());
    }
    context.set_fonts(fonts);
}