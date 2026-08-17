use nyanko::combat::{Entity, Faction, Identity, REGISTRY};

use crate::systems::combat::comparable;
use crate::systems::combat::registry::{get_display_def, Magnification, StatContext, ENEMY_STATS_REGISTRY};
use crate::domains::enemy::filter::{EnemyFilterState, MatchMode};
use crate::domains::enemy::scanner::EnemyEntry;

pub(crate) fn get_stat_value(s: &Entity, stat: &str, anim_frames: i32, mag: i32) -> i32 {
    let reg_name = match stat {
        "Atk Cycle (f)" => "Atk Cycle",
        _ => stat,
    };

    let magnification = Magnification { hitpoints: mag, attack: mag };

    ENEMY_STATS_REGISTRY.iter()
        .find(|d| d.name == reg_name)
        .map_or(0, |def| (def.get_value)(&StatContext::enemy(s, anim_frames, magnification)))
}

pub fn get_identity_name(identity: Identity) -> String {
    get_display_def(identity, Faction::Enemy).name.to_string()
}

pub(crate) fn has_trait_or_ability(s: &Entity, identity: Identity) -> bool {
    REGISTRY.iter().find(|d| d.identity == identity).is_some_and(|def| {
        !(def.attributes)(s).is_empty()
    })
}

pub fn entity_passes_filter(enemy: &EnemyEntry, filter: &EnemyFilterState) -> bool {
    let mag = filter.mag_input.parse::<i32>().unwrap_or(100);
    let has_stat_filters = filter.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty());
    let has_identity_filters = !filter.active_identities.is_empty();

    if !has_stat_filters && !has_identity_filters {
        return true;
    }

    let stats = &enemy.stats;
    let mut active_conditions = 0;
    let mut passed_conditions = 0;
    let mut failed_conditions = 0;

    if has_stat_filters {
        for (stat_name, range) in &filter.stat_ranges {
            if range.min.is_empty() && range.max.is_empty() { continue; }
            active_conditions += 1;

            let val = get_stat_value(stats, stat_name, enemy.atk_anim_frames, mag);

            let r_min = range.min.parse::<i32>().unwrap_or(i32::MIN);
            let r_max = range.max.parse::<i32>().unwrap_or(i32::MAX);

            if val <= r_max && val >= r_min {
                passed_conditions += 1;
            } else {
                failed_conditions += 1;
            }
        }
    }

    if has_identity_filters {
        for &identity in &filter.active_identities {
            active_conditions += 1;

            let has_inherent = has_trait_or_ability(stats, identity);
            let mut identity_passed = false;

            let ability_def = REGISTRY.iter().find(|d| d.identity == identity);

            if has_inherent {
                if let Some(adv_map) = filter.adv_ranges.get(&identity) {
                    let mut build_passed_all_attrs = true;

                    let attrs = ability_def.map(|def| (def.attributes)(stats)).unwrap_or_default();

                    for (attr, range) in adv_map {
                        let val = attrs.iter()
                            .find(|(k, _, _)| k == attr)
                            .map_or(0, |(_, v, _)| comparable(*v));

                        if let Ok(min) = range.min.parse::<i32>()
                            && val < min {
                            build_passed_all_attrs = false;
                            break;
                        }

                        if let Ok(max) = range.max.parse::<i32>()
                            && val > max {
                            build_passed_all_attrs = false;
                            break;
                        }
                    }

                    if build_passed_all_attrs {
                        identity_passed = true;
                    }
                } else {
                    identity_passed = true;
                }
            }

            if identity_passed {
                passed_conditions += 1;
            } else {
                failed_conditions += 1;
            }
        }
    }

    if active_conditions == 0 {
        return true;
    }

    if filter.match_mode == MatchMode::And {
        failed_conditions == 0
    } else {
        passed_conditions > 0
    }
}