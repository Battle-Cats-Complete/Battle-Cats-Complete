use crate::editor::figures::resolved::{Choice, Gate, Rule};

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
        "trait_starred_alien" => Some(Rule::Choice(&const {
            [
                Choice::new(0, "None"),
                Choice::new(1, "Starred Alien"),
                Choice::new(2, "CotC 1 Cat God"),
                Choice::new(3, "CotC 2 Cat God"),
                Choice::new(4, "CotC 3 Cat God"),
            ]
        })),

        "legacy_strong_against" | "trait_red" | "trait_floating" | "trait_dark" | "trait_metal"
        | "trait_traitless" | "trait_angel" | "trait_alien" | "trait_zombie" | "base_destroyer"
        | "wave_immune" | "wave_block" | "knockback_immune" | "freeze_immune" | "slow_immune"
        | "weaken_immune" | "trait_witch" | "trait_dojo" | "attack_1_abilities" | "attack_2_abilities"
        | "attack_3_abilities" | "spawn_animation_flag" | "use_gudetama_soul" | "warp_immune" | "trait_eva"
        | "trait_relic" | "surge_immune" | "mini_wave_flag" | "trait_aku" | "trait_colossus"
        | "long_distance_2_flag" | "long_distance_3_flag" | "trait_behemoth" | "mini_surge_flag"
        | "counter_surge" | "trait_sage" | "curse_immune" | "explosion_immune" | "trait_kaijin" => Some(Rule::Flag),

        "knockback_chance" | "freeze_chance" | "slow_chance" | "critical_chance" | "wave_chance"
        | "weaken_chance" | "weaken_to" | "strengthen_threshold" | "survive" | "warp_chance"
        | "curse_chance" | "savage_blow_chance" | "dodge_chance" | "toxic_chance" | "surge_chance"
        | "death_surge_chance" | "explosion_chance" | "drain_chance" => Some(Rule::Percent),

        "hitpoints" | "knockbacks" | "speed" | "attack_1" | "attack_cooldown" | "time_until_attack_1"
        | "freeze_duration" | "slow_duration" | "weaken_duration" | "attack_2" | "attack_3"
        | "time_until_attack_2" | "time_until_attack_3" | "warp_duration" | "curse_duration"
        | "dodge_duration" => Some(Rule::Floor(0)),

        "attack_count_total" | "time_before_death" | "spawn_animation_type" | "soul_animation_type" => Some(Rule::Floor(-1)),

        "standing_range" | "cash_drop" | "hitbox_position" | "hitbox_width" | "wave_level"
        | "strengthen_boost" | "long_distance_1_anchor" | "long_distance_1_span" | "burrow_amount"
        | "burrow_distance" | "revive_count" | "revive_time" | "revive_hp" | "barrier_hitpoints"
        | "warp_distance_minimum" | "warp_distance_maximum" | "savage_blow_boost" | "toxic_damage"
        | "surge_spawn_anchor" | "surge_spawn_span" | "surge_level" | "shield_hitpoints" | "shield_regen"
        | "death_surge_spawn_anchor" | "death_surge_spawn_span" | "death_surge_level"
        | "long_distance_2_anchor" | "long_distance_2_span" | "long_distance_3_anchor"
        | "long_distance_3_span" | "explosion_spawn_anchor" | "explosion_spawn_span" | "drain_percent" => Some(Rule::Plain),

        _ => None,
    }
}

pub(in crate::editor::figures) fn note(field: &str) -> Option<&'static str> {
    match field {
        "legacy_strong_against" => Some(
            "Legacy, does nothing now\nBack in 1.0.0 cats with \"Legacy Weak Against\" set dealt half damage to this enemy",
        ),
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
