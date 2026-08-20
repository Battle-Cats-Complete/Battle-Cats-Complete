use std::collections::HashSet;

use nyanko::chapter::Category;

use super::{GlobalMapId, GlobalStageId, Map, Stage, StageRegistry};

pub fn get_categories(registry: &StageRegistry) -> Vec<Category> {
    let unique_categories: HashSet<Category> = registry.maps.keys()
        .map(|id| id.category.clone())
        .collect();

    unique_categories.into_iter().collect()
}

pub fn get_maps<'a>(registry: &'a StageRegistry, category: &Category) -> Vec<&'a Map> {
    let mut maps: Vec<&Map> = registry.maps.iter()
        .filter(|(id, _)| id.category == *category)
        .map(|(_, m)| m)
        .collect();

    maps.sort_by_key(|m| m.map_id);
    maps
}

pub fn get_stages<'a>(registry: &'a StageRegistry, map_id: &GlobalMapId) -> Vec<&'a Stage> {
    let Some(map) = registry.maps.get(map_id) else { return Vec::new(); };

    let mut stages: Vec<&Stage> = map.stages.iter()
        .filter_map(|&stage_id| {
            let stage_key = GlobalStageId {
                category: map_id.category.clone(),
                map: map_id.map,
                stage: stage_id,
            };
            registry.stages.get(&stage_key)
        })
        .collect();

    stages.sort_by_key(|s| s.stage_id);
    stages
}