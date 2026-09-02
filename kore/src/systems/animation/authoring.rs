mod maanim;
mod mamodel;

use nyanko::graphics::rig::{AnimModification, Keyframe, Model};

pub use maanim::Maanim;
pub use mamodel::{bound, defaults, nameable, Mamodel, FIELDS, NAME_FIELD};

pub const KINDS: [i32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

pub fn neutral_value(kind: i32, part: usize, model: Option<&Model>) -> i32 {
    let Some(model) = model else {
        return 0;
    };

    match kind {
        0 => model.parts.get(part).map_or(0, |declared| declared.parent),
        8..=10 => model.scale_unit,
        12 => model.opacity_unit,
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
    match kind {
        0 => "Parent",
        1 => "Unit ID",
        2 => "Sprite",
        3 => "Z Order",
        4 => "X",
        5 => "Y",
        6 => "Pivot X",
        7 => "Pivot Y",
        8 => "Scale",
        9 => "Scale X",
        10 => "Scale Y",
        11 => "Angle",
        12 => "Opacity",
        13 => "Flip X",
        14 => "Flip Y",
        _ => "Unknown",
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

pub fn loop_label(count: i32) -> String {
    match count {
        -1 => "Forever".to_string(),
        held if held <= 1 => "Once".to_string(),
        _ => "Count".to_string(),
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
            parts: vec![ModelPart { parent: 4, ..ModelPart::default() }],
            scale_unit: 1000,
            opacity_unit: 1000,
            ..Model::default()
        };

        assert_eq!(neutral_value(0, 0, Some(&model)), 4);
        assert_eq!(neutral_value(9, 0, Some(&model)), 1000);
        assert_eq!(neutral_value(12, 0, Some(&model)), 1000);
        assert_eq!(neutral_value(5, 0, Some(&model)), 0);
        assert_eq!(neutral_value(11, 0, Some(&model)), 0);
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
