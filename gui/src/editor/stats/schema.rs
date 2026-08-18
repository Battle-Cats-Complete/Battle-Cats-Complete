use std::borrow::Cow;

const DOUBLED: &[usize] = &[4, 7];

const QUARTERED: &[usize] = &[73, 74, 87, 88, 114, 115];

const OPTIONAL: &[usize] = &[55, 56, 57, 66, 110];

const LABELS: &[&str] = &[
    "Hitpoints", "Knockbacks", "Speed", "Attack 1 Damage", "Attack Cooldown", "Standing Range",
    "EoC1 Cost", "Cooldown", "Hitbox Position", "Hitbox Width", "Target Red", "Unused",
    "Area Attack", "Time Until Attack 1", "Minimum Z Layer", "Maximum Z Layer", "Target Floating",
    "Target Dark", "Target Metal", "Target Traitless", "Target Angel", "Target Alien",
    "Target Zombie", "Strong Against", "Knockback Chance", "Freeze Chance", "Freeze Duration",
    "Slow Chance", "Slow Duration", "Resist", "Massive Damage", "Critical Chance", "Attack Only",
    "Double Bounty", "Base Destroyer", "Wave Chance", "Wave Level", "Weaken Chance",
    "Weaken Duration", "Weaken To", "Strengthen Threshold", "Strengthen Boost", "Survive", "Metal",
    "Long Distance 1 Anchor", "Long Distance 1 Span", "Wave Immune", "Wave Block",
    "Knockback Immune", "Freeze Immune", "Slow Immune", "Weaken Immune", "Zombie Killer",
    "Witch Killer", "Target Witch", "Attack Count Total", "Boss Wave Immune", "Time Before Death",
    "Attack Count State", "Attack 2 Damage", "Attack 3 Damage", "Time Until Attack 2", "Time Until Attack 3",
    "Attack 1 Abilities", "Attack 2 Abilities", "Attack 3 Abilities", "Spawn Animation Type",
    "Soul Animation Type", "Spawn Animation Flag", "Soul Animation Flag", "Barrier Breaker Chance",
    "Warp Chance", "Warp Duration", "Warp Distance Minimum", "Warp Distance Maximum",
    "Warp Immune", "Target Eva", "Eva Killer", "Target Relic", "Curse Immune", "Insanely Tough",
    "Insane Damage", "Savage Blow Chance", "Savage Blow Boost", "Dodge Chance", "Dodge Duration",
    "Surge Chance", "Surge Spawn Anchor", "Surge Spawn Span", "Surge Level", "Toxic Immune",
    "Surge Immune", "Curse Chance", "Curse Duration", "Mini Wave Flag", "Shield Pierce Chance",
    "Target Aku", "Colossus Slayer", "Soulstrike", "Long Distance 2 Flag", "Long Distance 2 Anchor",
    "Long Distance 2 Span", "Long Distance 3 Flag", "Long Distance 3 Anchor",
    "Long Distance 3 Span", "Behemoth Slayer", "Behemoth Dodge Chance", "Behemoth Dodge Duration",
    "Mini Surge Flag", "Counter Surge", "Conjure Unit Id", "Sage Slayer", "Metal Killer Percent",
    "Explosion Chance", "Explosion Spawn Anchor", "Explosion Spawn Span", "Explosion Immune",
];

pub(super) fn known() -> usize {
    LABELS.len()
}

pub(super) fn label(index: usize) -> Cow<'static, str> {
    LABELS
        .get(index)
        .map_or_else(|| Cow::Owned(format!("Column {}", index + 1)), |label| Cow::Borrowed(*label))
}

pub(super) fn to_display(index: usize, raw: i32) -> i32 {
    if DOUBLED.contains(&index) {
        return raw * 2;
    }

    if QUARTERED.contains(&index) {
        return raw / 4;
    }

    raw
}

pub(super) fn to_raw(index: usize, display: i32) -> i32 {
    if DOUBLED.contains(&index) {
        return display / 2;
    }

    if QUARTERED.contains(&index) {
        return display * 4;
    }

    display
}

pub(super) fn fallback(index: usize) -> i32 {
    if OPTIONAL.contains(&index) { -1 } else { 0 }
}
