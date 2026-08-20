use crate::editor::figures::resolved::Rule;

pub(in crate::editor::figures) fn rule(field: &str) -> Option<Rule> {
    match field {
        "hitpoints" | "knockbacks" | "speed" | "attack_1" | "attack_cooldown" | "standing_range"
        | "cash_drop" | "trait_red" | "area_attack" | "time_until_attack_1" | "trait_floating"
        | "trait_dark" | "trait_metal" | "trait_traitless" | "trait_angel" | "trait_alien" | "trait_zombie"
        | "knockback_chance" | "freeze_chance" | "freeze_duration" | "slow_chance" | "slow_duration"
        | "critical_chance" | "base_destroyer" | "wave_chance" | "wave_level" | "weaken_chance"
        | "weaken_duration" | "weaken_to" | "strengthen_threshold" | "strengthen_boost" | "survive"
        | "long_distance_1_anchor" | "long_distance_1_span" | "wave_immune" | "wave_block"
        | "knockback_immune" | "freeze_immune" | "slow_immune" | "weaken_immune" | "burrow_amount"
        | "burrow_distance" | "revive_count" | "revive_time" | "revive_hp" | "trait_witch" | "trait_dojo"
        | "attack_2" | "attack_3" | "time_until_attack_2" | "time_until_attack_3" | "attack_1_abilities"
        | "attack_2_abilities" | "attack_3_abilities" | "spawn_animation_flag" | "use_gudetama_soul"
        | "barrier_hitpoints" | "warp_chance" | "warp_duration" | "warp_distance_minimum"
        | "warp_distance_maximum" | "trait_starred_alien" | "warp_immune" | "trait_eva" | "trait_relic"
        | "curse_chance" | "curse_duration" | "savage_blow_chance" | "savage_blow_boost" | "dodge_chance"
        | "dodge_duration" | "toxic_chance" | "toxic_damage" | "surge_chance" | "surge_spawn_anchor"
        | "surge_spawn_span" | "surge_level" | "surge_immune" | "mini_wave_flag" | "shield_hitpoints"
        | "shield_regen" | "death_surge_chance" | "death_surge_spawn_anchor" | "death_surge_spawn_span"
        | "death_surge_level" | "trait_aku" | "trait_colossus" | "long_distance_2_flag"
        | "long_distance_2_anchor" | "long_distance_2_span" | "long_distance_3_flag"
        | "long_distance_3_anchor" | "long_distance_3_span" | "trait_behemoth" | "mini_surge_flag"
        | "counter_surge" | "trait_sage" | "curse_immune" | "explosion_chance" | "explosion_spawn_anchor"
        | "explosion_spawn_span" | "explosion_immune" | "trait_kaijin" | "drain_chance" | "drain_percent" => Some(Rule::Plain),

        "hitbox_position" | "hitbox_width" | "unused" | "attack_count_total" | "time_before_death"
        | "attack_count_state" | "spawn_animation_type" | "soul_animation_type" => Some(Rule::Opaque),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::rule;
    use crate::editor::figures::schema::{self, Subject};

    #[test]
    fn every_enemy_column_has_a_rule() {
        for entry in schema::of(Subject::Enemy).order() {
            assert!(
                rule(entry.field).is_some(),
                "enemy: nyanko publishes {}, which no resolved rule arm names",
                entry.field,
            );
        }
    }
}
