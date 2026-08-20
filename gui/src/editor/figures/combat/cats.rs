use crate::editor::figures::resolved::Rule;

pub(in crate::editor::figures) fn rule(field: &str) -> Option<Rule> {
    match field {
        "hitpoints" | "knockbacks" | "speed" | "attack_1" | "attack_cooldown" | "standing_range"
        | "eoc1_cost" | "cooldown" | "trait_red" | "area_attack" | "time_until_attack_1" | "minimum_z_layer"
        | "maximum_z_layer" | "trait_floating" | "trait_dark" | "trait_metal" | "trait_traitless"
        | "trait_angel" | "trait_alien" | "trait_zombie" | "strong_against" | "knockback_chance"
        | "freeze_chance" | "freeze_duration" | "slow_chance" | "slow_duration" | "resist"
        | "massive_damage" | "critical_chance" | "attack_only" | "double_bounty" | "base_destroyer"
        | "wave_chance" | "wave_level" | "weaken_chance" | "weaken_duration" | "weaken_to"
        | "strengthen_threshold" | "strengthen_boost" | "survive" | "is_metal" | "long_distance_1_anchor"
        | "long_distance_1_span" | "wave_immune" | "wave_block" | "knockback_immune" | "freeze_immune"
        | "slow_immune" | "weaken_immune" | "zombie_killer" | "witch_killer" | "trait_witch"
        | "boss_wave_immune" | "attack_2" | "attack_3" | "time_until_attack_2" | "time_until_attack_3"
        | "attack_1_abilities" | "attack_2_abilities" | "attack_3_abilities" | "spawn_animation_flag"
        | "use_gudetama_soul" | "barrier_breaker_chance" | "warp_chance" | "warp_duration"
        | "warp_distance_minimum" | "warp_distance_maximum" | "warp_immune" | "trait_eva" | "eva_killer"
        | "trait_relic" | "curse_immune" | "insanely_tough" | "insane_damage" | "savage_blow_chance"
        | "savage_blow_boost" | "dodge_chance" | "dodge_duration" | "surge_chance" | "surge_spawn_anchor"
        | "surge_spawn_span" | "surge_level" | "toxic_immune" | "surge_immune" | "curse_chance"
        | "curse_duration" | "mini_wave_flag" | "shield_pierce_chance" | "trait_aku" | "colossus_slayer"
        | "soulstrike" | "long_distance_2_flag" | "long_distance_2_anchor" | "long_distance_2_span"
        | "long_distance_3_flag" | "long_distance_3_anchor" | "long_distance_3_span" | "behemoth_slayer"
        | "behemoth_dodge_chance" | "behemoth_dodge_duration" | "mini_surge_flag" | "counter_surge"
        | "conjure_unit_id" | "sage_slayer" | "metal_killer_percent" | "explosion_chance"
        | "explosion_spawn_anchor" | "explosion_spawn_span" | "explosion_immune" => Some(Rule::Plain),

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
    fn every_cat_column_has_a_rule() {
        for entry in schema::of(Subject::Cat).order() {
            assert!(
                rule(entry.field).is_some(),
                "cat: nyanko publishes {}, which no resolved rule arm names",
                entry.field,
            );
        }
    }
}
