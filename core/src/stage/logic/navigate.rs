use std::collections::HashSet;

use nyanko::chapter::Category;
use crate::stage::registry::{StageRegistry, Map, Stage, GlobalMapId, GlobalStageId};

pub fn get_categories(registry: &StageRegistry) -> Vec<Category> {
    let unique_categories: HashSet<Category> = registry.maps.keys()
        .map(|id| id.category.clone())
        .collect();

    unique_categories.into_iter().collect()
}

pub fn get_maps(registry: &StageRegistry, category: &Category) -> Vec<Map> {
    let mut maps: Vec<Map> = registry.maps.iter()
        .filter(|(id, _)| id.category == *category)
        .map(|(_, m)| m.clone())
        .collect();

    maps.sort_by_key(|m| m.map_id);
    maps
}

pub fn get_stages(registry: &StageRegistry, map_id: &GlobalMapId) -> Vec<Stage> {
    let Some(map) = registry.maps.get(map_id) else { return Vec::new(); };

    let mut stages: Vec<Stage> = map.stages.iter()
        .filter_map(|&stage_id| {
            let stage_key = GlobalStageId {
                category: map_id.category.clone(),
                map: map_id.map,
                stage: stage_id,
            };
            registry.stages.get(&stage_key)
        })
        .cloned()
        .collect();

    stages.sort_by_key(|s| s.stage_id);
    stages
}