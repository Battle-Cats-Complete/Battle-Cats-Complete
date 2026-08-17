use std::collections::HashMap;

use nyanko::cat::unit::{LevelCurve, Talent};
use nyanko::combat::Entity;

use crate::domains::cat::game::talents;
use crate::domains::cat::scanner::CatEntry;
use crate::domains::settings::Settings;

pub use crate::domains::cat::waiter::unitid;

pub fn seeded_level(cat: &CatEntry, settings: &Settings) -> (i32, String) {
    if !settings.cat_data.auto_level_calculations {
        let default_level = settings.cat_data.default_level.max(1);
        return (default_level, default_level.to_string());
    }

    let base_max = cat.unitbuy.level_cap_catseye;
    let plus_max = cat.unitbuy.level_cap_plus;
    let is_legend_rare = cat.unitbuy.rarity == 5;
    let is_normal_rare = cat.unitbuy.rarity == 0;

    if is_legend_rare {
        (50, "50".to_string())
    } else if base_max == 1 || (5..=65).contains(&plus_max) || is_normal_rare {
        let input = if plus_max > 0 {
            format!("{}+{}", base_max, plus_max)
        } else {
            base_max.to_string()
        };
        (base_max + plus_max, input)
    } else if base_max > 50 {
        (50, "50".to_string())
    } else {
        (base_max, base_max.to_string())
    }
}

pub(crate) fn apply_level(base_stats: &Entity, curve: Option<&LevelCurve>, level: i32) -> Entity {
    let mut s = base_stats.clone();
    if let Some(c) = curve {
        s.hitpoints = c.calculate_stat(s.hitpoints, level);
        s.attack_1 = c.calculate_stat(s.attack_1, level);
        s.attack_2 = c.calculate_stat(s.attack_2, level);
        s.attack_3 = c.calculate_stat(s.attack_3, level);
    }
    s
}

pub fn get_final_stats(
    base_stats: &Entity,
    curve: Option<&LevelCurve>,
    level: i32,
    talent_data: Option<&Talent>,
    talent_levels: Option<&HashMap<u8, u8>>
) -> Entity {
    let leveled = apply_level(base_stats, curve, level);
    if let (Some(t_data), Some(levels)) = (talent_data, talent_levels) {
        talents::apply_talent_stats(&leveled, t_data, levels)
    } else {
        leveled
    }
}