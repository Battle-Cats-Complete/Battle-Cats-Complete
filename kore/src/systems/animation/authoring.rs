mod cadence;
mod imgcut;
mod maanim;
mod mamodel;

use nyanko::graphics::rig::{AnimModification, Keyframe, Model};
use nyanko::graphics::tools::property;

pub use cadence::{Beat, Cadence, Cycle, REACH};
pub use imgcut::{Imgcut, CUT_FIELDS, CUT_NAME_FIELD};
pub use maanim::Maanim;
pub use mamodel::{bound, defaults, nameable, Mamodel, FIELDS, NAME_FIELD};

pub fn kinds() -> Vec<i32> {
    property::PROPERTIES.iter().map(|entry| entry.kind).collect()
}

pub fn offsets(kind: i32) -> bool {
    property::property(kind).is_some_and(|entry| entry.blend == property::Blend::Offset)
}

pub fn neutral_value(kind: i32, part: usize, model: Option<&Model>) -> i32 {
    let Some(model) = model else {
        return 0;
    };

    let Some(entry) = property::property(kind) else {
        return 0;
    };

    if entry.blend == property::Blend::Offset {
        let Some(declared) = model.parts.get(part) else {
            return 0;
        };

        return match entry.field {
            "parent" => declared.parent,
            "id" => declared.id,
            "sprite" => declared.sprite,
            _ => declared.z,
        };
    }

    match entry.field {
        "scale" | "scale_x" | "scale_y" => model.scale_unit,
        "opacity" => model.opacity_unit,
        _ => 0,
    }
}

pub fn blank_curve(part: usize, kind: i32, model: Option<&Model>) -> AnimModification {
    let value = neutral_value(kind, part, model);

    AnimModification {
        part: i32::try_from(part).unwrap_or(0),
        kind,
        loop_count: 1,
        min_value: 0,
        max_value: 0,
        name: String::new(),
        keyframes: vec![Keyframe { frame: 0, value, ease: 0, ease_power: 0 }],
    }
}

pub fn kind_label(kind: i32) -> &'static str {
    let Some(entry) = property::property(kind) else {
        return "Unknown";
    };

    match entry.field {
        "parent" => "Parent",
        "id" => "Sheet ID",
        "sprite" => "Sprite",
        "depth" => "Z Order",
        "x" => "X",
        "y" => "Y",
        "pivot_x" => "Pivot X",
        "pivot_y" => "Pivot Y",
        "scale" => "Scale",
        "scale_x" => "Scale X",
        "scale_y" => "Scale Y",
        "angle" => "Angle",
        "opacity" => "Opacity",
        "flip_x" => "Flip X",
        "flip_y" => "Flip Y",
        other => other,
    }
}

pub const EASES: [&str; 4] = ["Linear", "Hold", "Exponential", "Polynomial"];

const EASE_EXPONENTIAL: i32 = 2;

pub fn ease_label(ease: i32) -> &'static str {
    usize::try_from(ease).ok().and_then(|at| EASES.get(at)).copied().unwrap_or("Unknown")
}

pub fn ease_takes_power(ease: i32) -> bool {
    ease == EASE_EXPONENTIAL
}

pub fn ease_value(label: &str) -> Option<i32> {
    EASES.iter().position(|known| *known == label).and_then(|at| i32::try_from(at).ok())
}

pub fn key_label(keys: usize) -> String {
    match keys {
        1 => "1 key".to_string(),
        other => format!("{} keys", other),
    }
}

pub fn loop_label(count: i32) -> &'static str {
    match count {
        -1 => "Forever",
        held if held <= 1 => "Once",
        _ => "Count",
    }
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    #[test]
    fn every_replay_count_reads_as_a_word_or_a_multiplier() {
        // The engine wraps only on -1 and on counts above one; everything else
        // rests on the final keyframe, so it plays through exactly once.
        assert_eq!(loop_label(-1), "Forever");
        assert_eq!(loop_label(1), "Once");
        assert_eq!(loop_label(0), "Once");
        assert_eq!(loop_label(-2), "Once");
        assert_eq!(loop_label(4), "Count");
    }

    #[test]
    fn every_label_comes_from_the_engines_own_table() {
        // The map used to be mirrored by hand here; nyanko publishes it now, so a
        // reorder upstream can no longer silently relabel a channel.
        assert_eq!(kinds().len(), 15);
        assert_eq!(kind_label(4), "X");
        assert_eq!(kind_label(11), "Angle");
        assert_eq!(kind_label(99), "Unknown");
    }

    #[test]
    fn only_the_leading_kinds_are_stored_as_a_difference_from_rest() {
        // Kinds 0-3 store `value - rest`; everything else is the pose outright, and
        // seeding a new channel has to know which.
        assert!(offsets(0) && offsets(3));
        assert!(!offsets(4) && !offsets(12));
        assert!(!offsets(99));
    }

    #[test]
    fn a_new_curve_names_its_part_and_plays_once() {
        let track = blank_curve(7, 4, None);

        assert_eq!((track.part, track.kind, track.loop_count), (7, 4, 1));
        assert_eq!(track.keyframes, vec![Keyframe::default()]);
    }

    #[test]
    fn a_new_curve_starts_where_the_engine_treats_it_as_absent() {
        // The pose clears to the model's units for scale and opacity, and parent
        // is stored as a difference from the part's own, so zero is not neutral there.
        let model = Model {
            parts: vec![ModelPart { parent: 4, z: 6, ..ModelPart::default() }],
            scale_unit: 1000,
            opacity_unit: 1000,
            ..Model::default()
        };

        assert_eq!(neutral_value(0, 0, Some(&model)), 4);
        assert_eq!(neutral_value(9, 0, Some(&model)), 1000);
        assert_eq!(neutral_value(12, 0, Some(&model)), 1000);
        assert_eq!(neutral_value(5, 0, Some(&model)), 0);
        assert_eq!(neutral_value(11, 0, Some(&model)), 0);
        assert_eq!(neutral_value(3, 0, Some(&model)), 6);
    }

    #[test]
    fn a_single_keyframe_is_not_plural() {
        assert_eq!(key_label(0), "0 keys");
        assert_eq!(key_label(1), "1 key");
        assert_eq!(key_label(9), "9 keys");
    }

    #[test]
    fn only_the_exponential_ease_reads_a_power() {
        // Linear, Hold and Polynomial never touch ease_power in the engine.
        assert!(ease_takes_power(2));
        assert!(!ease_takes_power(0));
        assert!(!ease_takes_power(1));
        assert!(!ease_takes_power(3));
        assert_eq!(EASES[2], "Exponential");
    }
}

