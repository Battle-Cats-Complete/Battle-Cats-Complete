use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppState {
    pub(crate) cat_data: CatListState,
    pub(crate) enemy_data: EnemyListState,
    pub(crate) game_data: GameDataState,
    pub(crate) animation: AnimState,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct CatListState {
    pub list_scroll_offset: f32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct EnemyListState {
    pub list_scroll_offset: f32,
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
    pub last_export_format: i32,
    pub last_export_quality: Option<i32>,
    pub last_export_compression: Option<i32>,
    pub controls_expanded: bool,
    pub export_popup_open: bool,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            last_export_format: 0,
            last_export_quality: None,
            last_export_compression: None,
            controls_expanded: true,
            export_popup_open: false,
        }
    }
}
