use nyanko::cat::unit::{Battle, UnitBuy};
use tracing::trace;

use crate::modules::cat::filter::{CatFilterState, FilterCounts};
use crate::modules::cat::game::registry::CAT_STATS_REGISTRY;
use crate::modules::cat::scanner::CatEntry;

pub(crate) fn get_stat_value(battle_stats: &Battle, stat_name: &str, animation_frames: i32, unitbuy: Option<&UnitBuy>) -> i32 {
    let registry_name = match stat_name {
        "Cooldown (f)" => "Cooldown",
        "Atk Cycle (f)" => "Atk Cycle",
        _ => stat_name,
    };

    let target_definition = CAT_STATS_REGISTRY.iter().find(|stat_definition| stat_definition.name == registry_name);

    if let Some(definition) = target_definition {
        return (definition.get_value)(battle_stats, animation_frames, unitbuy);
    }

    0
}

pub(crate) fn evaluate_stat_ranges(
    cat: &CatEntry,
    form_index: usize,
    filter: &CatFilterState,
    stats_min: &Battle,
    stats_max: &Battle,
    counts: &mut FilterCounts
) {
    let animation_frames = cat.atk_anim_frames[form_index];
    let unitbuy_ref = Some(&cat.unitbuy);
    trace!("evaluating statistical filters");

    for (stat_name, target_range) in &filter.stat_ranges {
        if target_range.min.is_empty() && target_range.max.is_empty() { continue; }

        counts.active += 1;

        let value_a = get_stat_value(stats_min, stat_name, animation_frames, unitbuy_ref);
        let value_b = get_stat_value(stats_max, stat_name, animation_frames, unitbuy_ref);

        let actual_min = value_a.min(value_b);
        let actual_max = value_a.max(value_b);

        let required_min = target_range.min.parse::<i32>().unwrap_or(i32::MIN);
        let required_max = target_range.max.parse::<i32>().unwrap_or(i32::MAX);

        if actual_min <= required_max && actual_max >= required_min {
            counts.passed += 1;
        } else {
            counts.failed += 1;
        }
    }
}