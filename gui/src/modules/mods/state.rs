use serde::{Deserialize, Serialize};

use core::mods::logic::state::ModDataState;

use crate::global::shared::DragGuard;

use super::list::ModList;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModListState {
    pub data: ModDataState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub list: Option<ModList>,
}