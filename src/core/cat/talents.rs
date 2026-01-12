use crate::core::files::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::core::files::unitid::CatRaw;
use crate::core::files::unitlevel::CatLevelCurve;

#[derive(Debug, Clone)]
pub struct CatTalents {
    pub unit_id: u16,
    pub implicit_targets: Vec<TalentTarget>,
    pub normal: Vec<SingleTalent>,
    pub ultra: Vec<SingleTalent>,
}

#[derive(Debug, Clone)]
pub struct SingleTalent {
    pub ability_id: u8,
    pub max_level: u8,
    pub params: Vec<(u16, u16)>, 
    pub description_id: u8,
    pub cost_id: u8,
    pub name_id: i16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TalentTarget {
    Red = 0,
    Floating = 1,
    Black = 2,
    Metal = 3,
    Angel = 4,
    Alien = 5,
    Zombie = 6,
    Relic = 7,
    Traitless = 8,
    Witch = 9,
    Eva = 10,
    Aku = 11,
    Unknown = 99,
}

impl CatTalents {
    pub fn from_raw(raw: &TalentRaw) -> Self {
        let mut normal = Vec::new();
        let mut ultra = Vec::new();

        for group in &raw.groups {
            let talent = SingleTalent::from_raw(group);
            if group.limit == 1 {
                ultra.push(talent);
            } else {
                normal.push(talent);
            }
        }

        Self {
            unit_id: raw.id,
            implicit_targets: parse_targets(raw.type_id),
            normal,
            ultra,
        }
    }
}

impl SingleTalent {
    pub fn from_raw(group: &TalentGroupRaw) -> Self {
        let mut params = Vec::new();
        for (min, max) in [
            (group.min_1, group.max_1),
            (group.min_2, group.max_2),
            (group.min_3, group.max_3),
            (group.min_4, group.max_4),
        ] {
            // We keep even 0,0 pairs because indices matter for some talents (e.g. Surge ranges)
            params.push((min, max));
        }

        Self {
            ability_id: group.ability_id,
            max_level: group.max_level,
            params,
            description_id: group.text_id,
            cost_id: group.cost_id,
            name_id: group.name_id,
        }
    }
}

fn parse_targets(mask: u16) -> Vec<TalentTarget> {
    let mut targets = Vec::new();
    for i in 0..12 {
        if (mask & (1 << i)) != 0 {
            let t = match i {
                0 => TalentTarget::Red,
                1 => TalentTarget::Floating,
                2 => TalentTarget::Black,
                3 => TalentTarget::Metal,
                4 => TalentTarget::Angel,
                5 => TalentTarget::Alien,
                6 => TalentTarget::Zombie,
                7 => TalentTarget::Relic,
                8 => TalentTarget::Traitless,
                9 => TalentTarget::Witch, 
                10 => TalentTarget::Eva,
                11 => TalentTarget::Aku,
                _ => TalentTarget::Unknown,
            };
            if t != TalentTarget::Unknown {
                targets.push(t);
            }
        }
    }
    targets
}

// --- CALCULATION LOGIC ---

fn calc_val_helper(min: u16, max: u16, level: u8, max_level: u8) -> i32 {
    if level == 0 { return 0; }
    if max_level <= 1 { return min as i32; }
    if level == 1 { return min as i32; }
    if level == max_level { return max as i32; }

    let min_f = min as f32;
    let max_f = max as f32;
    let lvl_f = level as f32;
    let max_lvl_f = max_level as f32;

    let val = min_f + (max_f - min_f) * (lvl_f - 1.0) / (max_lvl_f - 1.0);
    val.round() as i32
}

pub fn calculate_talent_display(
    group: &TalentGroupRaw, 
    stats: &CatRaw, 
    talent_level: u8, 
    curve: Option<&CatLevelCurve>, 
    unit_level: i32
) -> Option<String> {
    
    let get_val = |min, max| calc_val_helper(min, max, talent_level, group.max_level);

    let fmt_additive = |base: i32, bonus: i32, unit: &str| -> String {
        format!("{}{} (+{}{}) -> {}{}", base, unit, bonus, unit, base + bonus, unit)
    };

    let fmt_multi = |base: i32, pct: i32| -> String {
        let bonus_val = (base as f32 * (pct as f32 / 100.0)).round() as i32;
        format!("{} (+{}%) -> {}", base, pct, base + bonus_val)
    };

    let fmt_state = || -> String {
        if talent_level > 0 {
            "Inactive -> Active".to_string()
        } else {
            "Inactive -> Inactive".to_string()
        }
    };

    match group.text_id {
        // --- ADDITIVE WITH CHANCE ---
        
        // Weaken Duration (1, 70, 71)
        // Param 0: Chance (Static), Param 1: Duration (Scales)
        1 | 70 | 71 => {
            let chance = group.min_1; // Static chance
            let bonus = get_val(group.min_2, group.max_2); 
            Some(format!("{}\nChance: {}%", fmt_additive(stats.weaken_duration, bonus, "f"), chance))
        },
        // Upgrade Weaken (42) - Duration only
        42 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.weaken_duration, bonus, "f"))
        },

        // Freeze Duration (2, 76)
        // Param 0: Chance, Param 1: Duration
        2 | 76 => {
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.freeze_duration, bonus, "f"), chance))
        },
        // Upgrade Freeze (43)
        43 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_duration, bonus, "f"))
        },
        // Upgrade Freeze Chance (74)
        74 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_chance, bonus, "%"))
        },

        // Slow Duration (3, 69, 72)
        // Param 0: Chance, Param 1: Duration
        3 | 69 | 72 => {
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.slow_duration, bonus, "f"), chance))
        },
        // Upgrade Slow (44)
        44 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_duration, bonus, "f"))
        },
        // Upgrade Slow Chance (63)
        63 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_chance, bonus, "%"))
        },

        // Knockback Chance (8, 73, 75)
        // Param 0: Chance (Scales)
        8 | 73 | 75 => {
            let mut bonus = get_val(group.min_1, group.max_1);
            // Fallback for data irregularity where min_1 is 0 but min_2 has data
            if bonus == 0 && group.min_1 == 0 {
                bonus = get_val(group.min_2, group.max_2);
            }
            // Usually unlocks from 0, so base is 0
            Some(fmt_additive(stats.knockback_chance, bonus, "%"))
        },
        // Upgrade Knockback (45)
        45 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.knockback_chance, bonus, "%"))
        },

        // Strengthen (10)
        // Param 0: HP Threshold (Static), Param 1: Boost (Scales)
        10 => {
            let hp_limit = 100 - group.min_1; // Data is usually inverted or raw
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nTrigger at: {}% HP", fmt_additive(stats.strengthen_boost, bonus, "%"), hp_limit))
        },
        // Upgrade Strengthen (46)
        46 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.strengthen_boost, bonus, "%"))
        },

        // Survive (11)
        11 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },
        // Upgrade Survive (47)
        47 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },

        // Critical (13)
        13 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },
        // Upgrade Critical (48, 52)
        48 | 52 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },

        // Barrier Breaker (15)
        15 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.barrier_breaker_chance, bonus, "%"))
        },
        // Upgrade Barrier Breaker (49)
        49 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.barrier_breaker_chance, bonus, "%"))
        },

        // Wave (17)
        // Param 0: Chance, Param 1: Level
        17 => {
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            Some(format!("{}\nLevel: {}", fmt_additive(stats.wave_chance, bonus, "%"), level))
        },
        // Upgrade Wave (50)
        50 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.wave_chance, bonus, "%"))
        },

        // Cost Down (31)
        31 => {
            let reduction = get_val(group.min_1, group.max_1);
            // Inverted additive logic: Base - Reduction
            let base = stats.eoc1_cost * 3 / 2; // Approx Chp3 cost
            Some(format!("{}¢ (-{}¢) -> {}¢", base, reduction, base.saturating_sub(reduction as i32)))
        },

        // Recover Speed (32)
        32 => {
            let reduction = get_val(group.min_1, group.max_1);
            let base = stats.effective_cooldown();
            Some(format!("{}f (-{}f) -> {}f", base, reduction, base.saturating_sub(reduction)))
        },

        // Savage Blow (59)
        // Param 0: Chance, Param 1: Boost
        59 => {
            let bonus = get_val(group.min_1, group.max_1);
            let dmg_boost = group.min_2;
            Some(format!("{}\nDamage Boost: +{}%", fmt_additive(stats.savage_blow_chance, bonus, "%"), dmg_boost))
        },
        // Upgrade Savage Blow (61)
        61 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.savage_blow_chance, bonus, "%"))
        },

        // Dodge (60, 84, 87)
        // Param 0: Chance, Param 1: Duration
        60 | 84 | 87 => {
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.dodge_duration, bonus, "f"), chance))
        },
        // Upgrade Dodge (62, 81)
        62 | 81 => {
            // Check if upgrading chance or duration. Usually depends on which param scales.
            if group.min_1 != group.max_1 {
                 let bonus = get_val(group.min_1, group.max_1);
                 Some(fmt_additive(stats.dodge_chance, bonus, "%"))
            } else {
                 let bonus = get_val(group.min_2, group.max_2);
                 Some(fmt_additive(stats.dodge_duration, bonus, "f"))
            }
        },

        // Surge (68)
        // Param 0: Chance, Param 1: Level
        68 => {
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            Some(format!("{}\nLevel: {}", fmt_additive(stats.surge_chance, bonus, "%"), level))
        },

        // Shield Pierce (78)
        78 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.shield_pierce_chance, bonus, "%"))
        },

        // Curse (80)
        // Param 0: Chance, Param 1: Duration
        80 => {
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.curse_duration, bonus, "f"), chance))
        },
        // Upgrade Curse (93)
        93 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.curse_duration, bonus, "f"))
        },

        // Attack Frequency Up (82)
        // Reduces TBA
        82 => {
            let reduction = get_val(group.min_1, group.max_1);
            // This applies to time_before_attack usually?
            // Simplified display
            Some(format!("TBA: -{}f", reduction))
        },

        // Mini-Wave (83)
        83 => {
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            Some(format!("{}\nLevel: {}", fmt_additive(stats.wave_chance, bonus, "%"), level))
        },

        // Behemoth Slayer (86)
        // Param 0: Chance, Param 1: Duration (Dodge)
        86 => {
            let chance = group.min_1;
            let duration = group.min_2;
            Some(format!("Inactive -> Active\n{}% Chance to Dodge for {}f", chance, duration))
        },

        // Mini-Surge (89)
        89 => {
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            Some(format!("{}\nLevel: {}", fmt_additive(stats.surge_chance, bonus, "%"), level))
        },

        // Unlock Dodge (88, 90, 95)
        88 | 90 | 95 => {
            let bonus = get_val(group.min_1, group.max_1); // Chance scales?
            let duration = group.min_2;
            Some(format!("{}\nDuration: {}f", fmt_additive(stats.dodge_chance, bonus, "%"), duration))
        },

        // Explosion (94)
        94 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.explosion_chance, bonus, "%"))
        },

        // Speed (Flat)
        29 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.speed, bonus, ""))
        },
        
        // Resistances
        18 | 19 | 20 | 21 | 22 | 26 | 64 | 66 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(0, bonus, "%"))
        },

        // --- MULTIPLICATIVE (HP/ATK) ---
        // 27 (Health), 28 (Attack)
        27 => { 
            let pct = get_val(group.min_1, group.max_1);
            let base_hp = curve.map_or(stats.hitpoints, |c| c.calculate_stat(stats.hitpoints, unit_level));
            Some(fmt_multi(base_hp, pct))
        },
        28 => { 
            let pct = get_val(group.min_1, group.max_1);
            let total_base = stats.attack_1 + stats.attack_2 + stats.attack_3;
            let real_atk = curve.map_or(total_base, |c| c.calculate_stat(total_base, unit_level));
            Some(fmt_multi(real_atk, pct))
        },

        // --- STATE / BOOLEAN ---
        23 | 25 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 
        53 | 54 | 55 | 56 | 58 | 65 | 67 | 92 => {
            Some(fmt_state())
        },

        // Default Fallback
        _ => None,
    }
}