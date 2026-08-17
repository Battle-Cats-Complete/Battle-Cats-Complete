use std::collections::HashMap;

use nyanko::cat::unit::{LevelCurve, Talent, TalentCost, TalentGroup};
use nyanko::combat::{get_talent, Ability, Attribute, AttrUnit, Entity, Faction};

use crate::domains::cat::game::stats;
use crate::systems::combat::comparable;
use crate::systems::combat::registry::{get_display_def, StatContext, CAT_STATS_REGISTRY};

pub(crate) fn calculate_talent_value(minimum: u16, maximum: u16, level: u8, max_level: u8) -> i32 {
    if level == 0 { return 0; }
    if max_level <= 1 { return minimum as i32; }
    if level == 1 { return minimum as i32; }
    if level == max_level { return maximum as i32; }

    let min_float = minimum as f32;
    let max_float = maximum as f32;
    let level_float = level as f32;
    let max_level_float = max_level as f32;

    let calculated_value = min_float + (max_float - min_float) * (level_float - 1.0) / (max_level_float - 1.0);
    calculated_value.round() as i32
}

pub fn calculate_talent_display(
    talent_group: &TalentGroup,
    base_stats: &Entity,
    talent_level: u8,
    level_curve: Option<&LevelCurve>,
    unit_level: i32
) -> Option<String> {
    let pure_definition = get_talent(talent_group.ability_id)?;
    let display_definition = get_display_def(pure_definition.identity, Faction::Cat);

    let leveled_base_stats = stats::apply_level(base_stats, level_curve, unit_level);
    let mut mutated_stats = leveled_base_stats.clone();
    let mut dummy_min_stats = leveled_base_stats.clone();
    let mut dummy_max_stats = leveled_base_stats.clone();

    let value_one = calculate_talent_value(talent_group.min_1, talent_group.max_1, talent_level, talent_group.max_level);
    let value_two = calculate_talent_value(talent_group.min_2, talent_group.max_2, talent_level, talent_group.max_level);

    if let Some(apply_talent_mutation) = pure_definition.apply_talent {
        if talent_level > 0 {
            apply_talent_mutation(&mut mutated_stats, value_one, value_two, talent_group);
        }

        let value_one_minimum = calculate_talent_value(talent_group.min_1, talent_group.max_1, 1, talent_group.max_level);
        let value_two_minimum = calculate_talent_value(talent_group.min_2, talent_group.max_2, 1, talent_group.max_level);
        apply_talent_mutation(&mut dummy_min_stats, value_one_minimum, value_two_minimum, talent_group);

        let value_one_maximum = calculate_talent_value(talent_group.min_1, talent_group.max_1, talent_group.max_level, talent_group.max_level);
        let value_two_maximum = calculate_talent_value(talent_group.min_2, talent_group.max_2, talent_group.max_level, talent_group.max_level);
        apply_talent_mutation(&mut dummy_max_stats, value_one_maximum, value_two_maximum, talent_group);
    }

    let maximum_attributes = (pure_definition.attributes)(&dummy_max_stats);

    if !maximum_attributes.is_empty() {
        return process_generic_attributes(pure_definition, &leveled_base_stats, &mutated_stats, &dummy_min_stats, &maximum_attributes);
    }

    if display_definition.name.starts_with("Resist ") {
        if value_one == 0 {
            let value_one_minimum = calculate_talent_value(talent_group.min_1, talent_group.max_1, 1, talent_group.max_level);
            let value_one_maximum = calculate_talent_value(talent_group.min_1, talent_group.max_1, talent_group.max_level, talent_group.max_level);

            if value_one_minimum == value_one_maximum {
                return Some(format!("Resist: {}%", value_one_minimum));
            }
            return Some(format!("Resist: 0% (+{}%) -> 0%", value_one));
        }
        return Some(format!("Resist: 0% (+{}%) -> {}%", value_one, value_one));
    }

    let target_stat_definition = CAT_STATS_REGISTRY.iter().find(|stat_definition| stat_definition.linked_talent_id == Some(talent_group.ability_id));

    if let Some(stat_definition) = target_stat_definition {
        let old_stat_value = (stat_definition.get_value)(&StatContext::cat(&leveled_base_stats, 0, None));
        let new_stat_value = (stat_definition.get_value)(&StatContext::cat(&mutated_stats, 0, None));

        let value_one_minimum = calculate_talent_value(talent_group.min_1, talent_group.max_1, 1, talent_group.max_level);
        let value_one_maximum = calculate_talent_value(talent_group.min_1, talent_group.max_1, talent_group.max_level, talent_group.max_level);

        if value_one_minimum == value_one_maximum {
            let value_two_minimum = calculate_talent_value(talent_group.min_2, talent_group.max_2, 1, talent_group.max_level);

            let (displayed_stats, modifier_one, modifier_two) = if talent_level == 0 {
                (&leveled_base_stats, 0, 0)
            } else {
                (&dummy_min_stats, value_one_minimum, value_two_minimum)
            };

            let displayed_value = (stat_definition.get_value)(&StatContext::cat(displayed_stats, 0, None));
            let modifier_string = stat_definition.talent_modifier_fmt.map(|format_func| format_func(modifier_one, modifier_two)).unwrap_or_default();

            return Some(if modifier_string.is_empty() {
                format!("{}: {}", stat_definition.display_name, (stat_definition.formatter)(displayed_value))
            } else {
                format!("{}: {} {}", stat_definition.display_name, (stat_definition.formatter)(displayed_value), modifier_string)
            });
        }

        let old_string_format = (stat_definition.formatter)(old_stat_value);
        let new_string_format = (stat_definition.formatter)(new_stat_value);
        let modifier_string = stat_definition.talent_modifier_fmt.map(|format_func| format_func(value_one, value_two)).unwrap_or_default();

        return Some(format!("{}: {} {} -> {}", stat_definition.display_name, old_string_format, modifier_string, new_string_format));
    }

    None
}

fn process_generic_attributes(
    pure_definition: &Ability,
    leveled_base_stats: &Entity,
    mutated_stats: &Entity,
    dummy_min_stats: &Entity,
    maximum_attributes: &[Attribute]
) -> Option<String> {
    let mut strings_changed = Vec::new();
    let mut strings_unchanged = Vec::new();
    let mut handled_attribute_keys = std::collections::HashSet::new();

    let old_attributes = (pure_definition.attributes)(leveled_base_stats);
    let new_attributes = (pure_definition.attributes)(mutated_stats);
    let min_attributes = (pure_definition.attributes)(dummy_min_stats);

    let extract_value = |target_key: &str, attributes_list: &[Attribute]| -> i32 {
        attributes_list.iter().find(|(key, _, _)| *key == target_key).map_or(0, |(_, value, _)| comparable(*value))
    };

    if maximum_attributes.iter().any(|(key, _, _)| *key == "Active") {
        let old_active_value = extract_value("Active", &old_attributes);
        let new_active_value = extract_value("Active", &new_attributes);
        let string_old = if old_active_value > 0 { "Active" } else { "Inactive" };
        let string_new = if new_active_value > 0 { "Active" } else { "Inactive" };

        strings_changed.push(format!("{} -> {}", string_old, string_new));
        handled_attribute_keys.insert("Active");
    }

    let snapshots = AttrSnapshots {
        old: &old_attributes,
        new: &new_attributes,
        min: &min_attributes,
        max: maximum_attributes,
    };

    for &(attribute_key, attribute_unit) in pure_definition.schema {
        if handled_attribute_keys.contains(attribute_key) { continue; }

        if attribute_key.starts_with("Min ") {
            process_range_attribute(attribute_key, pure_definition, &snapshots, &mut handled_attribute_keys, &mut strings_changed, &mut strings_unchanged);
            continue;
        }

        process_single_attribute(attribute_key, attribute_unit, &snapshots, &mut handled_attribute_keys, &mut strings_changed, &mut strings_unchanged);
    }

    let mut final_display_strings = strings_changed;
    final_display_strings.extend(strings_unchanged);

    if final_display_strings.is_empty() {
        return None;
    }

    Some(final_display_strings.join("\n"))
}

struct AttrSnapshots<'a> {
    old: &'a [Attribute],
    new: &'a [Attribute],
    min: &'a [Attribute],
    max: &'a [Attribute],
}

fn process_range_attribute(
    attribute_key: &'static str,
    pure_definition: &Ability,
    snapshots: &AttrSnapshots<'_>,
    handled_attribute_keys: &mut std::collections::HashSet<&'static str>,
    strings_changed: &mut Vec<String>,
    strings_unchanged: &mut Vec<String>
) {
    let suffix = &attribute_key[4..];
    let maximum_key_string = format!("Max {}", suffix);

    let extract_value = |target_key: &str, attributes_list: &[Attribute]| -> i32 {
        attributes_list.iter().find(|(key, _, _)| *key == target_key).map_or(0, |(_, value, _)| comparable(*value))
    };

    let Some(&(maximum_key, _)) = pure_definition.schema.iter().find(|(schema_key, _)| *schema_key == maximum_key_string.as_str()) else {
        return;
    };

    handled_attribute_keys.insert(attribute_key);
    handled_attribute_keys.insert(maximum_key);

    let old_minimum = extract_value(attribute_key, snapshots.old);
    let new_minimum = extract_value(attribute_key, snapshots.new);
    let absolute_min_minimum = extract_value(attribute_key, snapshots.min);
    let absolute_max_minimum = extract_value(attribute_key, snapshots.max);

    let old_maximum = extract_value(maximum_key, snapshots.old);
    let new_maximum = extract_value(maximum_key, snapshots.new);
    let absolute_min_maximum = extract_value(maximum_key, snapshots.min);
    let absolute_max_maximum = extract_value(maximum_key, snapshots.max);

    let is_scalable = absolute_min_minimum != absolute_max_minimum || absolute_min_maximum != absolute_max_maximum;

    let format_range = |min_val, max_val| {
        if min_val == max_val { format!("{}", min_val) } else { format!("{}~{}", min_val, max_val) }
    };

    if !is_scalable {
        strings_unchanged.push(format!("{}: {}", suffix, format_range(absolute_min_minimum, absolute_min_maximum)));
        return;
    }

    let delta_minimum = new_minimum - old_minimum;
    let delta_maximum = new_maximum - old_maximum;

    let difference_string = if delta_minimum == delta_maximum {
        let sign = if delta_minimum >= 0 { "+" } else { "" };
        format!("({}{})", sign, delta_minimum)
    } else {
        let sign_minimum = if delta_minimum >= 0 { "+" } else { "" };
        let sign_maximum = if delta_maximum >= 0 { "+" } else { "" };
        format!("({}{}~{}{})", sign_minimum, delta_minimum, sign_maximum, delta_maximum)
    };

    strings_changed.push(format!("{}: {} {} -> {}", suffix, format_range(old_minimum, old_maximum), difference_string, format_range(new_minimum, new_maximum)));
}

fn process_single_attribute(
    attribute_key: &'static str,
    attribute_unit: AttrUnit,
    snapshots: &AttrSnapshots<'_>,
    handled_attribute_keys: &mut std::collections::HashSet<&'static str>,
    strings_changed: &mut Vec<String>,
    strings_unchanged: &mut Vec<String>
) {
    let extract_value = |target_key: &str, attributes_list: &[Attribute]| -> i32 {
        attributes_list.iter().find(|(key, _, _)| *key == target_key).map_or(0, |(_, value, _)| comparable(*value))
    };

    let old_value = extract_value(attribute_key, snapshots.old);
    let new_value = extract_value(attribute_key, snapshots.new);
    let absolute_minimum_value = extract_value(attribute_key, snapshots.min);
    let absolute_maximum_value = extract_value(attribute_key, snapshots.max);

    let is_scalable = absolute_minimum_value != absolute_maximum_value;

    let format_value = |value| match attribute_unit {
        AttrUnit::Percent => format!("{}%", value),
        AttrUnit::Frames => format!("{}f", value),
        AttrUnit::Range | AttrUnit::None => format!("{}", value),
    };

    handled_attribute_keys.insert(attribute_key);

    if !is_scalable {
        strings_unchanged.push(format!("{}: {}", attribute_key, format_value(absolute_minimum_value)));
        return;
    }

    let delta_value = new_value - old_value;
    let prefix_sign = if delta_value >= 0 { "+" } else { "" };

    let difference_string = match attribute_unit {
        AttrUnit::Percent => format!("({}{}%)", prefix_sign, delta_value),
        AttrUnit::Frames => format!("({}{}f)", prefix_sign, delta_value),
        AttrUnit::Range | AttrUnit::None => format!("({}{})", prefix_sign, delta_value),
    };

    strings_changed.push(format!("{}: {} {} -> {}", attribute_key, format_value(old_value), difference_string, format_value(new_value)));
}

fn apply_target_traits(battle_stats: &mut Entity, target_name_id: i16, bitmask_type_id: u16) {
    let mut apply_trait_bit = |bit_index: u16| {
        match bit_index {
            0 => battle_stats.trait_red = 1,
            1 => battle_stats.trait_floating = 1,
            2 => battle_stats.trait_dark = 1,
            3 => battle_stats.trait_metal = 1,
            4 => battle_stats.trait_angel = 1,
            5 => battle_stats.trait_alien = 1,
            6 => battle_stats.trait_zombie = 1,
            7 => battle_stats.trait_relic = 1,
            8 => battle_stats.trait_traitless = 1,
            9 => battle_stats.trait_witch = 1,
            10 => battle_stats.trait_eva = 1,
            11 => battle_stats.trait_aku = 1,
            _ => {}
        }
    };

    if (0..=11).contains(&target_name_id) {
        apply_trait_bit(target_name_id as u16);
    }

    if bitmask_type_id > 0 {
        for bit_index in 0..=11 {
            if (bitmask_type_id & (1 << bit_index)) != 0 {
                apply_trait_bit(bit_index);
            }
        }
    }
}

pub(crate) fn apply_talent_stats(base_stats: &Entity, talent_data: &Talent, talent_levels: &HashMap<u8, u8>) -> Entity {
    let mut mutated_stats = base_stats.clone();

    for (talent_index, talent_group) in talent_data.groups.iter().enumerate() {
        let current_level = *talent_levels.get(&(talent_index as u8)).unwrap_or(&0);

        if current_level > 0 && talent_group.name_id != -1 {
            apply_target_traits(&mut mutated_stats, talent_group.name_id, talent_data.type_id);
        }

        if current_level == 0 { continue; }

        let value_one = calculate_talent_value(talent_group.min_1, talent_group.max_1, current_level, talent_group.max_level);
        let value_two = calculate_talent_value(talent_group.min_2, talent_group.max_2, current_level, talent_group.max_level);

        if let Some(pure_definition) = get_talent(talent_group.ability_id)
            && let Some(apply_talent_mutation) = pure_definition.apply_talent {
            apply_talent_mutation(&mut mutated_stats, value_one, value_two, talent_group);
        }
    }
    mutated_stats
}

pub fn get_talent_np_cost(cost_id: u8, current_level: u8, costs_map: &HashMap<u8, TalentCost>) -> i32 {
    if current_level == 0 { return 0; }

    let Some(cost_data) = costs_map.get(&cost_id) else {
        return 0;
    };

    let level_limit = (current_level as usize).min(cost_data.costs.len());
    let mut total_cost = 0;

    for level_index in 0..level_limit {
        total_cost += cost_data.costs[level_index] as i32;
    }

    total_cost
}

pub fn get_total_np_cost(talent_data: &Talent, talent_levels: &HashMap<u8, u8>, costs_map: &HashMap<u8, TalentCost>) -> i32 {
    talent_data.groups.iter().enumerate()
        .map(|(index, group)| {
            let current_level = talent_levels.get(&(index as u8)).copied().unwrap_or(0);
            get_talent_np_cost(group.cost_id, current_level, costs_map)
        })
        .sum()
}
