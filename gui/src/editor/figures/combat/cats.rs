use crate::editor::figures::resolved::{Choice, Gate, Rule};

const COOLDOWN_MINIMUM: i32 = 60;

pub(in crate::editor::figures) fn rule(field: &str) -> Option<Rule> {
    match field {
        "area_attack" => {
            Some(Rule::Choice(&const { [Choice::new(0, "Single"), Choice::new(1, "Area")] }))
        }
        "attack_count_state" => Some(Rule::Gated(
            &const { [Choice::new(0, "Idle"), Choice::new(1, "Attacking"), Choice::new(2, "Self-Destruct")] },
            Gate {
                field: "attack_count_total",
                blocked: -1,
                reason: "Requires \"Attack Count Total\" to be greater than -1",
            },
        )),
        "cooldown" => Some(Rule::Floor(COOLDOWN_MINIMUM)),

        "trait_red" | "legacy_weak_against" | "trait_floating" | "trait_dark" | "trait_metal"
        | "trait_traitless" | "trait_angel" | "trait_alien" | "trait_zombie" | "strong_against" | "resist"
        | "massive_damage" | "attack_only" | "double_bounty" | "base_destroyer" | "is_metal" | "wave_immune"
        | "wave_block" | "knockback_immune" | "freeze_immune" | "slow_immune" | "weaken_immune"
        | "zombie_killer" | "witch_killer" | "trait_witch" | "boss_wave_immune" | "attack_1_abilities"
        | "attack_2_abilities" | "attack_3_abilities" | "spawn_animation_flag" | "use_gudetama_soul"
        | "warp_immune" | "trait_eva" | "eva_killer" | "trait_relic" | "curse_immune" | "insanely_tough"
        | "insane_damage" | "toxic_immune" | "surge_immune" | "mini_wave_flag" | "trait_aku"
        | "colossus_slayer" | "soulstrike" | "long_distance_2_flag" | "long_distance_3_flag"
        | "behemoth_slayer" | "mini_surge_flag" | "counter_surge" | "sage_slayer" | "explosion_immune" => Some(Rule::Flag),

        "knockback_chance" | "freeze_chance" | "slow_chance" | "critical_chance" | "wave_chance"
        | "weaken_chance" | "weaken_to" | "strengthen_threshold" | "survive" | "barrier_breaker_chance"
        | "warp_chance" | "savage_blow_chance" | "dodge_chance" | "surge_chance" | "curse_chance"
        | "shield_pierce_chance" | "behemoth_dodge_chance" | "metal_killer_percent" | "explosion_chance" => Some(Rule::Percent),

        "hitpoints" | "knockbacks" | "speed" | "attack_1" | "attack_cooldown"
        | "time_until_attack_1" | "freeze_duration" | "slow_duration" | "weaken_duration" | "attack_2"
        | "attack_3" | "time_until_attack_2" | "time_until_attack_3" | "warp_duration" | "dodge_duration"
        | "curse_duration" | "behemoth_dodge_duration" => Some(Rule::Floor(0)),

        "attack_count_total" | "time_before_death" | "spawn_animation_type" | "soul_animation_type" => Some(Rule::Floor(-1)),

        "standing_range" | "eoc1_cost" | "hitbox_position" | "hitbox_width" | "minimum_z_layer"
        | "maximum_z_layer" | "wave_level" | "strengthen_boost" | "long_distance_1_anchor"
        | "long_distance_1_span" | "warp_distance_minimum" | "warp_distance_maximum" | "savage_blow_boost"
        | "surge_spawn_anchor" | "surge_spawn_span" | "surge_level" | "long_distance_2_anchor"
        | "long_distance_2_span" | "long_distance_3_anchor" | "long_distance_3_span" | "conjure_unit_id"
        | "explosion_spawn_anchor" | "explosion_spawn_span" => Some(Rule::Plain),

        _ => None,
    }
}

pub(in crate::editor::figures) fn note(field: &str) -> Option<&'static str> {
    match field {
        "legacy_weak_against" => Some(
            "Legacy, does nothing now\nBack in 1.0.0 this cat dealt half damage to enemies whose \"Legacy Strong Against\" was also set",
        ),
        "cooldown" => Some("The game floors this at 60 frames, so anything lower is the same as 60"),
        "spawn_animation_type" => {
            Some("An animation ID, same idea as the soul one\n-1 means none, 0 means use ID 0")
        }
        "spawn_animation_flag" => Some(
            "Likely means \"use the entry maanim attached to my ID and form\"\nThose files exist, so that is the best read for now",
        ),
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
