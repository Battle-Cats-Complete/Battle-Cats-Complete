use std::collections::HashMap;

use nyanko::cat::unit::{Battle, LevelCurve, Talent};

use crate::modules::cat::game::talents;

pub use crate::modules::cat::waiter::unitid;

pub fn apply_level(base_stats: &Battle, curve: Option<&LevelCurve>, level: i32) -> Battle {
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
    base_stats: &Battle,
    curve: Option<&LevelCurve>,
    level: i32,
    talent_data: Option<&Talent>,
    talent_levels: Option<&HashMap<u8, u8>>
) -> Battle {
    let leveled = apply_level(base_stats, curve, level);
    if let (Some(t_data), Some(levels)) = (talent_data, talent_levels) {
        talents::apply_talent_stats(&leveled, t_data, levels)
    } else {
        leveled
    }
}