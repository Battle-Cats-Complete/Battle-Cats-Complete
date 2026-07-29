use std::cmp::Reverse;
use std::collections::HashMap;

use nyanko::common::data::{img015, Param};
use nyanko::enemy::abilities::{Identity, REGISTRY};
use nyanko::enemy::unit::Battle;
use serde::{Deserialize, Serialize};

use crate::common::game::CustomIcon;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Magnification {
    pub hitpoints: i32,
    pub attack: i32,
}

impl Default for Magnification {
    fn default() -> Self {
        Self { hitpoints: 100, attack: 100 }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum DisplayGroup {
    Type,
    Headline1,
    Headline2,
    Body1,
    Body2,
    Footer,
    Hidden,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum AbilityIcon {
    Standard(usize),
    Custom(CustomIcon),
    None,
}

pub struct EnemyAbilityDisplayDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub group: DisplayGroup,
    pub formatter: fn(primary_value: i32, stats: &Battle, duration_frames: i32, magnification: Magnification, param: &Param) -> String,
}


fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min_range: i32, max_range: i32) -> String {
    if min_range == max_range { format!("at {}", min_range) } else { format!("between {}~{}", min_range, max_range) }
}

fn fmt_compress(min_val: i32, max_val: i32) -> String {
    if min_val == max_val { format!("{}", min_val) } else { format!("{}~{}", min_val, max_val) }
}

fn fmt_count(count: i32) -> String {
    match count {
        -1 => "infinitely".to_string(),
        1 => "1 time".to_string(),
        _ => format!("{} times", count),
    }
}

fn fmt_effective_range(stats: &Battle) -> String {
    let primary_anchor = if stats.long_distance_anchor_1 != 0 {
        stats.long_distance_anchor_1
    } else {
        stats.standing_range
    };

    let mut range_strings = Vec::new();

    let has_ld_or_omni = (stats.long_distance_span_1 != 0 || stats.long_distance_anchor_1 != 0) ||
        (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span != 0 || stats.long_distance_2_anchor != 0)) ||
        (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span != 0 || stats.long_distance_3_anchor != 0));

    if has_ld_or_omni {
        let hit_data = [
            (true, stats.long_distance_anchor_1, stats.long_distance_span_1, 1),
            (stats.attack_2 > 0, stats.long_distance_2_anchor, stats.long_distance_2_span, stats.long_distance_2_flag),
            (stats.attack_3 > 0, stats.long_distance_3_anchor, stats.long_distance_3_span, stats.long_distance_3_flag),
        ];

        for (is_active, anchor, span, flag) in hit_data {
            if is_active {
                if flag > 0 && (span != 0 || anchor != 0) {
                    let start = anchor;
                    let end = anchor + span;
                    let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                    range_strings.push(format!("{}~{}", min_r, max_r));
                } else if stats.long_distance_span_1 != 0 || stats.long_distance_anchor_1 != 0 {
                    let start = stats.long_distance_anchor_1;
                    let end = stats.long_distance_anchor_1 + stats.long_distance_span_1;
                    let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                    range_strings.push(format!("{}~{}", min_r, max_r));
                } else {
                    range_strings.push(format!("{}~{}", -stats.hitbox_width, stats.standing_range));
                }
            }
        }
    }

    if range_strings.len() > 1 {
        let first_string = range_strings[0].clone();
        if range_strings.iter().all(|s| s == &first_string) {
            range_strings.truncate(1);
        }
    }

    let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
    format!("{} {}\nStands at {} Range relative to Cat Base", label_prefix, range_strings.join(" / "), primary_anchor)
}

fn fmt_multihit(stats: &Battle, magnification: Magnification) -> String {
    let magnification_factor = magnification.attack as f32 / 100.0;

    let damage_hit_1 = (stats.attack_1 as f32 * magnification_factor).round() as i32;
    let damage_hit_2 = (stats.attack_2 as f32 * magnification_factor).round() as i32;
    let damage_hit_3 = (stats.attack_3 as f32 * magnification_factor).round() as i32;

    let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };

    let ability_flag_3 = if stats.attack_3 == 0 {
        ""
    } else if stats.attack_3_abilities > 0 {
        " / True"
    } else {
        " / False"
    };

    let format_time = |frames: i32| -> String {
        format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
    };

    let damage_string = if stats.attack_3 > 0 {
        format!("{} / {} / {}", damage_hit_1, damage_hit_2, damage_hit_3)
    } else {
        format!("{} / {}", damage_hit_1, damage_hit_2)
    };

    let timing_string = if stats.attack_3 > 0 {
        format!("{} / {} / {}", format_time(stats.time_until_attack_1), format_time(stats.time_until_attack_2), format_time(stats.time_until_attack_3))
    } else {
        format!("{} / {}", format_time(stats.time_until_attack_1), format_time(stats.time_until_attack_2))
    };

    format!("Damage split {}\nTiming split {}\nAbility split {} / {}{}", damage_string, timing_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

fn fmt_sage(param: &Param) -> String {
    let mut resistance_groups_by_percentage: HashMap<i32, Vec<&str>> = HashMap::new();
    
    resistance_groups_by_percentage.entry(param.sage_type_resist_weaken).or_default().push("Weaken");
    resistance_groups_by_percentage.entry(param.sage_type_resist_freeze).or_default().push("Freeze");
    resistance_groups_by_percentage.entry(param.sage_type_resist_slow).or_default().push("Slow");
    resistance_groups_by_percentage.entry(param.sage_type_resist_curse).or_default().push("Curse");
    resistance_groups_by_percentage.entry(param.sage_type_resist_knockback).or_default().push("Knockback");

    let base_description = "Crowd Control effects inflicted upon Sage Enemies are reduced by";

    if resistance_groups_by_percentage.len() == 1
        && let Some((&percentage, _)) = resistance_groups_by_percentage.iter().next() {
        format!("{} {}%", base_description, percentage)
    } else {
        let mut formatted_resistance_lines = Vec::new();
        let mut sorted_resistance_groups: Vec<_> = resistance_groups_by_percentage.into_iter().collect();

        sorted_resistance_groups.sort_by_key(|group| Reverse(group.0));

        for (percentage, effect_names) in sorted_resistance_groups {
            let formatted_effect_list = match effect_names.len() {
                1 => effect_names[0].to_string(),
                2 => format!("{} and {}", effect_names[0], effect_names[1]),
                _ => match effect_names.split_last() {
                    Some((last_effect, leading_effects)) => format!("{}, and {}", leading_effects.join(", "), last_effect),
                    None => String::new(),
                }
            };
            formatted_resistance_lines.push(format!("{}% for {}", percentage, formatted_effect_list));
        }
        format!("{}\n{}", base_description, formatted_resistance_lines.join("\n"))
    }
}


pub fn get_display_def(identity: Identity) -> EnemyAbilityDisplayDef {
    match identity {
        Identity::TypeRed => EnemyAbilityDisplayDef {
            name: "Red",
            fallback: "Red",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RED),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Red".into(),
        },
        Identity::TypeFloating => EnemyAbilityDisplayDef {
            name: "Floating",
            fallback: "Float",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_FLOATING),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Floating".into(),
        },
        Identity::TypeDark => EnemyAbilityDisplayDef {
            name: "Dark",
            fallback: "Dark",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_BLACK),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Dark".into(),
        },
        Identity::TypeMetal => EnemyAbilityDisplayDef {
            name: "Metal",
            fallback: "Metal",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_METAL),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Metal".into(),
        },
        Identity::TypeAngel => EnemyAbilityDisplayDef {
            name: "Angel",
            fallback: "Angel",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ANGEL),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Angel".into(),
        },
        Identity::TypeAlien => EnemyAbilityDisplayDef {
            name: "Alien",
            fallback: "Alien",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ALIEN),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Alien".into(),
        },
        Identity::TypeZombie => EnemyAbilityDisplayDef {
            name: "Zombie",
            fallback: "Zomb",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ZOMBIE),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Zombie".into(),
        },
        Identity::TypeRelic => EnemyAbilityDisplayDef {
            name: "Relic",
            fallback: "Relic",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RELIC),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Relic".into(),
        },
        Identity::TypeAku => EnemyAbilityDisplayDef {
            name: "Aku",
            fallback: "Aku",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_AKU),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Aku".into(),
        },
        Identity::TypeTraitless => EnemyAbilityDisplayDef {
            name: "Traitless",
            fallback: "White",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_TRAITLESS),
            group: DisplayGroup::Type,
            formatter: |_,_,_,_,_| "Traitless".into(),
        },

        Identity::TypeDojo => EnemyAbilityDisplayDef {
            name: "Dojo",
            fallback: "Dojo",
            icon: AbilityIcon::Custom(CustomIcon::Dojo),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Dojo".into(),
        },
        Identity::TypeStarredAlien => EnemyAbilityDisplayDef {
            name: "Starred Alien",
            fallback: "Star",
            icon: AbilityIcon::Custom(CustomIcon::StarredAlien),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Starred Alien".into(),
        },
        Identity::TypeCatGod => EnemyAbilityDisplayDef {
            name: "Cat God",
            fallback: "God",
            icon: AbilityIcon::Custom(CustomIcon::God),
            group: DisplayGroup::Headline1,
            formatter: |type_val,_,_,_,_| format!("CotC {} Cat God", type_val - 1),
        },
        Identity::TypeColossus => EnemyAbilityDisplayDef {
            name: "Colossus",
            fallback: "Colos",
            icon: AbilityIcon::Standard(img015::ICON_COLOSSUS),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Colossus Enemy".into(),
        },
        Identity::TypeBehemoth => EnemyAbilityDisplayDef {
            name: "Behemoth",
            fallback: "Behem",
            icon: AbilityIcon::Standard(img015::ICON_BEHEMOTH),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Behemoth Enemy".into(),
        },
        Identity::TypeSage => EnemyAbilityDisplayDef {
            name: "Sage",
            fallback: "Sage",
            icon: AbilityIcon::Standard(img015::ICON_SAGE),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,param| fmt_sage(param),
        },
        Identity::TypeKaijin => EnemyAbilityDisplayDef {
            name: "Kaijin",
            fallback: "Villn",
            icon: AbilityIcon::Standard(img015::ICON_SUPERVILLIAN),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Kaijin Enemy".into(),
        },
        Identity::TypeWitch => EnemyAbilityDisplayDef {
            name: "Witch",
            fallback: "Witch",
            icon: AbilityIcon::Standard(img015::ICON_WITCH),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "Witch Enemy".into(),
        },
        Identity::TypeEva => EnemyAbilityDisplayDef {
            name: "EVA Angel",
            fallback: "EVA",
            icon: AbilityIcon::Standard(img015::ICON_EVA),
            group: DisplayGroup::Headline1,
            formatter: |_,_,_,_,_| "EVA Angel".into(),
        },

        Identity::Kamikaze => EnemyAbilityDisplayDef {
            name: "Kamikaze",
            fallback: "Kamik",
            icon: AbilityIcon::Custom(CustomIcon::Kamikaze),
            group: DisplayGroup::Headline2,
            formatter: |attacks,_,_,_,_| {
                let limit_suffix = match attacks {
                    0 => "immediately".to_string(),
                    1 => "after 1 attack".to_string(),
                    n => format!("after {} attacks", n),
                };
                format!("Unit disappears {}", limit_suffix)
            },
        },
        Identity::Stop => EnemyAbilityDisplayDef {
            name: "Stop",
            fallback: "Stop",
            icon: AbilityIcon::Custom(CustomIcon::Stop),
            group: DisplayGroup::Headline2,
            formatter: |attacks,_,_,_,_| {
                let limit_suffix = match attacks {
                    0 => "immediately".to_string(),
                    1 => "after 1 attack".to_string(),
                    n => format!("after {} attacks", n),
                };
                format!("Unit stops moving {}", limit_suffix)
            },
        },
        Identity::BaseDestroyer => EnemyAbilityDisplayDef {
            name: "Base Destroyer",
            fallback: "BaseD",
            icon: AbilityIcon::Standard(img015::ICON_BASE_DESTROYER),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "Deals 4× Damage to the Cat Base".into(),
        },
        Identity::WaveBlock => EnemyAbilityDisplayDef {
            name: "Wave Block",
            fallback: "W-Blk",
            icon: AbilityIcon::Standard(img015::ICON_WAVE_BLOCK),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        },
        Identity::CounterSurge => EnemyAbilityDisplayDef {
            name: "Counter Surge",
            fallback: "C-Srg",
            icon: AbilityIcon::Standard(img015::ICON_COUNTER_SURGE),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        },

        Identity::SingleAttack => EnemyAbilityDisplayDef {
            name: "Single Attack",
            fallback: "Sngl",
            icon: AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| {
                let atk_cd = fmt_time(stats.attack_cooldown);
                if stats.attack_2 > 0 {
                    format!("Attack cooldown {}", atk_cd)
                } else {
                    let tbh = fmt_time(stats.time_until_attack_1);
                    format!("Attack cooldown {}\nTime before hit {}", atk_cd, tbh)
                }
            },
        },
        Identity::AreaAttack => EnemyAbilityDisplayDef {
            name: "Area Attack",
            fallback: "Area",
            icon: AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| {
                let atk_cd = fmt_time(stats.attack_cooldown);
                if stats.attack_2 > 0 {
                    format!("Attack cooldown {}", atk_cd)
                } else {
                    let tbh = fmt_time(stats.time_until_attack_1);
                    format!("Attack cooldown {}\nTime before hit {}", atk_cd, tbh)
                }
            },
        },
        Identity::MultiHit => EnemyAbilityDisplayDef {
            name: "Multi-Hit",
            fallback: "Multi",
            icon: AbilityIcon::Custom(CustomIcon::Multihit),
            group: DisplayGroup::Body1,
            formatter: |_,stats,_,magnification,_| fmt_multihit(stats, magnification),
        },
        Identity::LongDistance => EnemyAbilityDisplayDef {
            name: "Long Distance",
            fallback: "LD",
            icon: AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
            group: DisplayGroup::Body1,
            formatter: |_,stats,_,_,_| fmt_effective_range(stats),
        },
        Identity::OmniStrike => EnemyAbilityDisplayDef {
            name: "Omni Strike",
            fallback: "Omni",
            icon: AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
            group: DisplayGroup::Body1,
            formatter: |_,stats,_,_,_| fmt_effective_range(stats),
        },
        Identity::WaveAttack => EnemyAbilityDisplayDef {
            name: "Wave Attack",
            fallback: "Wave",
            icon: AbilityIcon::Standard(img015::ICON_WAVE),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let maximum_reach = 467.5 + ((stats.wave_level - 1) as f32 * 200.0);
                format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", chance, stats.wave_level, maximum_reach)
            },
        },
        Identity::MiniWave => EnemyAbilityDisplayDef {
            name: "Mini-Wave",
            fallback: "MiniW",
            icon: AbilityIcon::Standard(img015::ICON_MINI_WAVE),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let maximum_reach = 467.5 + ((stats.wave_level - 1) as f32 * 200.0);
                format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", chance, stats.wave_level, maximum_reach)
            },
        },
        Identity::SurgeAttack => EnemyAbilityDisplayDef {
            name: "Surge Attack",
            fallback: "Surge",
            icon: AbilityIcon::Standard(img015::ICON_SURGE),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let start_bound = stats.surge_spawn_min;
                let end_bound = stats.surge_spawn_min + stats.surge_spawn_max;
                let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create a Level {} Surge\n{} Range", chance, stats.surge_level, fmt_range(minimum_range, maximum_range))
            },
        },
        Identity::MiniSurge => EnemyAbilityDisplayDef {
            name: "Mini-Surge",
            fallback: "MiniS",
            icon: AbilityIcon::Standard(img015::ICON_MINI_SURGE),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let start_bound = stats.surge_spawn_min;
                let end_bound = stats.surge_spawn_min + stats.surge_spawn_max;
                let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", chance, stats.surge_level, fmt_range(minimum_range, maximum_range))
            },
        },
        Identity::DeathSurge => EnemyAbilityDisplayDef {
            name: "Death Surge",
            fallback: "DSurg",
            icon: AbilityIcon::Standard(img015::ICON_DEATH_SURGE),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let start_bound = stats.death_surge_spawn_min;
                let end_bound = stats.death_surge_spawn_min + stats.death_surge_spawn_max;
                let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create a Level {} Surge\n{} Range upon death", chance, stats.death_surge_level, fmt_range(minimum_range, maximum_range))
            },
        },
        Identity::Explosion => EnemyAbilityDisplayDef {
            name: "Explosion",
            fallback: "Expl",
            icon: AbilityIcon::Standard(img015::ICON_EXPLOSION),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                let start_bound = stats.explosion_anchor;
                let end_bound = stats.explosion_anchor + stats.explosion_span;
                let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create an Explosion {} Range", chance, fmt_range(minimum_range, maximum_range))
            },
        },
        Identity::CriticalHit => EnemyAbilityDisplayDef {
            name: "Critical Hit",
            fallback: "Crit",
            icon: AbilityIcon::Standard(img015::ICON_CRITICAL_HIT),
            group: DisplayGroup::Body1,
            formatter: |chance,_,_,_,_| format!("{}% Chance to Critical Hit dealing +100% Damage\nCritcal Hits bypass Metal resistance", chance),
        },
        Identity::SavageBlow => EnemyAbilityDisplayDef {
            name: "Savage Blow",
            fallback: "Savge",
            icon: AbilityIcon::Standard(img015::ICON_SAVAGE_BLOW),
            group: DisplayGroup::Body1,
            formatter: |chance,stats,_,_,_| {
                format!("{}% Chance to Savage Blow\ndealing +{}% Damage", chance, stats.savage_blow_boost)
            },
        },
        Identity::Strengthen => EnemyAbilityDisplayDef {
            name: "Strengthen",
            fallback: "Str+",
            icon: AbilityIcon::Standard(img015::ICON_STRENGTHEN),
            group: DisplayGroup::Body1,
            formatter: |_,stats,_,_,_| format!("When reduced to or below {}% HP\nDamage dealt increases by +{}%", stats.strengthen_threshold, stats.strengthen_boost),
        },
        Identity::Survive => EnemyAbilityDisplayDef {
            name: "Survive",
            fallback: "Surv",
            icon: AbilityIcon::Standard(img015::ICON_SURVIVE),
            group: DisplayGroup::Body1,
            formatter: |chance,_,_,_,_| format!("{}% Chance to Survive a lethal strike", chance),
        },

        Identity::Barrier => EnemyAbilityDisplayDef {
            name: "Barrier",
            fallback: "Barri",
            icon: AbilityIcon::Standard(img015::ICON_BARRIER),
            group: DisplayGroup::Body2,
            formatter: |hp,_,_,_,_| format!("Has a Barrier with {} HP", hp),
        },
        Identity::AkuShield => EnemyAbilityDisplayDef {
            name: "Aku Shield",
            fallback: "Shiel",
            icon: AbilityIcon::Standard(img015::ICON_SHIELD),
            group: DisplayGroup::Body2,
            formatter: |hp,stats,_,magnification,_| {
                let scaled_hp = (hp as f32 * (magnification.hitpoints as f32 / 100.0)).round() as i32;
                if stats.shield_regen > 0 {
                    format!("Has a Shield with {} HP\nShield regenerates {}% HP when knocked back", scaled_hp, stats.shield_regen)
                } else {
                    format!("Has a Shield with {} HP", scaled_hp)
                }
            },
        },
        Identity::Burrow => EnemyAbilityDisplayDef {
            name: "Burrow",
            fallback: "Burro",
            icon: AbilityIcon::Custom(CustomIcon::Burrow),
            group: DisplayGroup::Body2,
            formatter: |count,stats,_,_,_| format!("Burrows {} Range {}", stats.burrow_distance, fmt_count(count)),
        },
        Identity::Revive => EnemyAbilityDisplayDef {
            name: "Revive",
            fallback: "Reviv",
            icon: AbilityIcon::Custom(CustomIcon::Revive),
            group: DisplayGroup::Body2,
            formatter: |count,stats,_,_,_| format!("Revives {} with {}% HP after {} \nDoesn't revive if Z-Killed", fmt_count(count), stats.revive_hp, fmt_time(stats.revive_time)),
        },
        Identity::Toxic => EnemyAbilityDisplayDef {
            name: "Toxic",
            fallback: "Toxic",
            icon: AbilityIcon::Standard(img015::ICON_TOXIC),
            group: DisplayGroup::Body2,
            formatter: |chance,stats,_,_,_| format!("{}% Chance to deal {}% of a\nCat's Max HP in additional damage", chance, stats.toxic_damage),
        },
        Identity::Drain => EnemyAbilityDisplayDef {
            name: "Drain",
            fallback: "Drain",
            icon: AbilityIcon::Standard(img015::ICON_DRAIN),
            group: DisplayGroup::Body2,
            formatter: |chance,stats,_,_,_| {
                format!("{}% Chance to extend\nongoing Cat cooldown by {}%", chance, stats.drain_percent)
            },
        },
        Identity::Dodge => EnemyAbilityDisplayDef {
            name: "Dodge",
            fallback: "Dodge",
            icon: AbilityIcon::Standard(img015::ICON_DODGE),
            group: DisplayGroup::Body2,
            formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Dodge attacks for {}", chance, fmt_time(duration_frames)),
        },
        Identity::Weaken => EnemyAbilityDisplayDef {
            name: "Weaken",
            fallback: "Weak",
            icon: AbilityIcon::Standard(img015::ICON_WEAKEN),
            group: DisplayGroup::Body2,
            formatter: |chance,stats,duration_frames,_,_| format!("{}% Chance to weaken Cats\nto {}% Attack Power for {}", chance, stats.weaken_percent, fmt_time(duration_frames)),
        },
        Identity::Freeze => EnemyAbilityDisplayDef {
            name: "Freeze",
            fallback: "Freez",
            icon: AbilityIcon::Standard(img015::ICON_FREEZE),
            group: DisplayGroup::Body2,
            formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Freeze Cats for {}", chance, fmt_time(duration_frames)),
        },
        Identity::Slow => EnemyAbilityDisplayDef {
            name: "Slow",
            fallback: "Slow",
            icon: AbilityIcon::Standard(img015::ICON_SLOW),
            group: DisplayGroup::Body2,
            formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Slow Cats for {}", chance, fmt_time(duration_frames)),
        },
        Identity::Knockback => EnemyAbilityDisplayDef {
            name: "Knockback",
            fallback: "KB",
            icon: AbilityIcon::Standard(img015::ICON_KNOCKBACK),
            group: DisplayGroup::Body2,
            formatter: |chance,_,_,_,_| format!("{}% Chance to Knockback Cats", chance),
        },
        Identity::Curse => EnemyAbilityDisplayDef {
            name: "Curse",
            fallback: "Curse",
            icon: AbilityIcon::Standard(img015::ICON_CURSE),
            group: DisplayGroup::Body2,
            formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Curse Cats for {}", chance, fmt_time(duration_frames)),
        },
        Identity::Warp => EnemyAbilityDisplayDef {
            name: "Warp",
            fallback: "Warp",
            icon: AbilityIcon::Standard(img015::ICON_WARP),
            group: DisplayGroup::Body2,
            formatter: |chance,stats,duration_frames,_,_| format!("{}% Chance to Warp Cats\n{} Range for {}", chance, fmt_compress(stats.warp_distance_minimum, stats.warp_distance_maximum), fmt_time(duration_frames)),
        },
        Identity::Unknown => EnemyAbilityDisplayDef {
            name: "Unknown",
            fallback: "Unkwn",
            icon: AbilityIcon::Custom(CustomIcon::Unknown),
            group: DisplayGroup::Body2,
            formatter: |_,_,_,_,_| "This Enemy may have an undefined ability\nBattle Cats Complete may need to be updated".into(),
        },

        Identity::ImmuneWave => EnemyAbilityDisplayDef {
            name: "Immune Wave",
            fallback: "NoWav",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WAVE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Wave Attacks".into(),
        },
        Identity::ImmuneSurge => EnemyAbilityDisplayDef {
            name: "Immune Surge",
            fallback: "NoSrg",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SURGE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Surge Attacks".into(),
        },
        Identity::ImmuneExplosion => EnemyAbilityDisplayDef {
            name: "Immune Explosion",
            fallback: "NoExp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_EXPLOSION),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Explosions".into(),
        },
        Identity::ImmuneWeaken => EnemyAbilityDisplayDef {
            name: "Immune Weaken",
            fallback: "NoWk",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WEAKEN),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Weaken".into(),
        },
        Identity::ImmuneFreeze => EnemyAbilityDisplayDef {
            name: "Immune Freeze",
            fallback: "NoFrz",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_FREEZE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Freeze".into(),
        },
        Identity::ImmuneSlow => EnemyAbilityDisplayDef {
            name: "Immune Slow",
            fallback: "NoSlw",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SLOW),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Slow".into(),
        },
        Identity::ImmuneKnockback => EnemyAbilityDisplayDef {
            name: "Immune Knockback",
            fallback: "NoKB",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_KNOCKBACK),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Knockback".into(),
        },
        Identity::ImmuneCurse => EnemyAbilityDisplayDef {
            name: "Immune Curse",
            fallback: "NoCur",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_CURSE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Curse".into(),
        },
        Identity::ImmuneWarp => EnemyAbilityDisplayDef {
            name: "Immune Warp",
            fallback: "NoWrp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WARP),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Warp".into(),
        },
    }
}

pub struct EnemyStatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&Battle, i32, Magnification) -> i32,
    pub formatter: fn(i32) -> String,
}

pub const STAT_HITPOINTS: EnemyStatsDef = EnemyStatsDef {
    name: "Hitpoints",
    display_name: "Hitpoints",
    get_value: |stats, _, magnification| (stats.hitpoints as f32 * (magnification.hitpoints as f32 / 100.0)).round() as i32,
    formatter: |hp| format!("{}", hp),
};

pub const STAT_KNOCKBACKS: EnemyStatsDef = EnemyStatsDef {
    name: "Knockbacks",
    display_name: "Knockback",
    get_value: |stats, _, _| stats.knockbacks,
    formatter: |kbs| format!("{}", kbs),
};

pub const STAT_SPEED: EnemyStatsDef = EnemyStatsDef {
    name: "Speed",
    display_name: "Speed",
    get_value: |stats, _, _| stats.speed,
    formatter: |speed| format!("{}", speed),
};

pub const STAT_RANGE: EnemyStatsDef = EnemyStatsDef {
    name: "Range",
    display_name: "Range",
    get_value: |stats, _, _| stats.standing_range,
    formatter: |range| format!("{}", range),
};

pub const STAT_ATTACK: EnemyStatsDef = EnemyStatsDef {
    name: "Attack",
    display_name: "Attack",
    get_value: |stats, _, magnification| {
        let magnification_factor = magnification.attack as f32 / 100.0;
        let damage_hit_1 = (stats.attack_1 as f32 * magnification_factor).round() as i32;
        let damage_hit_2 = (stats.attack_2 as f32 * magnification_factor).round() as i32;
        let damage_hit_3 = (stats.attack_3 as f32 * magnification_factor).round() as i32;
        damage_hit_1 + damage_hit_2 + damage_hit_3
    },
    formatter: |attack| format!("{}", attack),
};

pub const STAT_DPS: EnemyStatsDef = EnemyStatsDef {
    name: "Dps",
    display_name: "DPS",
    get_value: |stats, animation_frames, magnification| {
        let magnification_factor = magnification.attack as f32 / 100.0;
        let damage_hit_1 = (stats.attack_1 as f32 * magnification_factor).round() as i32;
        let damage_hit_2 = (stats.attack_2 as f32 * magnification_factor).round() as i32;
        let damage_hit_3 = (stats.attack_3 as f32 * magnification_factor).round() as i32;
        let total_attack_damage = damage_hit_1 + damage_hit_2 + damage_hit_3;

        let mut effective_foreswing = stats.time_until_attack_1;
        if stats.attack_3 > 0 && stats.time_until_attack_3 > 0 {
            effective_foreswing = stats.time_until_attack_3;
        } else if stats.attack_2 > 0 && stats.time_until_attack_2 > 0 {
            effective_foreswing = stats.time_until_attack_2;
        }
        let cooldown_frames = stats.attack_cooldown.saturating_sub(1);
        let attack_cycle = (effective_foreswing + cooldown_frames).max(animation_frames);

        if attack_cycle > 0 { ((total_attack_damage as f32 * 30.0) / attack_cycle as f32).round() as i32 } else { 0 }
    },
    formatter: |dps| format!("{}", dps),
};

pub const STAT_ATK_CYCLE: EnemyStatsDef = EnemyStatsDef {
    name: "Atk Cycle",
    display_name: "Atk Cycle",
    get_value: |stats, animation_frames, _| {
        let mut effective_foreswing = stats.time_until_attack_1;
        if stats.attack_3 > 0 && stats.time_until_attack_3 > 0 {
            effective_foreswing = stats.time_until_attack_3;
        } else if stats.attack_2 > 0 && stats.time_until_attack_2 > 0 {
            effective_foreswing = stats.time_until_attack_2;
        }
        let cooldown_frames = stats.attack_cooldown.saturating_sub(1);
        (effective_foreswing + cooldown_frames).max(animation_frames)
    },
    formatter: |cycle| format!("{}f", cycle),
};

pub const STAT_CASH_DROP: EnemyStatsDef = EnemyStatsDef {
    name: "Cash Drop",
    display_name: "Cash Drop",
    get_value: |stats, _, _| (stats.cash_drop as f32 * 3.95).floor() as i32,
    formatter: |cash| format!("{}¢", cash),
};

pub(crate) const ENEMY_STATS_REGISTRY: &[EnemyStatsDef] = &[
    STAT_HITPOINTS,
    STAT_KNOCKBACKS,
    STAT_SPEED,
    STAT_RANGE,
    STAT_ATTACK,
    STAT_DPS,
    STAT_ATK_CYCLE,
    STAT_CASH_DROP,
];

pub fn format_enemy_stat(def: &EnemyStatsDef, stats: &Battle, animation_frames: i32, magnification: Magnification) -> String {
    (def.formatter)((def.get_value)(stats, animation_frames, magnification))
}

pub fn get_fallback_by_icon(target_icon: AbilityIcon) -> &'static str {
    for pure_definition in REGISTRY {
        let display_definition = get_display_def(pure_definition.identity);

        if display_definition.icon == target_icon {
            return display_definition.fallback;
        }
    }
    "???"
}