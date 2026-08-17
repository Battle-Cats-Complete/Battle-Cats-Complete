use std::collections::HashMap;

use tracing::trace;

use crate::domains::cat::filter::icons::{evaluate_icon_requirements, TalentBuilds};
use crate::domains::cat::filter::stats::evaluate_stat_ranges;
use crate::domains::cat::filter::{CatFilterState, FilterCounts, MatchMode, TalentFilterMode};
use crate::domains::cat::game::stats::apply_level;
use crate::domains::cat::game::talents::apply_talent_stats;
use crate::domains::cat::scanner::CatEntry;

pub fn entity_passes_filter(cat: &CatEntry, filter: &CatFilterState) -> bool {
    let any_rarity_selected = filter.rarities.iter().any(|&is_selected| is_selected);

    if any_rarity_selected {
        let rarity_index = cat.unitbuy.rarity as usize;
        if rarity_index >= filter.rarities.len() || !filter.rarities[rarity_index] {
            return false;
        }
    }

    let any_form_selected = filter.forms.iter().any(|&is_selected| is_selected);
    let mut forms_to_check = Vec::new();

    for form_index in 0..4 {
        if !cat.forms[form_index] { continue; }
        if any_form_selected && !filter.forms[form_index] { continue; }
        forms_to_check.push(form_index);
    }

    if forms_to_check.is_empty() {
        return false;
    }

    let require_normal_talents = filter.talent_mode == TalentFilterMode::Only;
    let require_ultra_talents = filter.ultra_talent_mode == TalentFilterMode::Only;
    let has_stat_filters = filter.stat_ranges.values().any(|range_input| !range_input.min.is_empty() || !range_input.max.is_empty());
    let has_icon_filters = !filter.active_icons.is_empty();

    if !has_stat_filters && !has_icon_filters && !require_normal_talents && !require_ultra_talents {
        return true;
    }

    for &form_index in &forms_to_check {
        if evaluate_single_form(cat, form_index, filter, require_normal_talents, require_ultra_talents, has_stat_filters, has_icon_filters) {
            return true;
        }
    }

    false
}

fn evaluate_single_form(
    cat: &CatEntry,
    form_index: usize,
    filter: &CatFilterState,
    require_normal: bool,
    require_ultra: bool,
    has_stat_filters: bool,
    has_icon_filters: bool
) -> bool {
    let Some(Some(raw_stats)) = cat.stats.get(form_index) else {
        return false;
    };

    if !has_stat_filters && !has_icon_filters {
        return check_talent_presence_only(cat, form_index, require_normal, require_ultra);
    }

    if !check_talent_presence_only(cat, form_index, require_normal, require_ultra) {
        return false;
    }

    trace!(cat_id = cat.id, form = form_index, "evaluating entity form filter tree");

    let filter_level = filter.level_input.parse::<i32>().unwrap_or(50);
    let base_leveled = apply_level(raw_stats, cat.curve.as_ref(), filter_level);

    let mut state_normal = base_leveled.clone();
    let mut state_ultra = base_leveled.clone();
    let mut stats_min = base_leveled.clone();
    let mut stats_max = base_leveled.clone();

    let has_talent_data = form_index >= 2 && cat.talent_data.is_some();

    if has_talent_data
        && let Some(talent_data) = cat.talent_data.as_ref() {
        let mut min_levels = HashMap::new();
        let mut max_levels = HashMap::new();
        let mut normal_map = HashMap::new();
        let mut ultra_map = HashMap::new();

        for (talent_index, talent_group) in talent_data.groups.iter().enumerate() {
            let is_ultra_talent = talent_group.limit == 1;
            let current_mode = if is_ultra_talent { filter.ultra_talent_mode } else { filter.talent_mode };
            let talent_id_key = talent_index as u8;

            if current_mode == TalentFilterMode::Only {
                min_levels.insert(talent_id_key, talent_group.max_level);
                max_levels.insert(talent_id_key, talent_group.max_level);
            } else if current_mode == TalentFilterMode::Consider {
                max_levels.insert(talent_id_key, talent_group.max_level);
            }

            if is_ultra_talent {
                ultra_map.insert(talent_id_key, talent_group.max_level);
            } else {
                normal_map.insert(talent_id_key, talent_group.max_level);
                ultra_map.insert(talent_id_key, talent_group.max_level);
            }
        }

        state_normal = apply_talent_stats(&base_leveled, talent_data, &normal_map);
        state_ultra = apply_talent_stats(&base_leveled, talent_data, &ultra_map);
        stats_min = apply_talent_stats(&base_leveled, talent_data, &min_levels);
        stats_max = apply_talent_stats(&base_leveled, talent_data, &max_levels);
    }

    let mut counts = FilterCounts::default();

    if has_stat_filters {
        evaluate_stat_ranges(cat, form_index, filter, &stats_min, &stats_max, &mut counts);
    }

    if has_icon_filters {
        let builds = TalentBuilds { base: &base_leveled, normal: &state_normal, ultra: &state_ultra };
        evaluate_icon_requirements(cat, form_index, filter, &builds, &mut counts);
    }

    if counts.active == 0 {
        return true;
    }

    if filter.match_mode == MatchMode::And {
        return counts.failed == 0;
    }

    counts.passed > 0
}

fn check_talent_presence_only(cat: &CatEntry, form_index: usize, require_normal: bool, require_ultra: bool) -> bool {
    if !require_normal && !require_ultra {
        return true;
    }

    let mut has_any_normal = false;
    let mut has_any_ultra = false;

    if form_index >= 2
        && let Some(talent_data) = cat.talent_data.as_ref() {
        for talent_group in &talent_data.groups {
            if talent_group.limit == 1 {
                has_any_ultra = true;
            } else {
                has_any_normal = true;
            }
        }
    }

    if require_normal && require_ultra {
        return has_any_normal || has_any_ultra;
    }

    if require_normal {
        return has_any_normal;
    }

    has_any_ultra
}