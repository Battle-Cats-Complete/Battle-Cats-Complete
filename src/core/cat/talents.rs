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
            if min != 0 || max != 0 {
                params.push((min, max));
            }
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

// Helper: Interpolate between min and max based on level (1 to max_level)
fn calc_current_value(min: u16, max: u16, level: u8, max_level: u8) -> i32 {
    if level == 0 { return 0; }
    // If max level is 1, we just return the min value (unlocks immediately)
    if max_level <= 1 { return min as i32; }
    // If we are at level 1, return min
    if level == 1 { return min as i32; }
    // If we are at max level, return max
    if level == max_level { return max as i32; }

    let min_f = min as f32;
    let max_f = max as f32;
    let lvl_f = level as f32;
    let max_lvl_f = max_level as f32;

    // Linear Interpolation
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
    
    let get_val = |min, max| calc_current_value(min, max, talent_level, group.max_level);

    // Format 1: Additive -> "{base} (+{bonus}) -> {final}"
    let fmt_additive = |base: i32, bonus: i32, unit: &str| -> String {
        format!("{}{} (+{}{}) -> {}{}", base, unit, bonus, unit, base + bonus, unit)
    };

    // Format 2: Multiplicative (HP/Atk) -> "{base} (+{pct}%) -> {final}"
    let fmt_multi = |base: i32, pct: i32| -> String {
        let bonus_val = (base as f32 * (pct as f32 / 100.0)).round() as i32;
        format!("{} (+{}%) -> {}", base, pct, base + bonus_val)
    };

    // Format 3: State -> "{base} -> {current}"
    let fmt_state = || -> String {
        if talent_level > 0 {
            "Inactive -> Active".to_string()
        } else {
            "Inactive -> Inactive".to_string()
        }
    };

    match group.text_id {
        // --- ADDITIVE NUMERICAL TALENTS ---
        
        // Weaken Duration
        1 | 70 | 71 => {
            let bonus = get_val(group.min_2, group.max_2); 
            Some(fmt_additive(stats.weaken_duration, bonus, "f"))
        },
        42 => { // Upgrade Weaken Dur
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.weaken_duration, bonus, "f"))
        },
        
        // Freeze Duration
        2 | 76 => {
            let bonus = get_val(group.min_2, group.max_2);
            Some(fmt_additive(stats.freeze_duration, bonus, "f"))
        },
        43 => { // Upgrade Freeze Dur
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_duration, bonus, "f"))
        },
        
        // Freeze Chance
        74 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.freeze_chance, bonus, "%"))
        },

        // Slow Duration
        3 | 69 | 72 => {
            let bonus = get_val(group.min_2, group.max_2);
            Some(fmt_additive(stats.slow_duration, bonus, "f"))
        },
        44 => { // Upgrade Slow Dur
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_duration, bonus, "f"))
        },
        
        // Slow Chance
        63 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.slow_chance, bonus, "%"))
        },

        // Knockback Chance
        8 | 73 | 75 => {
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

        // Strengthen (Boost %)
        10 => {
            let bonus = get_val(group.min_2, group.max_2);
            Some(fmt_additive(stats.strengthen_boost, bonus, "%"))
        },
        46 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.strengthen_boost, bonus, "%"))
        },

        // Survive %
        11 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },
        47 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.survive, bonus, "%"))
        },

        // Critical %
        13 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },
        48 | 52 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.critical_chance, bonus, "%"))
        },
        
        // Speed (Flat)
        29 => { 
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.speed, bonus, ""))
        },
        
        // Cooldown (Reduction)
        32 => { 
            let reduction = get_val(group.min_1, group.max_1);
            let base = stats.effective_cooldown();
            Some(format!("{}f (-{}f) -> {}f", base, reduction, base.saturating_sub(reduction)))
        },
        
        // Resistances
        18 | 19 | 20 | 21 | 22 | 26 | 64 | 66 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(0, bonus, "%"))
        },

        // --- MULTIPLICATIVE (HP/ATK) ---
        
        // Health Buff
        27 => { 
            let pct = get_val(group.min_1, group.max_1);
            let base_hp = curve.map_or(stats.hitpoints, |c| c.calculate_stat(stats.hitpoints, unit_level));
            Some(fmt_multi(base_hp, pct))
        },
        
        // Attack Buff
        28 => { 
            let pct = get_val(group.min_1, group.max_1);
            let total_base = stats.attack_1 + stats.attack_2 + stats.attack_3;
            let real_atk = curve.map_or(total_base, |c| c.calculate_stat(total_base, unit_level));
            Some(fmt_multi(real_atk, pct))
        },

        // --- STATE / BOOLEAN ---
        23 | 25 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 
        53 | 54 | 55 | 56 | 58 | 65 | 67 => {
            Some(fmt_state())
        },

        // Default Fallback
        _ => None,
    }
}