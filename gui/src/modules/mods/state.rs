use serde::{Deserialize, Serialize};

use core::modules::mods::ModDataState;

use crate::common::shared::DragGuard;

use super::list::ModList;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ModListState {
    pub data: ModDataState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub list: Option<ModList>,
}