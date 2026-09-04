use nyanko::graphics::rig::{Animation, Model};

pub const SHEET_FIELD: usize = 1;
const NOT_DRAWN: i32 = -1;

const EASE_POLYNOMIAL: i32 = 3;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Hazard {
    ScaleUnit,
    OpacityUnit,
    PolynomialTie { track: usize, frame: i32 },
    ForeignSheet { id: i32, parts: usize },
}

impl Hazard {
    pub fn label(&self) -> &'static str {
        match self {
            Hazard::ScaleUnit => "Scale divisor is zero",
            Hazard::OpacityUnit => "Opacity divisor is zero",
            Hazard::PolynomialTie { .. } => "Two polynomial keys share a frame",
            Hazard::ForeignSheet { .. } => "Parts draw from another unit's sheet",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Hazard::ScaleUnit => {
                "The game divides every part's scale by this number twice while placing it. \
                 A zero faults on the first frame drawn."
                    .to_owned()
            }
            Hazard::OpacityUnit => {
                "The game divides every part's opacity by this number while placing it. \
                 A zero faults on the first frame drawn."
                    .to_owned()
            }
            Hazard::PolynomialTie { track, frame } => format!(
                "Channel {track} runs a polynomial curve through frame {frame} twice. \
                 The game divides by the gap between the two, and faults on zero."
            ),
            Hazard::ForeignSheet { id, parts } => format!(
                "{parts} parts name sheet {id}. The game looks that sheet up in the unit it is \
                 drawing, finds nothing, and dereferences null. Restamp them onto this unit."
            ),
        }
    }
}

pub fn model_hazards(model: &Model) -> Vec<Hazard> {
    let mut found = Vec::new();

    if model.scale_unit == 0 {
        found.push(Hazard::ScaleUnit);
    }

    if model.opacity_unit == 0 {
        found.push(Hazard::OpacityUnit);
    }

    found
}

pub fn sheet_hazards(model: &Model, unit: i32) -> Vec<Hazard> {
    let mut found: Vec<Hazard> = Vec::new();

    for part in &model.parts {
        if part.id == NOT_DRAWN || part.id == unit {
            continue;
        }

        match found.iter_mut().find(|held| matches!(held, Hazard::ForeignSheet { id, .. } if *id == part.id)) {
            Some(Hazard::ForeignSheet { parts, .. }) => *parts += 1,
            _ => found.push(Hazard::ForeignSheet { id: part.id, parts: 1 }),
        }
    }

    found
}

pub fn anim_hazards(anim: &Animation) -> Vec<Hazard> {
    let mut found = Vec::new();

    for (track, curve) in anim.modifications.iter().enumerate() {
        let keys = &curve.keyframes;

        for at in 0..keys.len() {
            if keys[at].ease != EASE_POLYNOMIAL {
                continue;
            }

            let (low, high) = run(keys, at);

            for outer in low..=high {
                for inner in (outer + 1)..=high {
                    if keys[outer].frame != keys[inner].frame {
                        continue;
                    }

                    let tie = Hazard::PolynomialTie { track, frame: keys[outer].frame };

                    if !found.contains(&tie) {
                        found.push(tie);
                    }
                }
            }
        }
    }

    found
}

fn run(keys: &[nyanko::graphics::rig::Keyframe], at: usize) -> (usize, usize) {
    let mut low = at;

    while low > 0 && keys[low - 1].ease == EASE_POLYNOMIAL {
        low -= 1;
    }

    let mut high = at + 1;

    while high + 1 < keys.len() && keys[high].ease == EASE_POLYNOMIAL {
        high += 1;
    }

    (low, high.min(keys.len().saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::{AnimModification, Keyframe, ModelPart};

    use super::*;

    fn curve(eases: &[(i32, i32)]) -> Animation {
        Animation {
            version: 1,
            modifications: vec![AnimModification {
                part: 0,
                kind: 4,
                loop_count: 1,
                keyframes: eases
                    .iter()
                    .map(|(frame, ease)| Keyframe { frame: *frame, value: 0, ease: *ease, ease_power: 0 })
                    .collect(),
                ..AnimModification::default()
            }],
        }
    }

    fn model(scale: i32, opacity: i32) -> Model {
        Model {
            parts: vec![ModelPart::default()],
            scale_unit: scale,
            opacity_unit: opacity,
            angle_unit: 3600,
            ..Model::default()
        }
    }

    fn rigged(ids: &[i32]) -> Model {
        Model {
            parts: ids.iter().map(|id| ModelPart { id: *id, ..ModelPart::default() }).collect(),
            scale_unit: 1000,
            opacity_unit: 1000,
            angle_unit: 3600,
            ..Model::default()
        }
    }

    #[test]
    fn a_rig_stamped_for_another_unit_is_caught() {
        // Renaming 034_s to 000_f leaves every drawn part asking for sheet 34, which the
        // unit-000 draw context does not have. The -1 control nulls draw nothing and are fine.
        let borrowed = rigged(&[-1, 34, -1, 34, 34]);

        assert_eq!(sheet_hazards(&borrowed, 0), vec![Hazard::ForeignSheet { id: 34, parts: 3 }]);
        assert_eq!(sheet_hazards(&borrowed, 34), vec![], "in its own unit it is correct");
        assert_eq!(sheet_hazards(&rigged(&[-1, 0, 0]), 0), vec![]);
    }

    #[test]
    fn a_zero_divisor_is_caught_on_both_columns() {
        assert_eq!(model_hazards(&model(1000, 1000)), vec![]);
        assert_eq!(model_hazards(&model(0, 1000)), vec![Hazard::ScaleUnit]);
        assert_eq!(model_hazards(&model(1000, 0)), vec![Hazard::OpacityUnit]);
    }

    #[test]
    fn two_polynomial_keys_on_one_frame_are_caught() {
        let held = curve(&[(0, 3), (10, 3), (10, 3), (20, 0)]);

        assert_eq!(anim_hazards(&held), vec![Hazard::PolynomialTie { track: 0, frame: 10 }]);
    }

    #[test]
    fn a_shared_frame_outside_a_polynomial_run_is_left_alone() {
        // 77 vanilla curves hold a duplicate frame and the game plays them fine, because
        // only the polynomial path divides by the gap between two keys.
        let linear = curve(&[(0, 0), (10, 0), (10, 0), (20, 0)]);

        assert_eq!(anim_hazards(&linear), vec![]);

        let eased = curve(&[(0, 1), (10, 2), (10, 2), (20, 0)]);

        assert_eq!(anim_hazards(&eased), vec![]);
    }

    #[test]
    fn a_polynomial_run_with_distinct_frames_is_clean() {
        let held = curve(&[(0, 3), (10, 3), (20, 3), (30, 0)]);

        assert_eq!(anim_hazards(&held), vec![]);
    }
}
