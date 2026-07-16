use std::collections::HashMap;

use eframe::egui;
use egui::TextureHandle;
use serde::{Deserialize, Serialize};

use core::enemy::logic::scanner::EnemyEntry;
use core::stage::logic::filter::{CompiledStageFilter, StageFilterState};
use core::stage::logic::state::StageDataState;

use crate::global::shared::DragGuard;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct StageListState {
    pub data: StageDataState,
    pub is_list_open: bool,
    pub selected_crown: u8,

    #[serde(skip)] pub filter_state: StageFilterState,
    #[serde(skip)] pub compiled_filter: Option<CompiledStageFilter>,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub enemy_texture_cache: HashMap<u32, TextureHandle>,
    #[serde(skip)] pub item_texture_cache: HashMap<u32, TextureHandle>,
    #[serde(skip)] pub stage_texture_cache: HashMap<String, TextureHandle>,
    #[serde(skip)] pub cat_texture_cache: HashMap<String, TextureHandle>,
}

impl StageListState {
    pub fn update_data(&mut self) {
        self.data.update_data();
    }

    pub fn sync_enemies(&mut self, extracted_enemies_array: &[EnemyEntry]) {
        self.data.sync_enemies(extracted_enemies_array);
    }
}