use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use core::modules::animation::export::ExportFormat;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppState {
    pub(crate) cat_data: CatListState,
    pub(crate) enemy_data: EnemyListState,
    pub(crate) game_data: GameDataState,
    pub(crate) animation: AnimState,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct CatListState {
    pub list_scroll_offset: f32,
    pub selected_cat: Option<u32>,
    pub selected_form: usize,
    pub search_query: String,
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>,
    pub current_level: i32,
    pub level_input: String,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            list_scroll_offset: 0.0,
            selected_cat: None,
            selected_form: 0,
            search_query: String::new(),
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            current_level: 1,
            level_input: String::from("1"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct EnemyListState {
    pub list_scroll_offset: f32,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct GameDataState {
    pub adb_import_type_idx: usize,
    pub adb_region_idx: usize,
}

impl Default for GameDataState {
    fn default() -> Self {
        Self {
            adb_import_type_idx: 0,
            adb_region_idx: 4,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct AnimState {
    pub last_export_format: ExportFormat,
    pub last_export_quality: Option<i32>,
    pub last_export_compression: Option<i32>,
    pub controls_expanded: bool,
    pub export_popup_open: bool,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            last_export_format: ExportFormat::Gif,
            last_export_quality: None,
            last_export_compression: None,
            controls_expanded: true,
            export_popup_open: false,
        }
    }
}
