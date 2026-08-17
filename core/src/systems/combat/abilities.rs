use nyanko::combat::{get_talent, AttrUnit, AttrValue, Faction, Identity, REGISTRY};
use nyanko::common::data::img015;
use tracing::trace;

use crate::systems::combat::{AbilityGroups, AbilityItem, CustomIcon};
use crate::domains::cat::game::talents;
use crate::systems::combat::registry::{self, AbilityIcon, DisplayGroup, FormatContext};
use crate::systems::combat::RenderContext;

const TRAIT: usize = 0;
const HEADLINE_1: usize = 1;
const HEADLINE_2: usize = 2;
const BODY_1: usize = 3;
const BODY_2: usize = 4;
const FOOTER: usize = 5;

pub fn collect_ability_data(ctx: &RenderContext<'_>) -> AbilityGroups {
    trace!("collecting ability rendering data");

    let faction = ctx.final_stats.faction;
    let mut groups: [Vec<AbilityItem>; 6] = Default::default();

    let target_label = match (faction, ctx.is_conjure_unit) {
        (Faction::Enemy, _) => "Cats",
        (Faction::Cat, true) => "Enemies",
        (Faction::Cat, false) => "Target Traits",
    };

    for pure_def in REGISTRY {
        let display_def = registry::get_display_def(pure_def.identity);
        let slot = slot(display_def.group);

        if ctx.is_conjure_unit {
            if slot == TRAIT || slot == HEADLINE_1 { continue; }
            if matches!(pure_def.identity, Identity::Dodge | Identity::ImmuneBossWave | Identity::Conjure | Identity::Kamikaze | Identity::SingleAttack | Identity::AreaAttack) {
                continue;
            }
        }

        let attrs = (pure_def.attributes)(ctx.final_stats);

        if attrs.is_empty() { continue; }

        let format_ctx = FormatContext {
            value: attrs.first().map_or(AttrValue::Finite(0), |(_, value, _)| *value),
            stats: ctx.final_stats,
            target: target_label,
            duration: attrs.iter().find(|(_, _, unit)| *unit == AttrUnit::Frames).map_or(0, |(_, value, _)| frames(*value)),
            magnification: ctx.magnification,
            param: ctx.global.param,
        };

        groups[slot].push(AbilityItem {
            icon_id: standard_icon(display_def.icon),
            text: (display_def.formatter)(&format_ctx),
            custom_icon: custom_icon(display_def.icon),
            border_id: pure_def.talent_id.and_then(|talent_id| talent_border(ctx, talent_id)),
        });
    }

    if let (Some(talent_data), Some(levels)) = (ctx.talent_data, ctx.talent_levels) {
        let mut talent_headline = Vec::new();

        for (index, group) in talent_data.groups.iter().enumerate() {
            let level = *levels.get(&(index as u8)).unwrap_or(&0);
            if level == 0 { continue; }

            let Some(pure_def) = get_talent(group.ability_id) else { continue; };

            let display_def = registry::get_display_def(pure_def.identity);
            let border_id = pure_def.talent_id.and_then(|talent_id| talent_border(ctx, talent_id));
            let icon_id = standard_icon(display_def.icon);
            let custom = custom_icon(display_def.icon);

            match group.ability_id {
                25 | 26 | 27 | 31 | 32 | 61 | 82 => {
                    if let Some(text) = talents::calculate_talent_display(group, ctx.base_stats, level, ctx.level_curve, ctx.current_level) {
                        talent_headline.push(AbilityItem { icon_id, text, custom_icon: custom, border_id });
                    }
                },
                18 | 19 | 20 | 21 | 22 | 24 | 30 | 52 | 54 => {
                    let format_ctx = FormatContext {
                        value: AttrValue::Finite(talents::calculate_talent_value(group.min_1, group.max_1, level, group.max_level)),
                        stats: ctx.final_stats,
                        target: target_label,
                        duration: 0,
                        magnification: ctx.magnification,
                        param: ctx.global.param,
                    };

                    let text = (display_def.formatter)(&format_ctx);
                    groups[FOOTER].push(AbilityItem { icon_id, text, custom_icon: custom, border_id });
                },
                _ => {}
            }
        }

        groups[HEADLINE_2].append(&mut talent_headline);
    }

    let [group_trait, headline_1, headline_2, body_1, body_2, footer] = groups;

    (group_trait, headline_1, headline_2, body_1, body_2, footer)
}

fn slot(group: DisplayGroup) -> usize {
    match group {
        DisplayGroup::Trait => TRAIT,
        DisplayGroup::Headline1 => HEADLINE_1,
        DisplayGroup::Headline2 => HEADLINE_2,
        DisplayGroup::Body1 => BODY_1,
        DisplayGroup::Body2 => BODY_2,
        DisplayGroup::Footer => FOOTER,
    }
}

fn frames(value: AttrValue) -> i32 {
    match value {
        AttrValue::Finite(amount) => amount,
        AttrValue::Infinite => 0,
    }
}

fn standard_icon(icon: AbilityIcon) -> Option<usize> {
    match icon {
        AbilityIcon::Standard(id) => Some(id),
        AbilityIcon::Custom(_) | AbilityIcon::None => None,
    }
}

fn custom_icon(icon: AbilityIcon) -> CustomIcon {
    match icon {
        AbilityIcon::Custom(custom) => custom,
        AbilityIcon::Standard(_) | AbilityIcon::None => CustomIcon::None,
    }
}

fn talent_border(ctx: &RenderContext<'_>, ability_id: u8) -> Option<usize> {
    let (data, levels) = (ctx.talent_data?, ctx.talent_levels?);

    let border_for = |target_id: u8| -> Option<usize> {
        let (index, group) = data.groups.iter().enumerate().find(|(_, group)| group.ability_id == target_id)?;
        let level = *levels.get(&(index as u8)).unwrap_or(&0);

        if level == 0 { return None; }

        let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
        Some(if level >= effective_max { img015::BORDER_GOLD } else { img015::BORDER_RED })
    };

    if let Some(border) = border_for(ability_id) { return Some(border); }
    if ability_id == 23 && let Some(border) = border_for(48) { return Some(border); }

    if !is_trait_id(ability_id) { return None; }

    data.groups.iter().enumerate().any(|(index, group)| {
        *levels.get(&(index as u8)).unwrap_or(&0) > 0 && enables_trait(group.name_id, data.type_id, ability_id)
    }).then_some(img015::BORDER_GOLD)
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

    type_id > 0 && (type_id & (1 << bit_idx)) != 0
}
