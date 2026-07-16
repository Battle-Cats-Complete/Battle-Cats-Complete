use std::collections::{HashMap, HashSet};

use nyanko::cat::abilities::REGISTRY;
use nyanko::cat::unit::{Battle, UnitBuy};
use nyanko::common::data::img015;

use crate::modules::cat::registry::{get_display_def, AbilityIcon, CAT_STATS_REGISTRY};
use crate::common::game::abilities::CustomIcon;

use super::scanner::CatEntry;
use super::talents::apply_talent_stats;

pub const ATTACK_TYPE_ICONS: &[AbilityIcon] = &[
    AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
    AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
    AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
    AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
    AbilityIcon::Custom(CustomIcon::Multihit),
];

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TalentFilterMode {
    #[default]
    Ignore,
    Consider,
    Only,
}

impl TalentFilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            TalentFilterMode::Ignore => "Ignore",
            TalentFilterMode::Consider => "Consider",
            TalentFilterMode::Only => "Only",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum MatchMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, PartialEq, Default)]
pub struct RangeInput {
    pub min: String,
    pub max: String,
}

#[derive(Clone, PartialEq)]
pub struct CatFilterState {
    pub is_open: bool,
    pub active_icons: HashSet<AbilityIcon>,
    pub rarities: [bool; 6],
    pub forms: [bool; 4],
    pub match_mode: MatchMode,
    pub talent_mode: TalentFilterMode,
    pub ultra_talent_mode: TalentFilterMode,
    pub adv_ranges: HashMap<AbilityIcon, HashMap<&'static str, RangeInput>>,
    pub level_input: String,
    pub stat_ranges: HashMap<&'static str, RangeInput>,
}

impl Default for CatFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_icons: HashSet::new(),
            rarities: [false; 6],
            forms: [false; 4],
            match_mode: MatchMode::And,
            talent_mode: TalentFilterMode::Ignore,
            ultra_talent_mode: TalentFilterMode::Ignore,
            adv_ranges: HashMap::new(),
            level_input: String::new(),
            stat_ranges: HashMap::new(),
        }
    }
}

impl CatFilterState {
    pub fn is_active(&self) -> bool {
        let has_icons = !self.active_icons.is_empty();
        let has_rarities = self.rarities.iter().any(|&is_selected| is_selected);
        let has_forms = self.forms.iter().any(|&is_selected| is_selected);
        let normal_talent_required = self.talent_mode == TalentFilterMode::Only;
        let ultra_talent_required = self.ultra_talent_mode == TalentFilterMode::Only;
        let has_stat_ranges = self.stat_ranges.values().any(|range_input| !range_input.min.is_empty() || !range_input.max.is_empty());

        has_icons || has_rarities || has_forms || normal_talent_required || ultra_talent_required || has_stat_ranges
    }
}

pub fn get_stat_value(battle_stats: &Battle, stat_name: &str, animation_frames: i32, unitbuy: Option<&UnitBuy>) -> i32 {
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

pub fn get_icon_name(icon: &AbilityIcon) -> String {
    for pure_definition in REGISTRY {
        let display_definition = get_display_def(pure_definition.identity);
        if &display_definition.icon == icon {
            return display_definition.name.to_string();
        }
    }
    "Unknown".to_string()
}

pub fn has_trait_or_ability(battle_stats: &Battle, icon: &AbilityIcon) -> bool {
    for pure_definition in REGISTRY {
        let display_definition = get_display_def(pure_definition.identity);
        if &display_definition.icon == icon {
            return !(pure_definition.attributes)(battle_stats).is_empty();
        }
    }
    false
}

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

    let filter_level = filter.level_input.parse::<i32>().unwrap_or(50);
    let base_leveled = crate::modules::cat::logic::stats::apply_level(raw_stats, cat.curve.as_ref(), filter_level);

    let mut state_normal = base_leveled.clone();
    let mut state_ultra = base_leveled.clone();
    let mut stats_min = base_leveled.clone();
    let mut stats_max = base_leveled.clone();

    let has_talent_data = form_index >= 2 && cat.talent_data.is_some();

    if has_talent_data {
        let talent_data = cat.talent_data.as_ref().unwrap();
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

    let mut active_conditions = 0;
    let mut passed_conditions = 0;
    let mut failed_conditions = 0;

    if has_stat_filters {
        evaluate_stat_ranges(cat, form_index, filter, &stats_min, &stats_max, &mut active_conditions, &mut passed_conditions, &mut failed_conditions);
    }

    if has_icon_filters {
        evaluate_icon_requirements(cat, form_index, filter, &base_leveled, &state_normal, &state_ultra, &mut active_conditions, &mut passed_conditions, &mut failed_conditions);
    }

    if active_conditions == 0 {
        return true;
    }

    if filter.match_mode == MatchMode::And {
        return failed_conditions == 0;
    }

    passed_conditions > 0
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

fn evaluate_stat_ranges(
    cat: &CatEntry,
    form_index: usize,
    filter: &CatFilterState,
    stats_min: &Battle,
    stats_max: &Battle,
    active_conditions: &mut i32,
    passed_conditions: &mut i32,
    failed_conditions: &mut i32
) {
    let animation_frames = cat.atk_anim_frames[form_index];
    let unitbuy_ref = Some(&cat.unitbuy);

    for (stat_name, target_range) in &filter.stat_ranges {
        if target_range.min.is_empty() && target_range.max.is_empty() { continue; }

        *active_conditions += 1;

        let value_a = get_stat_value(stats_min, stat_name, animation_frames, unitbuy_ref);
        let value_b = get_stat_value(stats_max, stat_name, animation_frames, unitbuy_ref);

        let actual_min = value_a.min(value_b);
        let actual_max = value_a.max(value_b);

        let required_min = target_range.min.parse::<i32>().unwrap_or(i32::MIN);
        let required_max = target_range.max.parse::<i32>().unwrap_or(i32::MAX);

        if actual_min <= required_max && actual_max >= required_min {
            *passed_conditions += 1;
        } else {
            *failed_conditions += 1;
        }
    }
}

fn evaluate_icon_requirements(
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
    pure_ability_definition: Option<&nyanko::cat::abilities::Ability>,
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
    pure_ability_definition: Option<&nyanko::cat::abilities::Ability>,
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