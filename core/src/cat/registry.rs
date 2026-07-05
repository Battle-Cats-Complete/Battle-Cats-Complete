use std::collections::HashMap;

use nyanko::cat::abilities::{Identity, REGISTRY};
use nyanko::cat::unit::{Battle, UnitBuy};
use nyanko::common::data::{img015, Param};

use crate::global::game::abilities::CustomIcon;

#[derive(PartialEq, Clone, Copy)]
pub enum DisplayGroup {
    Trait,
    Headline1,
    Headline2,
    Body1,
    Body2,
    Footer,
    Hidden,
}

#[derive(PartialEq, Clone, Copy, Hash, Eq)]
pub enum AbilityIcon {
    Standard(usize),
    Custom(CustomIcon),
    None,
}

pub struct AbilityDisplayDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub group: DisplayGroup,
    pub formatter: fn(val1: i32, stats: &Battle, target: &str, duration_frames: i32, param: &Param) -> String,
}

// --- CORE STRING FORMATTERS ---

fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min_range: i32, max_range: i32) -> String {
    if min_range == max_range {
        format!("at {}", min_range)
    } else {
        format!("between {}~{}", min_range, max_range)
    }
}

fn fmt_compress(min_val: i32, max_val: i32) -> String {
    if min_val == max_val {
        format!("{}", min_val)
    } else {
        format!("{}~{}", min_val, max_val)
    }
}

fn fmt_effective_range(stats: &Battle) -> String {
    let primary_anchor = if stats.long_distance_1_anchor != 0 {
        stats.long_distance_1_anchor
    } else {
        stats.standing_range
    };

    let mut range_strings = Vec::new();

    let has_ld_or_omni = (stats.long_distance_1_span != 0 || stats.long_distance_1_anchor != 0) ||
        (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span != 0 || stats.long_distance_2_anchor != 0)) ||
        (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span != 0 || stats.long_distance_3_anchor != 0));

    if has_ld_or_omni {
        let hit_data = [
            (true, stats.long_distance_1_anchor, stats.long_distance_1_span, 1),
            (stats.attack_2 > 0, stats.long_distance_2_anchor, stats.long_distance_2_span, stats.long_distance_2_flag),
            (stats.attack_3 > 0, stats.long_distance_3_anchor, stats.long_distance_3_span, stats.long_distance_3_flag),
        ];

        for (is_active, anchor, span, flag) in hit_data {
            if !is_active { continue; }

            if flag > 0 && (span != 0 || anchor != 0) {
                let start = anchor;
                let end = anchor + span;
                let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                range_strings.push(format!("{}~{}", min_r, max_r));
            } else if stats.long_distance_1_span != 0 || stats.long_distance_1_anchor != 0 {
                let start = stats.long_distance_1_anchor;
                let end = stats.long_distance_1_anchor + stats.long_distance_1_span;
                let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                range_strings.push(format!("{}~{}", min_r, max_r));
            } else {
                range_strings.push(format!("-320~{}", stats.standing_range));
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
    format!("{} {}\nStands at {} Range relative to Enemy Base", label_prefix, range_strings.join(" / "), primary_anchor)
}

fn fmt_multihit(stats: &Battle) -> String {
    let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if stats.attack_3 == 0 {
        ""
    } else if stats.attack_3_abilities > 0 {
        " / True"
    } else {
        " / False"
    };

    let damage_string = if stats.attack_3 > 0 {
        format!("{} / {} / {}", stats.attack_1, stats.attack_2, stats.attack_3)
    } else {
        format!("{} / {}", stats.attack_1, stats.attack_2)
    };

    let timing_string = if stats.attack_3 > 0 {
        format!("{} / {} / {}", fmt_time(stats.time_until_attack_1), fmt_time(stats.time_until_attack_2), fmt_time(stats.time_until_attack_3))
    } else {
        format!("{} / {}", fmt_time(stats.time_until_attack_1), fmt_time(stats.time_until_attack_2))
    };

    format!("Damage split {}\nTiming split {}\nAbility split {} / {}{}", damage_string, timing_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

fn fmt_sage(param: &Param) -> String {
    let mut resistance_groups_by_percentage: HashMap<i32, Vec<&str>> = HashMap::new();
    
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_weaken).or_default().push("Weaken");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_freeze).or_default().push("Freeze");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_slow).or_default().push("Slow");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_curse).or_default().push("Curse");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_other).or_default().push("Knockback");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_other).or_default().push("Delay");
    resistance_groups_by_percentage.entry(param.sage_slayer_resist_warp).or_default().push("Warp");

    let base_description = format!(
        "Deals {:.1}× Damage to and takes {:.1}× Damage from Sage Enemies\nIgnores the Crowd Control resistance of Sage Enemies\nCrowd Control effects originating from Sage Enemies reduced by",
        param.sage_slayer_attack_multiplier,
        param.sage_slayer_defense_multiplier
    );

    if resistance_groups_by_percentage.len() == 1 {
        if let Some((percentage, _)) = resistance_groups_by_percentage.into_iter().next() {
            return format!("{} {}%", base_description, percentage);
        }
        return base_description;
    }

    let mut formatted_resistance_lines = Vec::new();
    let mut sorted_resistance_groups: Vec<_> = resistance_groups_by_percentage.into_iter().collect();
    sorted_resistance_groups.sort_by(|group_a, group_b| group_b.0.cmp(&group_a.0));

    for (percentage, effect_names) in sorted_resistance_groups {
        let formatted_effect_list = match effect_names.len() {
            1 => effect_names[0].to_string(),
            2 => format!("{} and {}", effect_names[0], effect_names[1]),
            _ => {
                let all_effects_except_last = effect_names[..effect_names.len() - 1].join(", ");
                if let Some(last) = effect_names.last() {
                    format!("{}, and {}", all_effects_except_last, last)
                } else {
                    all_effects_except_last
                }
            }
        };
        formatted_resistance_lines.push(format!("{}% for {}", percentage, formatted_effect_list));
    }
    format!("{}\n{}", base_description, formatted_resistance_lines.join("\n"))
}

// --- EXHAUSTIVE PRESENTATION MATCH ---

pub fn get_display_def(identity: Identity) -> AbilityDisplayDef {
    match identity {
        Identity::TargetRed => AbilityDisplayDef {
            name: "Target Red",
            fallback: "Red",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RED),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Red Enemies".into(),
        },
        Identity::TargetFloat => AbilityDisplayDef {
            name: "Target Float",
            fallback: "Float",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_FLOATING),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Floating Enemies".into(),
        },
        Identity::TargetDark => AbilityDisplayDef {
            name: "Target Dark",
            fallback: "Dark",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_BLACK),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Dark Enemies".into(),
        },
        Identity::TargetMetal => AbilityDisplayDef {
            name: "Target Metal",
            fallback: "Metal",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_METAL),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Metal Enemies".into(),
        },
        Identity::TargetAngel => AbilityDisplayDef {
            name: "Target Angel",
            fallback: "Angel",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ANGEL),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Angel Enemies".into(),
        },
        Identity::TargetAlien => AbilityDisplayDef {
            name: "Target Alien",
            fallback: "Alien",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ALIEN),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Alien Enemies".into(),
        },
        Identity::TargetZombie => AbilityDisplayDef {
            name: "Target Zombie",
            fallback: "Zomb",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ZOMBIE),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Zombie Enemies".into(),
        },
        Identity::TargetRelic => AbilityDisplayDef {
            name: "Target Relic",
            fallback: "Relic",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RELIC),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Relic Enemies".into(),
        },
        Identity::TargetAku => AbilityDisplayDef {
            name: "Target Aku",
            fallback: "Aku",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_AKU),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Aku Enemies".into(),
        },
        Identity::TargetTraitless => AbilityDisplayDef {
            name: "Target Traitless",
            fallback: "NoTrt",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_TRAITLESS),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Traitless Enemies".into(),
        },
        Identity::TargetWitch => AbilityDisplayDef {
            name: "Target Witch",
            fallback: "Witch",
            icon: AbilityIcon::Standard(img015::ICON_WITCH),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets Witch Enemies".into(),
        },
        Identity::TargetEva => AbilityDisplayDef {
            name: "Target EVA",
            fallback: "EVA",
            icon: AbilityIcon::Standard(img015::ICON_EVA),
            group: DisplayGroup::Trait,
            formatter: |_,_,_,_,_| "Targets EVA Angels".into(),
        },

        // --- HEADLINE 1 ---
        Identity::AttackOnly => AbilityDisplayDef {
            name: "Attack Only",
            fallback: "AtkOnly",
            icon: AbilityIcon::Standard(img015::ICON_ATTACK_ONLY),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Only damages {}", target),
        },
        Identity::StrongAgainst => AbilityDisplayDef {
            name: "Strong Against",
            fallback: "Strng",
            icon: AbilityIcon::Standard(img015::ICON_STRONG_AGAINST),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target),
        },
        Identity::MassiveDamage => AbilityDisplayDef {
            name: "Massive Damage",
            fallback: "Massv",
            icon: AbilityIcon::Standard(img015::ICON_MASSIVE_DAMAGE),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Deals 3×~4× Damage to {}", target),
        },
        Identity::InsaneDamage => AbilityDisplayDef {
            name: "Insane Damage",
            fallback: "InsDmg",
            icon: AbilityIcon::Standard(img015::ICON_INSANE_DAMAGE),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Deals 5×~6× Damage to {}", target),
        },
        Identity::Resist => AbilityDisplayDef {
            name: "Resist",
            fallback: "Resist",
            icon: AbilityIcon::Standard(img015::ICON_RESIST),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Takes 1/4×~1/5× Damage from {}", target),
        },
        Identity::InsanelyTough => AbilityDisplayDef {
            name: "Insanely Tough",
            fallback: "InsRes",
            icon: AbilityIcon::Standard(img015::ICON_INSANELY_TOUGH),
            group: DisplayGroup::Headline1,
            formatter: |_, _, target, _, _| format!("Takes 1/6×~1/7× Damage from {}", target),
        },

        // --- HEADLINE 2 ---
        Identity::Metal => AbilityDisplayDef {
            name: "Metal",
            fallback: "Metal",
            icon: AbilityIcon::Standard(img015::ICON_METAL),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, _| "Damage taken is reduced to 1 for Non-Critical attacks".into(),
        },
        Identity::BaseDestroyer => AbilityDisplayDef {
            name: "Base Destroyer",
            fallback: "Base",
            icon: AbilityIcon::Standard(img015::ICON_BASE_DESTROYER),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "Deals 4× Damage to the Enemy Base".into(),
        },
        Identity::DoubleBounty => AbilityDisplayDef {
            name: "Double Bounty",
            fallback: "2×$",
            icon: AbilityIcon::Standard(img015::ICON_DOUBLE_BOUNTY),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "Receives 2× Cash from Enemies".into(),
        },
        Identity::ZombieKiller => AbilityDisplayDef {
            name: "Zombie Killer",
            fallback: "Zkill",
            icon: AbilityIcon::Standard(img015::ICON_ZOMBIE_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, _| "Prevents Zombies from reviving".into(),
        },
        Identity::Soulstrike => AbilityDisplayDef {
            name: "Soulstrike",
            fallback: "SolStk",
            icon: AbilityIcon::Standard(img015::ICON_SOULSTRIKE),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, _| "Will attack Zombie corpses".into(),
        },
        Identity::ColossusSlayer => AbilityDisplayDef {
            name: "Colossus Slayer",
            fallback: "Colos",
            icon: AbilityIcon::Standard(img015::ICON_COLOSSUS_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, _| "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(),
        },
        Identity::SageSlayer => AbilityDisplayDef {
            name: "Sage Slayer",
            fallback: "Sage",
            icon: AbilityIcon::Standard(img015::ICON_SAGE_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, param| fmt_sage(param),
        },
        Identity::BehemothSlayer => AbilityDisplayDef {
            name: "Behemoth Slayer",
            fallback: "Behem",
            icon: AbilityIcon::Standard(img015::ICON_BEHEMOTH_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |_, stats, _, _, param| {
                let mut formatted_text = format!("Deals {:.1}× Damage to and takes {:.1}× Damage from Behemoth Enemies", param.behemoth_slayer_attack_multiplier as f32 / 1000.0, param.behemoth_slayer_defense_multiplier as f32 / 1000.0);
                if stats.behemoth_dodge_chance > 0 {
                    formatted_text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", stats.behemoth_dodge_chance, fmt_time(stats.behemoth_dodge_duration)));
                }
                formatted_text
            },
        },
        Identity::WitchKiller => AbilityDisplayDef {
            name: "Witch Killer",
            fallback: "Witch",
            icon: AbilityIcon::Standard(img015::ICON_WITCH_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "Deals 5× Damage to and takes 0.1× Damage from Witches".into(),
        },
        Identity::EvaKiller => AbilityDisplayDef {
            name: "Eva Killer",
            fallback: "Eva",
            icon: AbilityIcon::Standard(img015::ICON_EVA_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(),
        },
        Identity::WaveBlock => AbilityDisplayDef {
            name: "Wave Block",
            fallback: "W-Blk",
            icon: AbilityIcon::Standard(img015::ICON_WAVE_BLOCK),
            group: DisplayGroup::Headline2,
            formatter: |_, _, _, _, _| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        },
        Identity::CounterSurge => AbilityDisplayDef {
            name: "Counter Surge",
            fallback: "C-Srg",
            icon: AbilityIcon::Standard(img015::ICON_COUNTER_SURGE),
            group: DisplayGroup::Headline2,
            formatter: |_,_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        },
        Identity::Kamikaze => AbilityDisplayDef {
            name: "Kamikaze",
            fallback: "Kamik",
            icon: AbilityIcon::Custom(CustomIcon::Kamikaze),
            group: DisplayGroup::Headline2,
            formatter: |attacks, _, _, _, _| {
                match attacks {
                    0 => "Unit disappears immediately".to_string(),
                    1 => "Unit disappears after 1 attack".to_string(),
                    n => format!("Unit disappears after {} attacks", n),
                }
            },
        },
        Identity::Stop => AbilityDisplayDef {
            name: "Stop",
            fallback: "Stop",
            icon: AbilityIcon::Custom(CustomIcon::Stop),
            group: DisplayGroup::Headline2,
            formatter: |attacks, _, _, _, _| {
                match attacks {
                    0 => "Unit stops moving immediately".to_string(),
                    1 => "Unit stops moving after 1 attack".to_string(),
                    n => format!("Unit stops moving after {} attacks", n),
                }
            },
        },

        // --- BODY 1 ---
        Identity::SingleAttack => AbilityDisplayDef {
            name: "Single Attack",
            fallback: "Sngl",
            icon: AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| {
                let tba = fmt_time(stats.attack_cooldown);
                if stats.attack_2 > 0 {
                    format!("Time between attacks {}", tba)
                } else {
                    let tbh = fmt_time(stats.time_until_attack_1);
                    format!("Time between attacks {}\nTime before hit {}", tba, tbh)
                }
            },
        },
        Identity::AreaAttack => AbilityDisplayDef {
            name: "Area Attack",
            fallback: "Area",
            icon: AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| {
                let tba = fmt_time(stats.attack_cooldown);
                if stats.attack_2 > 0 {
                    format!("Time between attacks {}", tba)
                } else {
                    let tbh = fmt_time(stats.time_until_attack_1);
                    format!("Time between attacks {}\nTime before hit {}", tba, tbh)
                }
            },
        },
        Identity::MultiHit => AbilityDisplayDef {
            name: "Multi-Hit",
            fallback: "Multi",
            icon: AbilityIcon::Custom(CustomIcon::Multihit),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| fmt_multihit(stats),
        },
        Identity::LongDistance => AbilityDisplayDef {
            name: "Long Distance",
            fallback: "LD",
            icon: AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| fmt_effective_range(stats),
        },
        Identity::OmniStrike => AbilityDisplayDef {
            name: "Omni Strike",
            fallback: "Omni",
            icon: AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| fmt_effective_range(stats),
        },
        Identity::Conjure => AbilityDisplayDef {
            name: "Conjure",
            fallback: "Spirit",
            icon: AbilityIcon::Standard(img015::ICON_CONJURE),
            group: DisplayGroup::Body1,
            formatter: |_,_,_,_,_| "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".into(),
        },
        Identity::MetalKiller => AbilityDisplayDef {
            name: "Metal Killer",
            fallback: "MetKil",
            icon: AbilityIcon::Standard(img015::ICON_METAL_KILLER),
            group: DisplayGroup::Body1,
            formatter: |percent,_,_,_,_| format!("Reduces Metal enemies current HP by {}% upon hit", percent),
        },
        Identity::WaveAttack => AbilityDisplayDef {
            name: "Wave Attack",
            fallback: "Wave",
            icon: AbilityIcon::Standard(img015::ICON_WAVE),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| {
                let maximum_reach = 332.5 + ((stats.wave_level - 1) as f32 * 200.0);
                format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", chance, stats.wave_level, maximum_reach)
            },
        },
        Identity::MiniWave => AbilityDisplayDef {
            name: "Mini-Wave",
            fallback: "MiniW",
            icon: AbilityIcon::Standard(img015::ICON_MINI_WAVE),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| {
                let maximum_reach = 332.5 + ((stats.wave_level - 1) as f32 * 200.0);
                format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", chance, stats.wave_level, maximum_reach)
            },
        },
        Identity::SurgeAttack => AbilityDisplayDef {
            name: "Surge Attack",
            fallback: "Surge",
            icon: AbilityIcon::Standard(img015::ICON_SURGE),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| {
                let start_bound = stats.surge_spawn_anchor;
                let end_bound = stats.surge_spawn_anchor + stats.surge_spawn_span;
                let (min_r, max_r) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create a Level {} Surge\n{} Range", chance, stats.surge_level, fmt_range(min_r, max_r))
            },
        },
        Identity::MiniSurge => AbilityDisplayDef {
            name: "Mini-Surge",
            fallback: "MiniS",
            icon: AbilityIcon::Standard(img015::ICON_MINI_SURGE),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| {
                let start_bound = stats.surge_spawn_anchor;
                let end_bound = stats.surge_spawn_anchor + stats.surge_spawn_span;
                let (min_r, max_r) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", chance, stats.surge_level, fmt_range(min_r, max_r))
            },
        },
        Identity::Explosion => AbilityDisplayDef {
            name: "Explosion",
            fallback: "Expl",
            icon: AbilityIcon::Standard(img015::ICON_EXPLOSION),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| {
                let start_bound = stats.explosion_spawn_anchor;
                let end_bound = stats.explosion_spawn_anchor + stats.explosion_spawn_span;
                let (min_r, max_r) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
                format!("{}% Chance to create an Explosion {} Range", chance, fmt_range(min_r, max_r))
            },
        },
        Identity::SavageBlow => AbilityDisplayDef {
            name: "Savage Blow",
            fallback: "Savge",
            icon: AbilityIcon::Standard(img015::ICON_SAVAGE_BLOW),
            group: DisplayGroup::Body1,
            formatter: |chance, stats, _, _, _| format!("{}% Chance to Savage Blow\ndealing +{}% Damage", chance, stats.savage_blow_boost),
        },
        Identity::CriticalHit => AbilityDisplayDef {
            name: "Critical Hit",
            fallback: "Crit",
            icon: AbilityIcon::Standard(img015::ICON_CRITICAL_HIT),
            group: DisplayGroup::Body1,
            formatter: |chance, _, _, _, _| format!("{}% Chance to Critical Hit dealing +100% Damage\nCritcal Hits bypass Metal resistance", chance),
        },
        Identity::Strengthen => AbilityDisplayDef {
            name: "Strengthen",
            fallback: "Str+",
            icon: AbilityIcon::Standard(img015::ICON_STRENGTHEN),
            group: DisplayGroup::Body1,
            formatter: |_, stats, _, _, _| format!("When reduced to or below {}% HP\nDamage dealt increases by +{}%", stats.strengthen_threshold, stats.strengthen_boost),
        },
        Identity::Survive => AbilityDisplayDef {
            name: "Survive",
            fallback: "Surv",
            icon: AbilityIcon::Standard(img015::ICON_SURVIVE),
            group: DisplayGroup::Body1,
            formatter: |chance, _, _, _, _| format!("{}% Chance to Survive a lethal strike", chance),
        },
        Identity::BarrierBreaker => AbilityDisplayDef {
            name: "Barrier Breaker",
            fallback: "Brkr",
            icon: AbilityIcon::Standard(img015::ICON_BARRIER_BREAKER),
            group: DisplayGroup::Body1,
            formatter: |chance, _, _, _, _| format!("{}% Chance to break enemy Barriers", chance),
        },
        Identity::ShieldPiercer => AbilityDisplayDef {
            name: "Shield Piercer",
            fallback: "Spierc",
            icon: AbilityIcon::Standard(img015::ICON_SHIELD_PIERCER),
            group: DisplayGroup::Body1,
            formatter: |chance, _, _, _, _| format!("{}% Chance to pierce enemy Shields", chance),
        },

        // --- BODY 2 ---
        Identity::Dodge => AbilityDisplayDef {
            name: "Dodge",
            fallback: "Dodge",
            icon: AbilityIcon::Standard(img015::ICON_DODGE),
            group: DisplayGroup::Body2,
            formatter: |chance, _, target, duration_frames, _| format!("{}% Chance to Dodge {} for {}", chance, target, fmt_time(duration_frames)),
        },
        Identity::Weaken => AbilityDisplayDef {
            name: "Weaken",
            fallback: "Weak",
            icon: AbilityIcon::Standard(img015::ICON_WEAKEN),
            group: DisplayGroup::Body2,
            formatter: |chance, stats, target, duration_frames, _| format!("{}% Chance to weaken {}\nto {}% Attack Power for {}", chance, target, stats.weaken_to, fmt_time(duration_frames)),
        },
        Identity::Freeze => AbilityDisplayDef {
            name: "Freeze",
            fallback: "Freez",
            icon: AbilityIcon::Standard(img015::ICON_FREEZE),
            group: DisplayGroup::Body2,
            formatter: |chance, _, target, duration_frames, _| format!("{}% Chance to Freeze {} for {}", chance, target, fmt_time(duration_frames)),
        },
        Identity::Slow => AbilityDisplayDef {
            name: "Slow",
            fallback: "Slow",
            icon: AbilityIcon::Standard(img015::ICON_SLOW),
            group: DisplayGroup::Body2,
            formatter: |chance, _, target, duration_frames, _| format!("{}% Chance to Slow {} for {}", chance, target, fmt_time(duration_frames)),
        },
        Identity::Knockback => AbilityDisplayDef {
            name: "Knockback",
            fallback: "KB",
            icon: AbilityIcon::Standard(img015::ICON_KNOCKBACK),
            group: DisplayGroup::Body2,
            formatter: |chance, _, target, _, _| format!("{}% Chance to Knockback {}", chance, target),
        },
        Identity::Curse => AbilityDisplayDef {
            name: "Curse",
            fallback: "Curse",
            icon: AbilityIcon::Standard(img015::ICON_CURSE),
            group: DisplayGroup::Body2,
            formatter: |chance, _, target, duration_frames, _| format!("{}% Chance to Curse {} for {}", chance, target, fmt_time(duration_frames)),
        },
        Identity::Warp => AbilityDisplayDef {
            name: "Warp",
            fallback: "Warp",
            icon: AbilityIcon::Standard(img015::ICON_WARP),
            group: DisplayGroup::Body2,
            formatter: |chance, stats, target, duration_frames, _| format!("{}% Chance to Warp {}\n{} Range for {}", chance, target, fmt_compress(stats.warp_distance_minimum, stats.warp_distance_maximum), fmt_time(duration_frames)),
        },
        Identity::Unknown => AbilityDisplayDef {
            name: "Unknown",
            fallback: "Unkwn",
            icon: AbilityIcon::Custom(CustomIcon::Unknown),
            group: DisplayGroup::Body2,
            formatter: |_,_,_,_,_| "This Cat has an undefined ability\nThe App may need to be updated".into(),
        },

        // --- FOOTER (IMMUNITIES) ---
        Identity::ImmuneWave => AbilityDisplayDef {
            name: "Immune Wave",
            fallback: "NoWav",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WAVE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Wave Attacks".into(),
        },
        Identity::ImmuneSurge => AbilityDisplayDef {
            name: "Immune Surge",
            fallback: "NoSrg",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SURGE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Surge Attacks".into(),
        },
        Identity::ImmuneExplosion => AbilityDisplayDef {
            name: "Immune Explosion",
            fallback: "NoExp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_EXPLOSION),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Explosions".into(),
        },
        Identity::ImmuneWeaken => AbilityDisplayDef {
            name: "Immune Weaken",
            fallback: "NoWk",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WEAKEN),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Weaken".into(),
        },
        Identity::ImmuneFreeze => AbilityDisplayDef {
            name: "Immune Freeze",
            fallback: "NoFrz",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_FREEZE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Freeze".into(),
        },
        Identity::ImmuneSlow => AbilityDisplayDef {
            name: "Immune Slow",
            fallback: "NoSlw",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SLOW),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Slow".into(),
        },
        Identity::ImmuneKnockback => AbilityDisplayDef {
            name: "Immune Knockback",
            fallback: "NoKB",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_KNOCKBACK),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Knockback".into(),
        },
        Identity::ImmuneCurse => AbilityDisplayDef {
            name: "Immune Curse",
            fallback: "NoCur",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_CURSE),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Curse".into(),
        },
        Identity::ImmuneToxic => AbilityDisplayDef {
            name: "Immune Toxic",
            fallback: "NoTox",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_TOXIC),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Toxic".into(),
        },
        Identity::ImmuneWarp => AbilityDisplayDef {
            name: "Immune Warp",
            fallback: "NoWrp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WARP),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Warp".into(),
        },
        Identity::ImmuneBossWave => AbilityDisplayDef {
            name: "Immune Boss Wave",
            fallback: "NoBos",
            icon: AbilityIcon::Custom(CustomIcon::BossWave),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "Immune to Boss Shockwaves".into(),
        },

        // --- FOOTER (RESISTANCES) ---
        Identity::ResistWeaken => AbilityDisplayDef {
            name: "Resist Weaken",
            fallback: "ReWkn",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WEAKEN),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Weaken ({}%)", percent),
        },
        Identity::ResistFreeze => AbilityDisplayDef {
            name: "Resist Freeze",
            fallback: "ReFrz",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_FREEZE),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Freeze ({}%)", percent),
        },
        Identity::ResistSlow => AbilityDisplayDef {
            name: "Resist Slow",
            fallback: "ReSlw",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_SLOW),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Slow ({}%)", percent),
        },
        Identity::ResistKnockback => AbilityDisplayDef {
            name: "Resist Knockback",
            fallback: "ReKB",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_KNOCKBACK),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Knockback ({}%)", percent),
        },
        Identity::ResistWave => AbilityDisplayDef {
            name: "Resist Wave",
            fallback: "ReWav",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WAVE),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Wave ({}%)", percent),
        },
        Identity::ResistWarp => AbilityDisplayDef {
            name: "Resist Warp",
            fallback: "ReWrp",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WARP),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Warp ({}%)", percent),
        },
        Identity::ResistCurse => AbilityDisplayDef {
            name: "Resist Curse",
            fallback: "ReCur",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_CURSE),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Curse ({}%)", percent),
        },
        Identity::ResistToxic => AbilityDisplayDef {
            name: "Resist Toxic",
            fallback: "ReTox",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_TOXIC),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Toxic ({}%)", percent),
        },
        Identity::ResistSurge => AbilityDisplayDef {
            name: "Resist Surge",
            fallback: "ReSrg",
            icon: AbilityIcon::Standard(img015::ICON_SURGE_RESIST),
            group: DisplayGroup::Footer,
            formatter: |percent,_,_,_,_| format!("Resist Surge ({}%)", percent),
        },

        // --- STAT TALENTS ---
        Identity::CostDown => AbilityDisplayDef {
            name: "Cost Down",
            fallback: "Cost-",
            icon: AbilityIcon::Standard(img015::ICON_COST_DOWN),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::RecoverSpeedUp => AbilityDisplayDef {
            name: "Recover Speed Up",
            fallback: "Rec+",
            icon: AbilityIcon::Standard(img015::ICON_RECOVER_SPEED_UP),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::MoveSpeedUp => AbilityDisplayDef {
            name: "Move Speed Up",
            fallback: "Spd",
            icon: AbilityIcon::Standard(img015::ICON_MOVE_SPEED),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::AttackBuff => AbilityDisplayDef {
            name: "Attack Buff",
            fallback: "Atk+",
            icon: AbilityIcon::Standard(img015::ICON_ATTACK_BUFF),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::HealthBuff => AbilityDisplayDef {
            name: "Health Buff",
            fallback: "HP+",
            icon: AbilityIcon::Standard(img015::ICON_HEALTH_BUFF),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::TbaDown => AbilityDisplayDef {
            name: "TBA Down",
            fallback: "TBA-",
            icon: AbilityIcon::Standard(img015::ICON_TBA_DOWN),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
        Identity::ImproveKnockbacks => AbilityDisplayDef {
            name: "Improve Knockbacks",
            fallback: "KB+",
            icon: AbilityIcon::Standard(img015::ICON_IMPROVE_KNOCKBACK_COUNT),
            group: DisplayGroup::Footer,
            formatter: |_,_,_,_,_| "".into(),
        },
    }
}

// --- STATS REGISTRY ---

pub struct CatStatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&Battle, i32, Option<&UnitBuy>) -> i32,
    pub formatter: fn(i32) -> String,
    pub linked_talent_id: Option<u8>,
    pub talent_modifier_fmt: Option<fn(i32, i32) -> String>,
}

pub const CAT_STATS_REGISTRY: &[CatStatsDef] = &[
    CatStatsDef {
        name: "Hitpoints",
        display_name: "Hitpoints",
        get_value: |stats, _, _| stats.hitpoints,
        formatter: |hp| format!("{}", hp),
        linked_talent_id: Some(32),
        talent_modifier_fmt: Some(|percent, _| format!("(+{}%)", percent)),
    },
    CatStatsDef {
        name: "Knockbacks",
        display_name: "Knockback",
        get_value: |stats, _, _| stats.knockbacks,
        formatter: |kbs| format!("{}", kbs),
        linked_talent_id: Some(28),
        talent_modifier_fmt: Some(|count, _| format!("(+{})", count)),
    },
    CatStatsDef {
        name: "Speed",
        display_name: "Speed",
        get_value: |stats, _, _| stats.speed,
        formatter: |spd| format!("{}", spd),
        linked_talent_id: Some(27),
        talent_modifier_fmt: Some(|spd, _| format!("(+{})", spd)),
    },
    CatStatsDef {
        name: "Range",
        display_name: "Range",
        get_value: |stats, _, _| stats.standing_range,
        formatter: |rng| format!("{}", rng),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Attack",
        display_name: "Attack",
        get_value: |stats, _, _| stats.attack_1 + stats.attack_2 + stats.attack_3,
        formatter: |atk| format!("{}", atk),
        linked_talent_id: Some(31),
        talent_modifier_fmt: Some(|percent, _| format!("(+{}%)", percent)),
    },
    CatStatsDef {
        name: "Dps",
        display_name: "DPS",
        get_value: |stats, animation_frames, _| {
            let total_attack_damage = stats.attack_1 + stats.attack_2 + stats.attack_3;
            let mut effective_foreswing = stats.time_until_attack_1;
            if stats.attack_3 > 0 && stats.time_until_attack_3 > 0 { effective_foreswing = stats.time_until_attack_3; }
            else if stats.attack_2 > 0 && stats.time_until_attack_2 > 0 { effective_foreswing = stats.time_until_attack_2; }
            let cooldown_frames = stats.attack_cooldown.saturating_sub(1);
            let attack_cycle = (effective_foreswing + cooldown_frames).max(animation_frames);
            if attack_cycle > 0 { ((total_attack_damage as f32 * 30.0) / attack_cycle as f32).round() as i32 } else { 0 }
        },
        formatter: |dps| format!("{}", dps),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Atk Cycle",
        display_name: "Atk Cycle",
        get_value: |stats, animation_frames, _| {
            let mut effective_foreswing = stats.time_until_attack_1;
            if stats.attack_3 > 0 && stats.time_until_attack_3 > 0 { effective_foreswing = stats.time_until_attack_3; }
            else if stats.attack_2 > 0 && stats.time_until_attack_2 > 0 { effective_foreswing = stats.time_until_attack_2; }
            let cooldown_frames = stats.attack_cooldown.saturating_sub(1);
            (effective_foreswing + cooldown_frames).max(animation_frames)
        },
        formatter: |frames| format!("{}f", frames),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Rarity",
        display_name: "Rarity",
        get_value: |_, _, unitbuy| {
            if let Some(unitbuy) = unitbuy {
                unitbuy.rarity as i32
            } else {
                -1
            }
        },
        formatter: |rarity_val| match rarity_val {
            0 => "N".to_string(),
            1 => "EX".to_string(),
            2 => "RR".to_string(),
            3 => "SR".to_string(),
            4 => "UR".to_string(),
            5 => "LR".to_string(),
            _ => "??".to_string(),
        },
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Cost",
        display_name: "Cost",
        get_value: |stats, _, _| (stats.eoc1_cost as f32 * 1.5).round() as i32,
        formatter: |cost| format!("{}¢", cost),
        linked_talent_id: Some(25),
        talent_modifier_fmt: Some(|reduction, _| format!("(-{}¢)", (reduction as f32 * 1.5).round() as i32)),
    },
    CatStatsDef {
        name: "Cooldown",
        display_name: "Cooldown",
        get_value: |stats, _, _| (stats.cooldown - 264).max(60),
        formatter: |cd| format!("{:.2}s^{}f", cd as f32 / 30.0, cd),
        linked_talent_id: Some(26),
        talent_modifier_fmt: Some(|frames, _| format!("(-{}f)", frames)),
    },
    CatStatsDef {
        name: "TBA",
        display_name: "TBA",
        get_value: |stats, _, _| stats.attack_cooldown,
        formatter: |tba| format!("{}f", tba),
        linked_talent_id: Some(61),
        talent_modifier_fmt: Some(|percent, _| format!("(-{}%)", percent)),
    },
];

// --- REGISTRY HELPER FUNCTIONS ---

pub fn get_cat_stat(name: &str) -> &'static CatStatsDef {
    CAT_STATS_REGISTRY.iter().find(|stat_definition| stat_definition.name == name).expect("CRITICAL: Hardcoded stat name was not found in CAT_STATS_REGISTRY")
}

pub fn format_cat_stat(name: &str, stats: &Battle, animation_frames: i32, unitbuy: Option<&UnitBuy>) -> String {
    let stat_definition = get_cat_stat(name);
    (stat_definition.formatter)((stat_definition.get_value)(stats, animation_frames, unitbuy))
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