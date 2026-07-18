use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread;

use eframe::egui;
use tracing::{debug, info, trace};

use core::common::io::cache;
use core::common::resolver;
use core::modules::cat::paths as cat_paths;
use core::modules::cat::patterns as cat_patterns;
use core::modules::cat::scanner as cat_loader;
use core::modules::enemy::scanner as enemy_loader;
use core::modules::stage::filter::enemy::EnemyFilter;
use core::modules::stage::filter::StageFilterState;

use crate::common::GuiWatcher;

use super::{BattleCatsApp, Page};

impl BattleCatsApp {
    pub(crate) fn process_file_events(&mut self, ctx: &egui::Context) {
        if self.global_watcher.is_none() {
            self.global_watcher = GuiWatcher::new(ctx.clone());
        }

        let Some(watcher) = &self.global_watcher else { return; };

        let mut paths = Vec::new();
        while let Ok(path) = watcher.rx.try_recv() {
            paths.push(path);
        }

        if paths.is_empty() { return; }

        if self.import_state.config.import_rx.is_some() || self.import_state.config.import_job_status.load(Ordering::Relaxed) == 1 { return; }
        if self.import_state.config.export_rx.is_some() || self.import_state.config.export_job_status.load(Ordering::Relaxed) == 1 { return; }

        trace!("File watcher received {} paths to process", paths.len());

        let mut cat_ids = HashSet::new();
        let mut enemy_ids = HashSet::new();
        let mut mod_icons = HashSet::new();

        let mut global_cat = false;
        let mut global_enemy = false;
        let mut global_stage = false;
        let mut refresh_mods = false;
        let mut mod_changed = false;

        let active_mod = self.mod_state.data.loaded_mods.iter()
            .find(|m| m.enabled)
            .map(|m| m.folder_name.clone());

        resolver::set_active_mod(active_mod.clone());

        for path in paths {
            let path_str = path.to_string_lossy().to_lowercase();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            trace!("Processing modified path: {}", path_str);

            let is_mod = path_str.contains("mods") && !path_str.contains("packages");
            if is_mod {
                refresh_mods = true;
                if Self::check_if_active_mod_changed(&path, active_mod.as_deref()) {
                    mod_changed = true;
                }

                if path_str.contains("icons") && file_name == "icon.png" {
                    let index_option = path.components().position(|c| c.as_os_str().to_string_lossy().to_lowercase() == "mods");
                    if let Some(index) = index_option {
                        if let Some(folder) = path.components().nth(index + 1) {
                            mod_icons.insert(folder.as_os_str().to_string_lossy().into_owned());
                        }
                    }
                }
            }

            if path_str.contains("img015") || path_str.contains("img022") {
                self.cat_list_state.img015_sheets.clear();
                self.cat_list_state.img022_sheets.clear();
                self.enemy_list_state.img015_sheets.clear();
            }

            if path_str.contains("ui") || path_str.contains("gatyaitem") || path_str.contains("sheets") {
                self.cat_list_state.gatya_item_textures.clear();
                self.cat_list_state.texture_cache_version += 1;
            }

            if path_str.contains("tables") {
                global_cat = true;
                global_enemy = true;
                global_stage = true;
            }

            let cat_global = cat_patterns::CAT_UNIVERSAL_FILES.contains(&file_name);
            let cat_buy = file_name == cat_paths::UNIT_BUY;
            let cat_evolve = path_str.contains(cat_paths::DIR_UNIT_EVOLVE) || path_str.contains("unitevolve");
            let is_cat_dir = path_str.contains("cats");

            if cat_global || cat_buy || cat_evolve {
                global_cat = true;
                global_stage = true;
            } else if is_cat_dir {
                global_stage = true;
                if self.process_cat_path(&path, &mut cat_ids) {
                    global_cat = true;
                }
            }

            let enemy_global = file_name.contains("t_unit") || file_name.contains("enemyname") || file_name.contains("enemypicturebook");
            let is_enemy_dir = path_str.contains("enemies");

            if enemy_global {
                global_enemy = true;
                global_stage = true;
            } else if is_enemy_dir {
                global_stage = true;
                if self.process_enemy_path(&path, &mut enemy_ids) {
                    global_enemy = true;
                }
            }

            if path_str.contains("stages") {
                global_stage = true;
            }
        }

        if refresh_mods {
            debug!("Refreshing UI mods list");
            self.mod_state.data.refresh_mods();
        }

        if !mod_icons.is_empty() {
            if let Some(list) = &mut self.mod_state.list {
                for name in mod_icons {
                    list.flush_icon(&name);
                }
            }
        }

        let game_dir = Path::new("game");
        let is_empty = !game_dir.exists() || std::fs::read_dir(game_dir).map(|mut iterator| iterator.next().is_none()).unwrap_or(true);
        ctx.data_mut(|data_map| data_map.insert_temp(egui::Id::new("is_game_empty"), is_empty));

        if mod_changed || global_cat || global_enemy {
            info!("Global files or active mod changed. Triggering full reload.");
            resolver::clear_override_cache();
            self.perform_full_data_reload();
            ctx.request_repaint();
            return;
        }

        let limit = 5;
        let config = self.settings.scanner_config();

        if cat_ids.len() > limit {
            debug!("Mass threshold exceeded for cats, resyncing scan...");
            self.cat_list_state.detail_texture = None;
            self.cat_list_state.data.detail_key.clear();
            self.cat_list_state.texture_cache_version += 1;
            self.cat_list_state.anim_viewer.loaded_id.clear();
            cat_loader::resync_scan(&mut self.cat_list_state.data, config.clone());
        } else {
            for &id in &cat_ids {
                self.cat_list_state.cat_list.flush_icon(id);
                if self.cat_list_state.data.selected_cat == Some(id) {
                    self.cat_list_state.detail_texture = None;
                    self.cat_list_state.data.detail_key.clear();
                    self.cat_list_state.texture_cache_version += 1;
                }
                cat_loader::refresh_cat(&mut self.cat_list_state.data, id, config.clone());
            }
        }

        if enemy_ids.len() > limit {
            debug!("Mass threshold exceeded for enemies, resyncing scan...");
            self.enemy_list_state.detail_texture = None;
            self.enemy_list_state.data.detail_key.clear();
            enemy_loader::resync_scan(&mut self.enemy_list_state.data, config.clone());
        } else {
            for &id in &enemy_ids {
                self.enemy_list_state.enemy_list.flush_icon(id);
                if self.enemy_list_state.data.selected_enemy == Some(id) {
                    self.enemy_list_state.detail_texture = None;
                    self.enemy_list_state.data.detail_key.clear();
                }
                enemy_loader::refresh_enemy(&mut self.enemy_list_state.data, id, &config);
            }
        }

        if global_stage {
            info!("Cascading stage reload triggered.");
            self.stage_list_state.data.registry.clear_cache();
            self.stage_list_state.data.sync_enemies(&self.enemy_list_state.data.enemies);
            self.stage_list_state.data.restart_scan(config);
        }

        let has_changes = !cat_ids.is_empty() || !enemy_ids.is_empty();

        if has_changes && !resolver::is_mod_active() {
            debug!("Updating vanilla cache bins in background thread");
            let cats = self.cat_list_state.data.cats.clone();
            let enemies = self.enemy_list_state.data.enemies.clone();

            thread::spawn(move || {
                let current_hash = cache::get_game_hash(None);
                cache::save("cats_cache.bin", current_hash, &cats);
                cache::save("enemies_cache.bin", current_hash, &enemies);
            });
        }

        ctx.request_repaint();
    }

    pub(crate) fn check_if_active_mod_changed(path: &Path, active_mod: Option<&str>) -> bool {
        let Some(active) = active_mod else { return false; };

        let path_components: Vec<_> = path.components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect();

        let Some(index) = path_components.iter().position(|c| c == "mods") else { return false; };
        let Some(folder) = path_components.get(index + 1) else { return false; };

        folder == &active.to_lowercase()
    }

    pub(crate) fn process_cat_path(&mut self, path: &Path, cat_ids: &mut HashSet<u32>) -> bool {
        let path_components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();

        let Some(index) = path_components.iter().position(|c| c == "cats") else { return false; };
        let Some(folder) = path_components.get(index + 1) else { return false; };

        let parsed = if let Ok(id) = folder.parse::<u32>() {
            Some(id)
        } else if folder.starts_with("egg_") {
            folder[4..].parse::<u32>().ok()
        } else {
            None
        };

        let Some(id) = parsed else { return true; };

        let is_anim = path_components.get(index + 3).map(|s| s.as_ref()) == Some("anim");
        if !is_anim || self.cat_list_state.data.selected_cat != Some(id) {
            cat_ids.insert(id);
            return false;
        }

        let form = match path_components.get(index + 2) {
            Some(s) => s.to_string(),
            None => "f".to_string(),
        };

        let marker = format!("_{}_", form);
        let loaded = &mut self.cat_list_state.anim_viewer.loaded_id;

        if loaded.is_empty() || loaded.contains(&marker) {
            loaded.clear();
            self.cat_list_state.anim_viewer.texture_version += 1;
        }

        false
    }

    pub(crate) fn process_enemy_path(&mut self, path: &Path, enemy_ids: &mut HashSet<u32>) -> bool {
        let path_components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();

        let Some(index) = path_components.iter().position(|c| c == "enemies") else { return false; };
        let Some(folder) = path_components.get(index + 1) else { return false; };

        let Ok(id) = folder.parse::<u32>() else { return true; };

        let is_anim = path_components.get(index + 2).map(|s| s.as_ref()) == Some("anim");
        if !is_anim || self.enemy_list_state.data.selected_enemy != Some(id) {
            enemy_ids.insert(id);
            return false;
        }

        let loaded = &mut self.enemy_list_state.anim_viewer.loaded_id;
        loaded.clear();
        self.enemy_list_state.anim_viewer.texture_version += 1;

        false
    }

    pub(crate) fn process_ui_events(&mut self, ctx: &egui::Context) {
        let Some(enemy_id) = ctx.data_mut(|d| d.remove_temp::<u32>(egui::Id::new("navigate_to_stage_appearances"))) else { return; };

        info!("Navigating to stage appearances for enemy ID: {}", enemy_id);

        self.current_page = Page::Stages;
        self.stage_list_state.is_list_open = true;

        self.stage_list_state.filter_state.is_open = false;
        self.stage_list_state.filter_state = StageFilterState::default();

        let mut enemy_filter = EnemyFilter::default();
        enemy_filter.name_or_id = enemy_id.to_string();
        self.stage_list_state.filter_state.enemies.push(enemy_filter);

        self.settings.runtime.show_ip_field = false;
    }
}