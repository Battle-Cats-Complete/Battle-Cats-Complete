use crate::core::files::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::core::files::unitid::CatRaw;
use crate::core::files::unitlevel::CatLevelCurve;
use std::collections::HashMap;

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
                0 => Some(TalentTarget::Red),
                1 => Some(TalentTarget::Floating),
                2 => Some(TalentTarget::Black),
                3 => Some(TalentTarget::Metal),
                4 => Some(TalentTarget::Angel),
                5 => Some(TalentTarget::Alien),
                6 => Some(TalentTarget::Zombie),
                7 => Some(TalentTarget::Relic),
                8 => Some(TalentTarget::Traitless),
                9 => Some(TalentTarget::Witch), 
                10 => Some(TalentTarget::Eva),
                11 => Some(TalentTarget::Aku),
                _ => None,
            };
            if let Some(target) = t {
                targets.push(target);
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

    let fmt_range = |min: i32, max: i32| -> String {
        if min == max {
            format!("Range: {}", min)
        } else {
            format!("Range: {}~{}", min, max)
        }
    };

    match group.text_id {
        // --- ADDITIVE WITH CHANCE ---
        1 | 70 | 71 => { // Weaken
            let chance = group.min_1; 
            let bonus = get_val(group.min_2, group.max_2); 
            Some(format!("{}\nChance: {}%", fmt_additive(stats.weaken_duration, bonus, "f"), chance))
        },
        42 => { // Upgrade Weaken
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.weaken_duration, bonus, "f"))
        },
        2 | 76 => { // Freeze
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.freeze_duration, bonus, "f"), chance))
        },
        43 => { // Upgrade Freeze
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_duration, bonus, "f"))
        },
        74 => { // Upgrade Freeze Chance
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_chance, bonus, "%"))
        },
        3 | 69 | 72 => { // Slow
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.slow_duration, bonus, "f"), chance))
        },
        44 => { // Upgrade Slow
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_duration, bonus, "f"))
        },
        63 => { // Upgrade Slow Chance
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_chance, bonus, "%"))
        },
        8 | 73 | 75 => { // Knockback
            let mut bonus = get_val(group.min_1, group.max_1);
            if bonus == 0 && group.min_1 == 0 {
                bonus = get_val(group.min_2, group.max_2);
            }
            Some(fmt_additive(stats.knockback_chance, bonus, "%"))
        },
        45 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.knockback_chance, bonus, "%"))
        },
        10 => { // Strengthen
            let hp_limit = 100 - group.min_1; 
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nTrigger at: {}% HP", fmt_additive(stats.strengthen_boost, bonus, "%"), hp_limit))
        },
        46 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.strengthen_boost, bonus, "%"))
        },
        11 => { // Survive
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },
        47 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },
        13 => { // Critical
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },
        48 | 52 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },
        15 => { // Barrier Breaker
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.barrier_breaker_chance, bonus, "%"))
        },
        49 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.barrier_breaker_chance, bonus, "%"))
        },
        17 => { // Wave
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            Some(format!("{}\nLevel: {}\nRange: {}", fmt_additive(stats.wave_chance, bonus, "%"), level, range))
        },
        50 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.wave_chance, bonus, "%"))
        },
        31 => { // Cost Down
            let reduction = get_val(group.min_1, group.max_1);
            let base = stats.eoc1_cost * 3 / 2;
            Some(format!("{}¢ (-{}¢) -> {}¢", base, reduction, base.saturating_sub(reduction as i32)))
        },
        32 => { // Recover Speed
            let reduction = get_val(group.min_1, group.max_1);
            let base = stats.effective_cooldown();
            Some(format!("{}f (-{}f) -> {}f", base, reduction, base.saturating_sub(reduction)))
        },
        59 => { // Savage Blow
            let bonus = get_val(group.min_1, group.max_1);
            let dmg_boost = group.min_2;
            Some(format!("{}\nDamage Boost: +{}%", fmt_additive(stats.savage_blow_chance, bonus, "%"), dmg_boost))
        },
        61 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.savage_blow_chance, bonus, "%"))
        },
        60 | 84 | 87 => { // Dodge
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.dodge_duration, bonus, "f"), chance))
        },
        62 | 81 => { // Upgrade Dodge
            if group.min_1 != group.max_1 {
                 let bonus = get_val(group.min_1, group.max_1);
                 Some(fmt_additive(stats.dodge_chance, bonus, "%"))
            } else {
                 let bonus = get_val(group.min_2, group.max_2);
                 Some(fmt_additive(stats.dodge_duration, bonus, "f"))
            }
        },
        68 => { // Surge
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let min_range = group.min_3 / 4;
            let max_range = min_range + (group.min_4 / 4);
            Some(format!("{}\nLevel: {}\n{}", fmt_additive(stats.surge_chance, bonus, "%"), level, fmt_range(min_range as i32, max_range as i32)))
        },
        78 => { // Shield Pierce
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.shield_pierce_chance, bonus, "%"))
        },
        80 => { // Curse
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nChance: {}%", fmt_additive(stats.curse_duration, bonus, "f"), chance))
        },
        93 => { // Upgrade Curse
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.curse_duration, bonus, "f"))
        },
        82 => { // Attack Freq
            let reduction = get_val(group.min_1, group.max_1);
            Some(format!("TBA: -{}f", reduction))
        },
        83 => { // Mini-Wave
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            Some(format!("{}\nLevel: {} (Mini)\nRange: {}", fmt_additive(stats.wave_chance, bonus, "%"), level, range))
        },
        86 => { // Behemoth Slayer
            let chance = group.min_1;
            let duration = group.min_2;
            Some(format!("Inactive -> Active\n{}% Chance to Dodge for {}f", chance, duration))
        },
        89 => { // Mini-Surge
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let min_range = group.min_3 / 4;
            let max_range = min_range + (group.min_4 / 4);
            Some(format!("{}\nLevel: {} (Mini)\n{}", fmt_additive(stats.surge_chance, bonus, "%"), level, fmt_range(min_range as i32, max_range as i32)))
        },
        88 | 90 | 95 => { // Unlock Dodge
            let bonus = get_val(group.min_1, group.max_1);
            let duration = group.min_2;
            Some(format!("{}\nDuration: {}f", fmt_additive(stats.dodge_chance, bonus, "%"), duration))
        },
        94 => { // Explosion
            let bonus = get_val(group.min_1, group.max_1);
            let min_range = group.min_2 / 4;
            // Explosion usually just has an anchor, checking if span (idx 3) exists or if anchor represents range
            let max_range = min_range + (group.min_3 / 4); // Assuming min_3 is span if it exists
            Some(format!("{}\n{}", fmt_additive(stats.explosion_chance, bonus, "%"), fmt_range(min_range as i32, max_range as i32)))
        },
        29 => { // Speed
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.speed, bonus, ""))
        },
        18 | 19 | 20 | 21 | 22 | 26 | 64 | 66 => { // Resistances
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(0, bonus, "%"))
        },
        // --- MULTIPLICATIVE (HP/ATK) ---
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
        _ => None,
    }
}

// --- APPLY TALENT STATS TO BASE ---
pub fn apply_talent_stats(base: &CatRaw, talent_data: &TalentRaw, levels: &HashMap<u8, u8>) -> CatRaw {
    let mut s = base.clone();
    
    for (idx, group) in talent_data.groups.iter().enumerate() {
        let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
        
        // --- TARGET TRAIT MAPPING ---
        if lv > 0 && group.name_id != -1 {
            match group.name_id {
                0 => s.target_red = 1,
                1 => s.target_floating = 1,
                2 => s.target_black = 1,
                3 => s.target_metal = 1,
                4 => s.target_angel = 1,
                5 => s.target_alien = 1,
                6 => s.target_zombie = 1,
                7 => s.target_relic = 1,
                8 => s.target_traitless = 1,
                9 => s.target_witch = 1,
                10 => s.target_eva = 1,
                11 => s.target_aku = 1,
                _ => {}
            }
        }

        if lv == 0 { continue; }
        
        let val = calc_val_helper(group.min_1, group.max_1, lv, group.max_level);
        let val2 = calc_val_helper(group.min_2, group.max_2, lv, group.max_level);

        match group.ability_id {
            1 => { // Weaken
                if s.weaken_chance == 0 {
                    s.weaken_chance = group.min_1 as i32; 
                    s.weaken_duration = val2;
                    s.weaken_to = group.min_3 as i32; 
                } else {
                    s.weaken_duration += val; 
                }
            },
            2 => { // Freeze
                if s.freeze_chance == 0 {
                    s.freeze_chance = group.min_1 as i32;
                    s.freeze_duration = val2;
                } else {
                    s.freeze_duration += val;
                }
            },
            3 => { // Slow
                if s.slow_chance == 0 {
                    s.slow_chance = group.min_1 as i32;
                    s.slow_duration = val2;
                } else {
                    s.slow_duration += val;
                }
            },
            8 => s.knockback_chance += val,
            10 => { // Strengthen
                s.strengthen_threshold = (100 - group.min_1) as i32;
                s.strengthen_boost += val2;
            },
            11 => s.survive += val,
            13 => s.critical_chance += val,
            15 => s.barrier_breaker_chance += val,
            17 => { // Wave
                s.wave_chance += val;
                s.wave_level = group.min_2 as i32;
            },
            25 => s.eoc1_cost -= val, 
            26 => s.cooldown -= val,
            27 => s.speed += val, 
            31 => { // Attack Buff
                let factor = (100 + val) as f32 / 100.0;
                s.attack_1 = (s.attack_1 as f32 * factor) as i32;
                s.attack_2 = (s.attack_2 as f32 * factor) as i32;
                s.attack_3 = (s.attack_3 as f32 * factor) as i32;
            },
            32 => { // Health Buff
                let factor = (100 + val) as f32 / 100.0;
                s.hitpoints = (s.hitpoints as f32 * factor) as i32;
            },
            50 => { // Savage Blow
                s.savage_blow_chance += val;
                s.savage_blow_boost = group.min_2 as i32;
            },
            51 => { // Dodge
                s.dodge_chance += val;
                s.dodge_duration += val2;
            },
            56 => { // Surge
                s.surge_chance += val;
                s.surge_level = group.min_2 as i32;
                // Params 3 & 4 contain raw range (4x game units)
                s.surge_spawn_anchor = group.min_3 as i32 / 4; 
                s.surge_spawn_span = group.min_4 as i32 / 4;   
            },
            58 => s.shield_pierce_chance += val,
            60 => { // Curse
                s.curse_chance += val;
                s.curse_duration += val2;
            },
            61 => { // TBA
                s.time_before_attack_1 = s.time_before_attack_1.saturating_sub(val);
            },
            62 => { // Mini-Wave
                s.mini_wave_flag = 1;
                s.wave_chance += val;
                s.wave_level = group.min_2 as i32;
            },
            65 => { // Mini-Surge
                s.mini_surge_flag = 1;
                s.surge_chance += val;
                s.surge_level = group.min_2 as i32;
                s.surge_spawn_anchor = group.min_3 as i32 / 4; 
                s.surge_spawn_span = group.min_4 as i32 / 4;
            },
            67 => { // Explosion
                s.explosion_chance += val;
                s.explosion_spawn_anchor = group.min_2 as i32 / 4; 
                s.explosion_spawn_span = group.min_3 as i32 / 4;
            },
            
            // Immunities & Flags
            23 => s.wave_immune = 1,
            29 => s.curse_immune = 1, 
            44 => s.weaken_immune = 1,
            45 => s.freeze_immune = 1,
            46 => s.slow_immune = 1,
            47 => s.knockback_immune = 1,
            48 => s.wave_immune = 1,
            49 => s.warp_immune = 1,
            53 => s.toxic_immune = 1,
            55 => s.surge_immune = 1,
            57 => s.target_aku = 1,
            33 => s.target_red = 1,
            34 => s.target_floating = 1,
            35 => s.target_black = 1,
            36 => s.target_metal = 1,
            37 => s.target_angel = 1,
            38 => s.target_alien = 1,
            39 => s.target_zombie = 1,
            40 => s.target_relic = 1,
            41 => s.target_traitless = 1,
            _ => {}
        }
    }
    s
}