use nyanko::cat::unit::{Talent, TalentGroup};
use nyanko::combat::get_talent;

use crate::systems::combat::registry::{get_display_def, AbilityIcon};

use super::{FileDelta, RowDelta, Status};

#[derive(Clone)]
pub struct Gain {
    pub index: u8,
    pub group: TalentGroup,
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub ultra: bool,
}

#[derive(Clone)]
pub struct Retune {
    pub gain: Gain,
    pub before: TalentGroup,
}

pub struct Find {
    pub cat_id: u32,
    pub fresh: bool,
    pub gained: Vec<Gain>,
    pub retuned: Vec<Retune>,
}

impl Find {
    pub fn enabled_levels(&self) -> std::collections::HashMap<u8, u8> {
        self.gained.iter().map(|gain| (gain.index, gain.group.max_level.max(1))).collect()
    }

    pub fn ultra_only(&self) -> bool {
        !self.gained.is_empty() && self.gained.iter().all(|gain| gain.ultra)
    }

    pub fn has_ultra(&self) -> bool {
        self.gained.iter().chain(self.retuned.iter().map(|retune| &retune.gain)).any(|gain| gain.ultra)
    }
}

pub struct Report {
    pub status: Status,
    pub region: String,
    pub rows_before: usize,
    pub rows_after: usize,
    pub finds: Vec<Find>,
    pub dropped: Vec<u32>,
    pub unreadable: usize,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.finds.is_empty() && self.dropped.is_empty()
    }
}

pub fn read(delta: &FileDelta) -> Report {
    let mut report = Report {
        status: delta.status,
        region: delta.region.clone(),
        rows_before: delta.rows_before,
        rows_after: delta.rows_after,
        finds: Vec::new(),
        dropped: Vec::new(),
        unreadable: 0,
    };

    for row in &delta.rows {
        match interpret(row) {
            Row::Find(find) => report.finds.push(find),
            Row::Dropped(id) => report.dropped.push(id),
            Row::Quiet => {}
            Row::Unreadable => report.unreadable += 1,
        }
    }

    report.finds.sort_by_key(|find| find.cat_id);
    report.dropped.sort_unstable();

    report
}

enum Row {
    Find(Find),
    Dropped(u32),
    Quiet,
    Unreadable,
}

fn interpret(row: &RowDelta) -> Row {
    let Some(after) = row.after.as_deref() else {
        return row.key.trim().parse::<u32>().map_or(Row::Unreadable, Row::Dropped);
    };

    let Some(current) = parse(after) else {
        return Row::Unreadable;
    };

    let previous = row.before.as_deref().and_then(parse);

    compare(previous.as_ref(), &current).map_or(Row::Quiet, Row::Find)
}

fn parse(line: &str) -> Option<Talent> {
    Talent::parse(line, None).ok()?.into_values().next()
}

fn compare(previous: Option<&Talent>, current: &Talent) -> Option<Find> {
    let carried = previous.map_or(&[][..], |talent| talent.groups.as_slice());

    let mut gained = Vec::new();
    let mut retuned = Vec::new();

    for (index, group) in current.groups.iter().enumerate() {
        let slot = index as u8;

        let Some(before) = carried.iter().find(|held| held.ability_id == group.ability_id) else {
            gained.push(gain(slot, group));
            continue;
        };

        if before != group {
            retuned.push(Retune { gain: gain(slot, group), before: before.clone() });
        }
    }

    if gained.is_empty() && retuned.is_empty() {
        return None;
    }

    Some(Find { cat_id: current.id, fresh: carried.is_empty(), gained, retuned })
}

fn gain(index: u8, group: &TalentGroup) -> Gain {
    let ability = get_talent(group.ability_id);

    let (name, fallback, icon) = ability.map_or(("Unknown Talent", "?", AbilityIcon::None), |def| {
        let display = get_display_def(def.identity);

        (display.name, display.fallback, display.icon)
    });

    Gain { index, group: group.clone(), name, fallback, icon, ultra: group.limit == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One group of talent 1 (Weaken): chance climbs 10% -> 20% across five levels.
    const WEAKEN: &str = "42,0,1,5,10,20,30,60,70,70,0,0,1,1,1,0";

    // The same unit plus a second group carrying the ultra marker in its limit column.
    const ULTRA: &str = "42,0,1,5,10,20,30,60,70,70,0,0,1,1,1,0,8,5,50,80,0,0,0,0,0,0,2,2,2,1";

    fn changed(before: Option<&str>, after: &str) -> Report {
        read(&FileDelta {
            file: "SkillAcquisition.csv".to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 1,
            rows_after: 1,
            rows: vec![RowDelta {
                key: "42".to_string(),
                before: before.map(str::to_string),
                after: Some(after.to_string()),
            }],
        })
    }

    #[test]
    fn a_brand_new_row_reads_as_a_unit_gaining_its_first_talents() {
        let report = changed(None, WEAKEN);
        let find = report.finds.first().expect("one find");

        assert_eq!(find.cat_id, 42);
        assert!(find.fresh);
        assert_eq!(find.gained.len(), 1);
        assert!(!find.gained[0].ultra);
    }

    #[test]
    fn a_talented_unit_gaining_an_ultra_group_reports_only_that_group() {
        let report = changed(Some(WEAKEN), ULTRA);
        let find = report.finds.first().expect("one find");

        assert!(!find.fresh, "the unit already had talents, so this is not a first grant");
        assert_eq!(find.gained.len(), 1);
        assert!(find.gained[0].ultra);
        assert!(find.ultra_only());

        // The cats page keys talent levels by slot, so the gain must carry its position.
        assert_eq!(find.enabled_levels(), std::collections::HashMap::from([(1, 5)]));
    }

    // The group travels whole so the reader can hand it to the cat talent display logic.
    #[test]
    fn a_gain_carries_the_group_the_row_declared() {
        let report = changed(None, WEAKEN);
        let group = &report.finds[0].gained[0].group;

        assert_eq!(group.ability_id, 1);
        assert_eq!(group.max_level, 5);
        assert_eq!((group.min_1, group.max_1), (10, 20));
    }

    #[test]
    fn a_rebalanced_group_is_a_retune_and_not_a_gain() {
        let buffed = "42,0,1,5,10,40,30,60,70,70,0,0,1,1,1,0";
        let report = changed(Some(WEAKEN), buffed);
        let find = report.finds.first().expect("one find");

        assert!(find.gained.is_empty());
        assert_eq!(find.retuned.len(), 1);
        assert_eq!(find.retuned[0].before.max_1, 20);
        assert_eq!(find.retuned[0].gain.group.max_1, 40);
    }

    #[test]
    fn a_row_that_will_not_parse_is_counted_rather_than_dropped_silently() {
        let report = changed(None, "not,a,talent,row");

        assert_eq!(report.unreadable, 1);
        assert!(report.finds.is_empty());
    }
}
