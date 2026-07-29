use std::collections::HashMap;

use nyanko::cat::abilities::{Ability, REGISTRY};
use nyanko::cat::unit::Battle;
use tracing::trace;

use crate::modules::cat::filter::{CatFilterState, RangeInput, TalentFilterMode};
use crate::modules::cat::game::registry::{get_display_def, AbilityIcon};
use crate::modules::cat::scanner::CatEntry;

pub fn get_icon_name(icon: &AbilityIcon) -> String {
    for pure_definition in REGISTRY {
        let display_definition = get_display_def(pure_definition.identity);
        if &display_definition.icon == icon {
            return display_definition.name.to_string();
        }
    }
    "Unknown".to_string()
}

pub(crate) fn has_trait_or_ability(battle_stats: &Battle, icon: &AbilityIcon) -> bool {
    for pure_definition in REGISTRY {
        let display_definition = get_display_def(pure_definition.identity);
        if &display_definition.icon == icon {
            return !(pure_definition.attributes)(battle_stats).is_empty();
        }
    }
    false
}

pub(crate) fn evaluate_icon_requirements(
    cat: &CatEntry,
    form_index: usize,
    filter: &CatFilterState,
    base_leveled: &Battle,
    state_normal: &Battle,
    state_ultra: &Battle,
    active_conditions: &mut i32,
    passed_conditions: &mut i32,
    failed_conditions: &mut i32
) {
    trace!("evaluating icon filters");
    for target_icon in &filter.active_icons {
        *active_conditions += 1;

        let has_inherent_ability = has_trait_or_ability(base_leveled, target_icon);
        let pure_ability_definition = REGISTRY.iter().find(|pure_def| &get_display_def(pure_def.identity).icon == target_icon);

        let mut has_normal_talent = false;
        let mut has_ultra_talent = false;

        if form_index >= 2
            && let Some(talent_data) = cat.talent_data.as_ref() {
            for talent_group in &talent_data.groups {
                let matches_target_icon = pure_ability_definition.is_some_and(|pure_def| {
                    talent_group.ability_id == pure_def.talent_id || talent_group.name_id as u8 == pure_def.talent_id
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
            icon_condition_passed = check_advanced_ranges(target_icon, filter, base_leveled, state_normal, state_ultra, pure_ability_definition, is_inherent_valid, is_normal_valid, is_ultra_valid);
        }

        if icon_condition_passed {
            *passed_conditions += 1;
        } else {
            *failed_conditions += 1;
        }
    }
}

fn check_advanced_ranges(
    target_icon: &AbilityIcon,
    filter: &CatFilterState,
    base_leveled: &Battle,
    state_normal: &Battle,
    state_ultra: &Battle,
    pure_ability_definition: Option<&Ability>,
    is_inherent_valid: bool,
    is_normal_valid: bool,
    is_ultra_valid: bool
) -> bool {
    let Some(advanced_ranges_map) = filter.adv_ranges.get(target_icon) else {
        return true;
    };

    let mut test_build_states = Vec::new();
    if is_inherent_valid { test_build_states.push(base_leveled); }
    if is_normal_valid { test_build_states.push(state_normal); }
    if is_ultra_valid { test_build_states.push(state_ultra); }

    for build_stats in test_build_states {
        if test_single_build_ranges(build_stats, pure_ability_definition, advanced_ranges_map) {
            return true;
        }
    }

    false
}

fn test_single_build_ranges(
    build_stats: &Battle,
    pure_ability_definition: Option<&Ability>,
    advanced_ranges_map: &HashMap<&'static str, RangeInput>
) -> bool {
    let active_attributes = pure_ability_definition.map(|pure_def| (pure_def.attributes)(build_stats)).unwrap_or_default();

    for (attribute_key, target_range) in advanced_ranges_map {
        let attribute_value = active_attributes.iter()
            .find(|(key, _, _)| key == attribute_key)
            .map(|(_, value, _)| *value)
            .unwrap_or(0);

        if let Ok(minimum_required) = target_range.min.parse::<i32>()
            && attribute_value < minimum_required {
            return false;
        }

        if let Ok(maximum_required) = target_range.max.parse::<i32>()
            && attribute_value > maximum_required {
            return false;
        }
    }

    true
}