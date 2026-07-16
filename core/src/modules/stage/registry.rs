use std::collections::HashMap;

use nyanko::chapter::Category;
pub use nyanko::chapter::Map;
pub use nyanko::chapter::Stage;

#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GlobalMapId {
    pub category: Category,
    pub map: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GlobalStageId {
    pub category: Category,
    pub map: u32,
    pub stage: u32,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct StageRegistry {
    pub maps: HashMap<GlobalMapId, Map>,
    pub stages: HashMap<GlobalStageId, Stage>,
}

impl StageRegistry {
    pub fn clear_cache(&mut self) {
        self.maps.clear();
        self.stages.clear();
    }
}