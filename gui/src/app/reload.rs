use std::path::Path;

use core::common::game::waiter::{localizable, param};

use super::BattleCatsApp;

impl BattleCatsApp {
    pub(crate) fn perform_full_data_reload(&mut self) {
        tracing::info!("Executing perform_full_data_reload");

        self.cat_list_state.texture_cache_version += 1;
        self.cat_list_state.anim_viewer.loaded_id.clear();
        self.cat_list_state.detail_texture = None;
        self.cat_list_state.data.detail_key.clear();

        self.cat_list_state.img015_sheets.clear();
        self.cat_list_state.img022_sheets.clear();
        self.cat_list_state.gatya_item_textures.clear();

        self.enemy_list_state.anim_viewer.loaded_id.clear();
        self.enemy_list_state.detail_texture = None;
        self.enemy_list_state.data.detail_key.clear();
        self.enemy_list_state.img015_sheets.clear();

        tracing::debug!("Flushing stage UI image caches");
        self.stage_list_state.enemy_texture_cache.clear();
        self.stage_list_state.item_texture_cache.clear();
        self.stage_list_state.stage_texture_cache.clear();
        self.stage_list_state.cat_texture_cache.clear();

        let viewers = [
            &mut self.cat_list_state.anim_viewer,
            &mut self.enemy_list_state.anim_viewer,
        ];

        for viewer in viewers {
            viewer.loaded_id.clear();
            viewer.held_unit = None;
            viewer.current_anim = None;
            viewer.current_frame = 0.0;
            viewer.texture_version += 1;
        }

        let config = self.settings.scanner_config();

        tracing::debug!("Dropping old UI caches and restarting data scans");

        self.cat_list_state.cat_list = Default::default();
        self.cat_list_state.data.restart_scan(config.clone());

        self.enemy_list_state.enemy_list = Default::default();
        self.enemy_list_state.data.restart_scan(config.clone());

        self.stage_list_state.data.registry.clear_cache();
        self.stage_list_state.data.restart_scan(config);

        tracing::debug!("Reloading core tables");

        let langs = &self.settings.general.language_priority;
        let tables_dir = Path::new("game/tables");
        let loc_dir = Path::new("game/tables/localizable");

        self.param = param(tables_dir, langs).unwrap_or_default();
        self.localizable = localizable(loc_dir, langs);
    }
}