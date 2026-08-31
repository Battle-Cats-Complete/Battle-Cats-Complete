pub mod changes;
pub mod forms;
pub mod talents;
pub mod units;

use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::common::dirs;
use crate::common::io::json;
use crate::domains::cat::files as cat_files;

const FILE: &str = "ore.json";

const BASE: &str = "base.json";

const SCHEMA: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keying {
    Column,
    Line,
}

const MINEABLE: &[(&str, Keying)] = &[
    (cat_files::SKILL_ACQUISITION, Keying::Column),
    (cat_files::UNIT_BUY, Keying::Line),
];

pub(crate) fn mineable(filename: &str) -> bool {
    keying(filename).is_some()
}

fn keying(filename: &str) -> Option<Keying> {
    if cat_files::stats_id(filename).is_some() {
        return Some(Keying::Line);
    }

    MINEABLE.iter().find(|(name, _)| *name == filename).map(|(_, keying)| *keying)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Build {
    pub code: u32,
    pub name: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Base {
    #[serde(default)]
    builds: Vec<Build>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Baseline,
    Changed,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RowDelta {
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileDelta {
    pub file: String,
    pub region: String,
    pub status: Status,
    pub rows_before: usize,
    pub rows_after: usize,
    pub rows: Vec<RowDelta>,
}

impl FileDelta {
    pub fn added(&self) -> usize {
        self.rows.iter().filter(|row| row.before.is_none()).count()
    }

    pub fn removed(&self) -> usize {
        self.rows.iter().filter(|row| row.after.is_none()).count()
    }

    pub fn modified(&self) -> usize {
        self.rows.iter().filter(|row| row.before.is_some() && row.after.is_some()).count()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Ore {
    pub schema: u32,
    pub stamp: u64,
    #[serde(default)]
    pub before: Vec<Build>,
    #[serde(default)]
    pub after: Vec<Build>,
    pub files: Vec<FileDelta>,
}

impl Ore {
    pub fn age(&self) -> Option<Duration> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|now| now.checked_sub(Duration::from_secs(self.stamp)))
    }

    pub fn file(&self, name: &str) -> Option<&FileDelta> {
        self.files.iter().find(|delta| delta.file == name)
    }
}

pub fn load() -> Option<Ore> {
    json::load_state::<Ore>(FILE).filter(|ore| ore.schema == SCHEMA)
}

pub fn clear() {
    for name in [FILE, BASE] {
        drop_state(name);
    }
}

fn drop_state(name: &str) {
    let Some(directory) = dirs::state() else {
        return;
    };

    let path = directory.join(name);

    if path.exists()
        && let Err(err) = fs::remove_file(&path)
    {
        warn!("Failed to clear {}: {}", name, err);
    }
}

fn base() -> Vec<Build> {
    json::load_state::<Base>(BASE).map_or_else(Vec::new, |base| base.builds)
}

fn set_base(builds: &[Build]) {
    if let Err(err) = json::save_state(BASE, &Base { builds: builds.to_vec() }) {
        warn!("Failed to save {}: {}", BASE, err);
    }
}

pub(crate) fn commit(files: Vec<FileDelta>, after: Vec<Build>) -> bool {
    let before = base();

    if !after.is_empty() && before != after {
        set_base(&after);
    }

    if files.is_empty() {
        return false;
    }

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let ore = Ore { schema: SCHEMA, stamp, before, after, files };

    if let Err(err) = json::save_state(FILE, &ore) {
        warn!("Failed to save {}: {}", FILE, err);
        return false;
    }

    true
}

pub(crate) fn delta(file: &str, region: &str, before: Option<&[u8]>, after: &[u8]) -> Option<FileDelta> {
    let keying = keying(file)?;
    let after_text = String::from_utf8_lossy(after).into_owned();
    let after_rows = rows(&after_text, keying);

    let Some(before) = before else {
        return Some(FileDelta {
            file: file.to_string(),
            region: region.to_string(),
            status: Status::Baseline,
            rows_before: 0,
            rows_after: after_rows.len(),
            rows: Vec::new(),
        });
    };

    let before_text = String::from_utf8_lossy(before).into_owned();
    let mut carried: HashMap<String, &str> = rows(&before_text, keying).into_iter().collect();
    let rows_before = carried.len();

    let mut changes = Vec::new();

    for (key, line) in &after_rows {
        match carried.remove(key) {
            Some(previous) if previous == *line => {}
            Some(previous) => changes.push(RowDelta {
                key: key.clone(),
                before: Some(previous.to_string()),
                after: Some((*line).to_string()),
            }),
            None => changes.push(RowDelta {
                key: key.clone(),
                before: None,
                after: Some((*line).to_string()),
            }),
        }
    }

    let mut dropped: Vec<RowDelta> = carried
        .into_iter()
        .map(|(key, line)| RowDelta { key, before: Some(line.to_string()), after: None })
        .collect();

    dropped.sort_by(|left, right| left.key.cmp(&right.key));
    changes.extend(dropped);

    (!changes.is_empty()).then(|| FileDelta {
        file: file.to_string(),
        region: region.to_string(),
        status: Status::Changed,
        rows_before,
        rows_after: after_rows.len(),
        rows: changes,
    })
}

fn rows(text: &str, keying: Keying) -> Vec<(String, &str)> {
    match keying {
        Keying::Column => text
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim_end();
                let end = trimmed.find([',', '\t'])?;
                let key = trimmed[..end].trim();

                (!key.is_empty()).then(|| (key.to_string(), trimmed))
            })
            .collect(),
        Keying::Line => text
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let trimmed = line.trim_end();

                (!trimmed.trim().is_empty()).then(|| (index.to_string(), trimmed))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &[u8] = b"1,0,10,5,100,200,0,0,0,0,0,0,1,1,0\n2,0,11,5,50,60,0,0,0,0,0,0,2,2,0\n";

    // unitbuy.csv is positional: its first column is rarity, so column keying would
    // collide thousands of rows onto a handful of keys.
    // Every per-unit stats table is mineable, and each of its lines is one form.
    #[test]
    fn a_per_unit_stats_table_is_mined_by_line() {
        assert!(mineable("unit440.csv"));
        assert!(!mineable("unitlevel.csv"));
        assert!(!mineable("unit44.csv"));
    }

    #[test]
    fn a_positional_table_keys_by_line_rather_than_by_its_first_column() {
        let old = b"0,0,50\n0,1,60\n";
        let new = b"0,0,50\n0,1,60\n3,2,70\n";

        let delta = delta("unitbuy.csv", "en", Some(old), new).expect("changed");

        assert_eq!(delta.rows_before, 2);
        assert_eq!(delta.added(), 1);
        assert_eq!(delta.rows[0].key, "2", "the third line is unit two");
    }

    #[test]
    fn a_first_sighting_of_a_file_is_a_baseline_rather_than_a_wall_of_additions() {
        let delta = delta("SkillAcquisition.csv", "en", None, OLD).expect("baseline");

        assert_eq!(delta.status, Status::Baseline);
        assert_eq!(delta.rows_after, 2);
        assert!(delta.rows.is_empty(), "a baseline must not report every row as new");
    }

    #[test]
    fn an_inserted_row_does_not_smear_the_rows_beneath_it() {
        // A mid-file insertion is the case a line diff misreads as a mass rewrite.
        let new = b"1,0,10,5,100,200,0,0,0,0,0,0,1,1,0\n7,0,99,5,1,2,0,0,0,0,0,0,3,3,0\n2,0,11,5,50,60,0,0,0,0,0,0,2,2,0\n";
        let delta = delta("SkillAcquisition.csv", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.added(), 1);
        assert_eq!(delta.modified(), 0);
        assert_eq!(delta.removed(), 0);
        assert_eq!(delta.rows[0].key, "7");
    }

    #[test]
    fn an_edited_row_is_reported_with_both_sides() {
        let new = b"1,0,10,5,100,300,0,0,0,0,0,0,1,1,0\n2,0,11,5,50,60,0,0,0,0,0,0,2,2,0\n";
        let delta = delta("SkillAcquisition.csv", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.modified(), 1);
        assert_eq!(delta.rows[0].before.as_deref(), Some("1,0,10,5,100,200,0,0,0,0,0,0,1,1,0"));
    }

    #[test]
    fn an_identical_reimport_yields_nothing_to_mine() {
        assert!(delta("SkillAcquisition.csv", "en", Some(OLD), OLD).is_none());
    }

    #[test]
    fn a_vanished_row_is_kept_so_a_downgrade_can_be_called_out() {
        let new = b"1,0,10,5,100,200,0,0,0,0,0,0,1,1,0\n";
        let delta = delta("SkillAcquisition.csv", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.removed(), 1);
        assert_eq!(delta.rows_after, 1);
    }
}
