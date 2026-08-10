use nyanko::cat::abilities::{get_talent, AttrUnit, Identity, REGISTRY};
use nyanko::common::data::img015;
use tracing::trace;

use crate::common::game::{AbilityGroups, AbilityItem, CustomIcon};
use crate::modules::cat::game::registry::{self, AbilityIcon, DisplayGroup};
use crate::modules::cat::game::talents;
use crate::modules::cat::game::CatRenderContext;

pub fn collect_ability_data(ctx: &CatRenderContext) -> AbilityGroups {
    trace!("collecting ability rendering data");

    let mut group_trait = Vec::new();
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    let get_talent_border = |ability_id: u8| -> Option<usize> {
        if ability_id == 0 { return None; }
        if let (Some(data), Some(levels)) = (ctx.talent_data, ctx.talent_levels) {
            let check_id = |target_id: u8| -> Option<usize> {
                if let Some((idx, group)) = data.groups.iter().enumerate().find(|(_, g)| g.ability_id == target_id) {
                    let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
                    if lv > 0 {
                        let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
                        return Some(if lv >= effective_max { img015::BORDER_GOLD } else { img015::BORDER_RED });
                    }
                }
                None
            };

            if let Some(border) = check_id(ability_id) { return Some(border); }
            if ability_id == 23 && let Some(border) = check_id(48) { return Some(border); }

            if is_trait_id(ability_id) {
                for (idx, group) in data.groups.iter().enumerate() {
                    let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
                    if lv > 0
                        && enables_trait(group.name_id, data.type_id, ability_id) {
                        return Some(img015::BORDER_GOLD);
                    }
                }
            }
        }
        None
    };

    let target_label = if ctx.is_conjure_unit { "Enemies" } else { "Target Traits" };

    for pure_def in REGISTRY {
        let display_def = registry::get_display_def(pure_def.identity);

        if display_def.group == DisplayGroup::Hidden { continue; }

        if ctx.is_conjure_unit {
            if display_def.group == DisplayGroup::Trait || display_def.group == DisplayGroup::Headline1 { continue; }
            if matches!(pure_def.identity, Identity::Dodge | Identity::ImmuneBossWave | Identity::Conjure | Identity::Kamikaze | Identity::SingleAttack | Identity::AreaAttack) {
                continue;
            }
        }

        let attrs = (pure_def.attributes)(ctx.final_stats);

        if !attrs.is_empty() {
            let val = attrs.first().map(|(_, v, _)| *v).unwrap_or(0);
            let dur = attrs.iter().find(|(_, _, u)| *u == AttrUnit::Frames).map(|(_, v, _)| *v).unwrap_or(0);

            let text = (display_def.formatter)(val, ctx.final_stats, target_label, dur, ctx.global.param);
            let border = get_talent_border(pure_def.talent_id);

            let (mut final_icon, final_custom) = match display_def.icon {
                AbilityIcon::Standard(id) => (Some(id), CustomIcon::None),
                AbilityIcon::Custom(c) => (None, c),
                AbilityIcon::None => (None, CustomIcon::None),
            };

            if pure_def.identity == Identity::WaveAttack && ctx.final_stats.mini_wave_flag > 0 {
                final_icon = Some(img015::ICON_MINI_WAVE);
            } else if pure_def.identity == Identity::SurgeAttack && ctx.final_stats.mini_surge_flag > 0 {
                final_icon = Some(img015::ICON_MINI_SURGE);
            }

            let item = AbilityItem { icon_id: final_icon, text, custom_icon: final_custom, border_id: border };

            match display_def.group {
                DisplayGroup::Trait => group_trait.push(item),
                DisplayGroup::Headline1 => group_headline_1.push(item),
                DisplayGroup::Headline2 => group_headline_2.push(item),
                DisplayGroup::Body1 => group_body_1.push(item),
                DisplayGroup::Body2 => group_body_2.push(item),
                DisplayGroup::Footer => group_footer.push(item),
                DisplayGroup::Hidden => {},
            }
        }
    }

    if let (Some(t_data), Some(levels)) = (ctx.talent_data, ctx.talent_levels) {
        let mut talent_headline = Vec::new();

        for (idx, group) in t_data.groups.iter().enumerate() {
            let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
            if lv == 0 { continue; }

            if let Some(pure_def) = get_talent(group.ability_id) {
                let display_def = registry::get_display_def(pure_def.identity);

                let (final_icon, final_custom) = match display_def.icon {
                    AbilityIcon::Standard(id) => (Some(id), CustomIcon::None),
                    AbilityIcon::Custom(c) => (None, c),
                    AbilityIcon::None => (None, CustomIcon::None),
                };

                match group.ability_id {
                    25 | 26 | 27 | 31 | 32 | 61 | 82 => {
                        if let Some(text) = talents::calculate_talent_display(group, ctx.base_stats, lv, ctx.level_curve, ctx.current_level) {
                            let item = AbilityItem { icon_id: final_icon, text, custom_icon: final_custom, border_id: get_talent_border(pure_def.talent_id) };
                            talent_headline.push(item);
                        }
                    },
                    18 | 19 | 20 | 21 | 22 | 24 | 30 | 52 | 54 => {
                        let val = talents::calculate_talent_value(group.min_1, group.max_1, lv, group.max_level);
                        let text = (display_def.formatter)(val, ctx.final_stats, target_label, 0, ctx.global.param);
                        let item = AbilityItem { icon_id: final_icon, text, custom_icon: final_custom, border_id: get_talent_border(pure_def.talent_id) };
                        group_footer.push(item);
                    },
                    _ => {}
                }
            }
        }

        group_headline_2.append(&mut talent_headline);
    }

    (group_trait, group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}

fn is_trait_id(id: u8) -> bool {
    (33..=41).contains(&id) || id == 57
}

fn enables_trait(name_id: i16, type_id: u16, target_id: u8) -> bool {
    let bit_idx = match target_id {
        33 => 0, 34 => 1, 35 => 2, 36 => 3, 37 => 4, 38 => 5, 39 => 6, 40 => 7, 41 => 8, 57 => 11,
        _ => return false,
    };
    if name_id == bit_idx { return true; }
    if type_id > 0 && (type_id & (1 << bit_idx)) != 0 { return true; }
    false
}