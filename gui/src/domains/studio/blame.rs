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

        found.extend(crash::sheet_faults(model, unit.or_else(|| majority(model)).unwrap_or(NO_SHEET)));

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

fn majority(model: &Model) -> Option<i32> {
    let mut tally: Vec<(i32, usize)> = Vec::new();

    for part in model.parts.iter().filter(|part| part.id != NOT_DRAWN) {
        match tally.iter_mut().find(|(id, _)| *id == part.id) {
            Some((_, held)) => *held += 1,
            None => tally.push((part.id, 1)),
        }
    }

    let most = tally.iter().map(|(_, held)| *held).max()?;

    match tally.iter().filter(|(_, held)| *held == most).count() {
        1 => tally.iter().find(|(_, held)| *held == most).map(|(id, _)| *id),
        _ => None,
    }
}

fn detail(fault: &Fault) -> String {
    match fault {
        Fault::ScaleUnit => SCALE_UNIT_DETAIL.to_owned(),
        Fault::OpacityUnit => OPACITY_UNIT_DETAIL.to_owned(),
        Fault::PolynomialTie { first, second, frame } => {
            format!("Keyframes {} and {} both sit on frame {} inside a curved run\nThe game divides by the gap between them", first + 1, second + 1, frame)
        }
        Fault::ForeignSheet { id } => {
            format!("Draws from sheet {} which it doesnt load\nThe game reads a null pointer", id)
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
    fn one_odd_sheet_id_is_caught_with_no_unit_to_compare_against() {
        // The set is not installed anywhere, so nothing declares a unit. The rig still
        // has to agree with itself: one part stamped 608 among 609s is the crash.
        let model = chain(&[609, 609, 608]);
        let blame = Blame::of(&model, None, None);

        assert_eq!(blame.part(2), Some(Alarm::Faulted));
        assert_eq!(blame.part(1), Some(Alarm::Tainted), "and its parents carry the mark");
        assert_eq!(blame.part(0), Some(Alarm::Tainted));
    }

    #[test]
    fn an_even_split_blames_every_drawn_part_because_no_id_is_the_majority() {
        let model = chain(&[609, 608]);
        let blame = Blame::of(&model, None, None);

        assert_eq!(blame.part(0), Some(Alarm::Faulted));
        assert_eq!(blame.part(1), Some(Alarm::Faulted));
    }

    #[test]
    fn a_declared_unit_beats_the_majority_so_an_all_wrong_rig_still_trips() {
        // Every part agrees with itself and is still wrong for the slot it sits in.
        let model = chain(&[34, 34, 34]);

        assert!(Blame::of(&model, None, None).quiet(), "self-consistent, nothing to say");
        assert_eq!(Blame::of(&model, None, Some(44)).part(0), Some(Alarm::Faulted));
    }

    #[test]
    fn parts_that_draw_nothing_never_count_toward_the_majority() {
        let model = chain(&[-1, 609, 608]);
        let blame = Blame::of(&model, None, None);

        assert_eq!(blame.part(0), Some(Alarm::Tainted), "undrawn, but an ancestor");
        assert_eq!(blame.part(2), Some(Alarm::Faulted));
    }

    #[test]
    fn a_whole_file_fault_lands_on_the_root_so_it_shows_from_the_top() {
        let model = Model { scale_unit: 0, ..chain(&[609, 609]) };
        let blame = Blame::of(&model, None, Some(609));

        assert_eq!(blame.part(0), Some(Alarm::Faulted));
        assert_eq!(blame.part(1), None);
    }
}
