use std::collections::{HashMap, HashSet};

use nyanko::graphics::rig::{Animation, Model};
use nyanko::graphics::tools::crash::{self, Fault};

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Alarm {
    Tainted,
    Faulted,
}

#[derive(Default)]
pub(super) struct Blame {
    parts: HashMap<usize, Fault>,
    tracks: HashMap<usize, Fault>,
    tainted: HashSet<usize>,
    loose: bool,
}

impl Blame {
    pub(super) fn of(model: &Model, anim: Option<&Animation>, unit: Option<i32>) -> Self {
        let mut found = crash::model_faults(model);

        if let Some(unit) = unit {
            found.extend(crash::sheet_faults(model, unit));
        }

        if let Some(anim) = anim {
            found.extend(crash::anim_faults(anim, model));
        }

        let mut blame = Self::default();

        for sited in found {
            match (sited.track, sited.part) {
                (Some(track), part) => {
                    blame.tracks.insert(track, sited.fault);

                    match part {
                        Some(part) => {
                            blame.tainted.insert(part);
                            blame.ascend(model, part);
                        }
                        None => blame.loose = true,
                    }
                }
                (None, Some(part)) => {
                    blame.parts.insert(part, sited.fault);
                    blame.ascend(model, part);
                }
                (None, None) => {
                    for root in tree::roots(model) {
                        blame.parts.insert(root, sited.fault);
                    }
                }
            }
        }

        blame
    }

    fn ascend(&mut self, model: &Model, from: usize) {
        let mut at = from;

        for _ in 0..model.parts.len() {
            let parent = model
                .parts
                .get(at)
                .and_then(|part| usize::try_from(part.parent).ok())
                .filter(|parent| *parent != at && *parent < model.parts.len());

            let Some(parent) = parent else {
                return;
            };

            self.tainted.insert(parent);
            at = parent;
        }
    }

    pub(super) fn part(&self, at: usize) -> Option<Alarm> {
        if self.parts.contains_key(&at) {
            return Some(Alarm::Faulted);
        }

        self.tainted.contains(&at).then_some(Alarm::Tainted)
    }

    pub(super) fn track(&self, at: usize) -> Option<Alarm> {
        self.tracks.contains_key(&at).then_some(Alarm::Faulted)
    }

    pub(super) fn bucket(&self) -> Option<Alarm> {
        self.loose.then_some(Alarm::Tainted)
    }

    pub(super) fn quiet(&self) -> bool {
        self.parts.is_empty() && self.tracks.is_empty()
    }

    pub(super) fn notice(&self, part: Option<usize>, track: Option<usize>) -> Option<String> {
        if let Some(fault) = track.and_then(|at| self.tracks.get(&at)) {
            return Some(format!("{}\n{}", CHANNEL_FAULT, detail(fault)));
        }

        if let Some(at) = part {
            if let Some(fault) = self.parts.get(&at) {
                return Some(format!("{}\n{}", PART_FAULT, detail(fault)));
            }

            if self.tainted.contains(&at) {
                return Some(format!("{}\n{}", TAINTED_FAULT, TAINTED_HINT));
            }
        }

        None
    }
}

fn detail(fault: &Fault) -> String {
    match fault {
        Fault::ScaleUnit => SCALE_UNIT_DETAIL.to_owned(),
        Fault::OpacityUnit => OPACITY_UNIT_DETAIL.to_owned(),
        Fault::PolynomialTie { first, second, frame } => {
            format!("Keyframes {} and {} both sit on frame {} inside a curved run, and the game divides by the gap between them", first + 1, second + 1, frame)
        }
        Fault::ForeignSheet { id } => {
            format!("It draws from sheet {}, which this unit never loads, so the game reads a null pointer", id)
        }
        _ => UNKNOWN_DETAIL.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    fn chain(ids: &[i32]) -> Model {
        let parts = ids
            .iter()
            .enumerate()
            .map(|(at, id)| ModelPart {
                parent: at.checked_sub(1).and_then(|up| i32::try_from(up).ok()).unwrap_or(-1),
                id: *id,
                ..ModelPart::default()
            })
            .collect();

        Model { parts, scale_unit: 1000, opacity_unit: 1000, ..Model::default() }
    }

    #[test]
    fn a_faulted_part_taints_every_ancestor_and_nothing_else() {
        // 609 is the unit; part 2 draws from a sheet it never loads.
        let model = chain(&[609, 609, 44]);
        let blame = Blame::of(&model, None, Some(609));

        assert_eq!(blame.part(2), Some(Alarm::Faulted));
        assert_eq!(blame.part(1), Some(Alarm::Tainted));
        assert_eq!(blame.part(0), Some(Alarm::Tainted));

        assert!(blame.notice(Some(2), None).is_some_and(|held| held.starts_with(PART_FAULT)));
        assert!(blame.notice(Some(0), None).is_some_and(|held| held.starts_with(TAINTED_FAULT)));
    }

    #[test]
    fn a_clean_rig_blames_nobody() {
        let model = chain(&[609, 609, -1]);
        let blame = Blame::of(&model, None, Some(609));

        assert!(blame.quiet());
        assert_eq!(blame.part(2), None, "a part that draws nothing names no sheet");
        assert_eq!(blame.notice(Some(0), None), None);
    }

    #[test]
    fn a_whole_file_fault_lands_on_the_root_so_it_shows_from_the_top() {
        let model = Model { scale_unit: 0, ..chain(&[609, 609]) };
        let blame = Blame::of(&model, None, Some(609));

        assert_eq!(blame.part(0), Some(Alarm::Faulted));
        assert_eq!(blame.part(1), None);
    }
}
