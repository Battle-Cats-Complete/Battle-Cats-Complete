use std::collections::HashMap;

use nyanko::combat::{Ability, Entity, REGISTRY};
use tracing::trace;

use crate::domains::cat::filter::{CatFilterState, FilterCounts, RangeInput, TalentFilterMode};
use crate::domains::cat::scanner::CatEntry;
use crate::systems::combat::comparable;
use crate::systems::combat::registry::{get_display_def, AbilityIcon};

fn definition_for_icon(icon: &AbilityIcon) -> Option<&'static Ability> {
    REGISTRY.iter().find(|pure_definition| &get_display_def(pure_definition.identity).icon == icon)
}

pub fn get_icon_name(icon: &AbilityIcon) -> String {
    definition_for_icon(icon).map_or_else(
        || "Unknown".to_string(),
        |pure_definition| get_display_def(pure_definition.identity).name.to_string(),
    )
}

pub(crate) fn has_trait_or_ability(battle_stats: &Entity, icon: &AbilityIcon) -> bool {
    definition_for_icon(icon).is_some_and(|pure_definition| !(pure_definition.attributes)(battle_stats).is_empty())
}

pub(crate) struct TalentBuilds<'a> {
    pub(crate) base: &'a Entity,
    pub(crate) normal: &'a Entity,
    pub(crate) ultra: &'a Entity,
}

pub(crate) fn evaluate_icon_requirements(
    cat: &CatEntry,
    form_index: usize,
    filter: &CatFilterState,
    builds: &TalentBuilds<'_>,
    counts: &mut FilterCounts
) {
    trace!("evaluating icon filters");
    for target_icon in &filter.active_icons {
        counts.active += 1;

        let has_inherent_ability = has_trait_or_ability(builds.base, target_icon);
        let pure_ability_definition = definition_for_icon(target_icon);

        let mut has_normal_talent = false;
        let mut has_ultra_talent = false;

        if form_index >= 2
            && let Some(talent_data) = cat.talent_data.as_ref() {
            for talent_group in &talent_data.groups {
                let matches_target_icon = pure_ability_definition.is_some_and(|pure_def| {
                    pure_def.talent_id.is_some_and(|talent_id| talent_group.ability_id == talent_id || talent_group.name_id as u8 == talent_id)
                });

                if matches_target_icon {
                    if talent_group.limit == 1 {
                        has_ultra_talent = true;
                    } else {
                        has_normal_talent = true;
                    }
                }
            }
        }

        let is_inherent_valid = filter.talent_mode != TalentFilterMode::Only && filter.ultra_talent_mode != TalentFilterMode::Only && has_inherent_ability;
        let is_normal_valid = filter.talent_mode != TalentFilterMode::Ignore && has_normal_talent;
        let is_ultra_valid = filter.ultra_talent_mode != TalentFilterMode::Ignore && has_ultra_talent;

        let mut icon_condition_passed = false;

        if is_inherent_valid || is_normal_valid || is_ultra_valid {
            icon_condition_passed = check_advanced_ranges(target_icon, filter, builds, pure_ability_definition, is_inherent_valid, is_normal_valid, is_ultra_valid);
        }

        if icon_condition_passed {
            counts.passed += 1;
        } else {
            counts.failed += 1;
        }
    }
}

fn check_advanced_ranges(
    target_icon: &AbilityIcon,
    filter: &CatFilterState,
    builds: &TalentBuilds<'_>,
    pure_ability_definition: Option<&Ability>,
    is_inherent_valid: bool,
    is_normal_valid: bool,
    is_ultra_valid: bool
) -> bool {
    let Some(advanced_ranges_map) = filter.adv_ranges.get(target_icon) else {
        return true;
    };

    let mut test_build_states = Vec::new();
    if is_inherent_valid { test_build_states.push(builds.base); }
    if is_normal_valid { test_build_states.push(builds.normal); }
    if is_ultra_valid { test_build_states.push(builds.ultra); }

    for build_stats in test_build_states {
        if test_single_build_ranges(build_stats, pure_ability_definition, advanced_ranges_map) {
            return true;
        }
    }

    false
}

fn test_single_build_ranges(
    build_stats: &Entity,
    pure_ability_definition: Option<&Ability>,
    advanced_ranges_map: &HashMap<&'static str, RangeInput>
) -> bool {
    let active_attributes = pure_ability_definition.map(|pure_def| (pure_def.attributes)(build_stats)).unwrap_or_default();

    for (attribute_key, target_range) in advanced_ranges_map {
        let attribute_value = active_attributes.iter()
            .find(|(key, _, _)| key == attribute_key)
            .map_or(0, |(_, value, _)| comparable(*value));

        if target_range.min.parse::<i32>().is_ok_and(|minimum| attribute_value < minimum) {
            return false;
        }

        if target_range.max.parse::<i32>().is_ok_and(|maximum| attribute_value > maximum) {
            return false;
        }
    }

    true
}