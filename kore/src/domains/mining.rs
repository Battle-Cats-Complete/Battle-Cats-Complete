pub mod changes;
pub mod enemies;
pub mod forms;
pub mod levels;
pub mod localized;
pub mod stages;
pub mod talents;
pub mod units;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::common::architecture;
use crate::common::dirs;
use crate::common::io::json;
use crate::domains::cat::files as cat_files;
use crate::domains::enemy::files as enemy_files;
use crate::domains::import::engine::manifest;


const ORE: &str = "ore.json";

const BEDROCK: &str = "bedrock.json";

const SEAMS: &str = "bedrock";

const CENSUS: &str = "census.json";

const EMPTY_MAP: u64 = 2;

const SCHEMA: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keying {
    Column,
    Line,
}

const MINEABLE: &[(&str, Keying)] = &[
    (cat_files::SKILL_ACQUISITION, Keying::Column),
    (cat_files::UNIT_BUY, Keying::Line),
    (enemy_files::STATS, Keying::Line),
    (stages::MAP_OPTION, Keying::Column),
];

pub(crate) fn mineable(filename: &str) -> bool {
    keying(filename).is_some()
}

fn keying(filename: &str) -> Option<Keying> {
    if cat_files::stats_id(filename).is_some() || stages::mineable(filename) || localized::mineable(filename) {
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

type Census = HashMap<String, u64>;

type Tables = HashMap<String, String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seam {
    Cat,
    Enemy,
    Stage,
}

const SEAM_ORDER: [Seam; 3] = [Seam::Cat, Seam::Enemy, Seam::Stage];

impl Seam {
    fn file(self) -> &'static str {
        match self {
            Self::Cat => "cat.json",
            Self::Enemy => "enemy.json",
            Self::Stage => "stage.json",
        }
    }
}

fn seam(filename: &str) -> Option<Seam> {
    if cat_files::stats_id(filename).is_some()
        || filename == cat_files::UNIT_BUY
        || filename == cat_files::SKILL_ACQUISITION
    {
        return Some(Seam::Cat);
    }

    match filename {
        enemy_files::STATS => Some(Seam::Enemy),
        stages::MAP_OPTION => Some(Seam::Stage),
        _ => None,
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Bedrock {
    #[serde(default)]
    builds: Vec<Build>,
    #[serde(default)]
    roster: Vec<u32>,
    #[serde(default)]
    promoted: Vec<u32>,
    #[serde(default)]
    bestiary: Vec<u32>,
    #[serde(default)]
    surfaced: Vec<u32>,
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
    #[serde(default)]
    pub from: String,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct FileTouch {
    pub file: String,
    pub status: Status,
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
    #[serde(default)]
    pub touched: Vec<FileTouch>,
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
    json::load_state::<Ore>(ORE).filter(|ore| ore.schema == SCHEMA)
}

fn walk(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(path);
                continue;
            }

            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                found.push((name.to_string(), path));
            }
        }
    }

    found
}

fn census(found: &[(String, PathBuf)]) -> Census {
    found
        .par_iter()
        .filter_map(|(name, path)| manifest::hash_file(path).ok().map(|hash| (name.to_string(), hash)))
        .collect()
}

fn tables(found: &[(String, PathBuf)], wanted: Seam) -> Tables {
    found
        .iter()
        .filter(|(name, _)| seam(name) == Some(wanted))
        .filter_map(|(name, path)| fs::read_to_string(path).ok().map(|text| (name.to_string(), text)))
        .collect()
}

fn seam_file(name: &str) -> Option<String> {
    let directory = dirs::state()?.join(SEAMS);

    fs::create_dir_all(&directory).ok()?;

    Some(format!("{}/{}", SEAMS, name))
}

fn read_seam<T: DeserializeOwned + Default>(name: &str) -> T {
    seam_file(name).and_then(|held| json::load_state(&held)).unwrap_or_default()
}

fn write_seam<T: Serialize>(name: &str, data: &T) {
    let Some(held) = seam_file(name) else {
        return;
    };

    if let Err(err) = json::save_state(&held, data) {
        warn!("Failed to save {}: {}", held, err);
    }
}

fn stored_census() -> Census {
    read_seam(CENSUS)
}

fn stored_rows() -> Tables {
    let mut all = Tables::new();

    for held in SEAM_ORDER {
        all.extend(read_seam::<Tables>(held.file()));
    }

    all
}

fn rebase(found: &[(String, PathBuf)], mut taken: Census) {
    let strays: Census = found
        .par_iter()
        .filter(|(name, _)| !taken.contains_key(name))
        .filter_map(|(name, path)| manifest::hash_file(path).ok().map(|hash| (name.to_string(), hash)))
        .collect();

    taken.extend(strays);

    write_seam(CENSUS, &taken);

    for held in SEAM_ORDER {
        write_seam(held.file(), &tables(found, held));
    }
}

pub(crate) fn enroll(taken: Census) {
    rebase(&walk(Path::new(architecture::GAME)), taken);
}

pub fn arrivals() -> (Vec<u32>, Vec<u32>) {
    let bedrock = stored_bedrock();

    (bedrock.promoted, bedrock.surfaced)
}

pub fn discard() {
    drop_state(ORE);

    let mut bedrock = stored_bedrock();

    bedrock.promoted.clear();
    bedrock.surfaced.clear();

    save_bedrock(&bedrock);
}

pub fn stamp() -> Option<u64> {
    let data = fs::metadata(dirs::state()?.join(ORE)).ok()?;
    let mut hasher = FxHasher::default();

    data.len().hash(&mut hasher);
    data.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos().hash(&mut hasher);

    Some(hasher.finish())
}

pub fn has_bedrock() -> bool {
    dirs::state()
        .and_then(|directory| fs::metadata(directory.join(SEAMS).join(CENSUS)).ok())
        .is_some_and(|data| data.len() > EMPTY_MAP)
}

pub fn capturable() -> bool {
    architecture::game_present()
}

pub fn capture() -> usize {
    let found = walk(Path::new(architecture::GAME));
    let taken = census(&found);
    let seen = taken.len();

    rebase(&found, taken);

    seen
}

pub fn craft() -> bool {
    let held = stored_census();
    let rows = stored_rows();
    let found = walk(Path::new(architecture::GAME));
    let taken = census(&found);

    let mut touched: Vec<FileTouch> = taken
        .iter()
        .filter_map(|(name, hash)| touch(name, held.get(name).copied(), *hash))
        .collect();

    touched.sort_by(|left, right| left.file.cmp(&right.file));

    let mut files = Vec::new();

    for (name, path) in found.iter().filter(|(name, _)| seam(name).is_some()) {
        let Some(before) = rows.get(name) else {
            continue;
        };

        let Ok(after) = fs::read(path) else {
            continue;
        };

        files.extend(delta(name, "", "", Some(before.as_bytes()), &after));
    }

    let recorded = commit(files, touched, Vec::new());

    rebase(&found, taken);

    recorded
}

pub fn clear() {
    for name in [ORE, BEDROCK] {
        drop_state(name);
    }

    if let Some(directory) = dirs::state() {
        let _ = fs::remove_dir_all(directory.join(SEAMS));
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

fn stored_bedrock() -> Bedrock {
    json::load_state::<Bedrock>(BEDROCK).unwrap_or_default()
}

fn save_bedrock(bedrock: &Bedrock) {
    if let Err(err) = json::save_state(BEDROCK, bedrock) {
        warn!("Failed to save {}: {}", BEDROCK, err);
    }
}

fn builds() -> Vec<Build> {
    stored_bedrock().builds
}

fn set_builds(after: &[Build]) {
    let mut bedrock = stored_bedrock();
    bedrock.builds = after.to_vec();

    save_bedrock(&bedrock);
}

pub fn reconcile(listable: &[u32]) -> Vec<u32> {
    settle(listable, |bedrock| (&mut bedrock.roster, &mut bedrock.promoted))
}

pub fn reconcile_foes(listable: &[u32]) -> Vec<u32> {
    settle(listable, |bedrock| (&mut bedrock.bestiary, &mut bedrock.surfaced))
}

fn settle(listable: &[u32], pick: impl Fn(&mut Bedrock) -> (&mut Vec<u32>, &mut Vec<u32>)) -> Vec<u32> {
    let mut bedrock = stored_bedrock();
    let (roster, promoted) = pick(&mut bedrock);

    if listable.is_empty() || listable == roster.as_slice() {
        return promoted.clone();
    }

    let held: HashSet<u32> = roster.iter().copied().collect();

    let arrivals: Vec<u32> = if held.is_empty() {
        Vec::new()
    } else {
        listable.iter().filter(|id| !held.contains(id)).copied().collect()
    };

    *roster = listable.to_vec();
    promoted.clone_from(&arrivals);

    save_bedrock(&bedrock);

    arrivals
}

pub(crate) fn commit(files: Vec<FileDelta>, touched: Vec<FileTouch>, after: Vec<Build>) -> bool {
    let before = builds();

    if !after.is_empty() && before != after {
        set_builds(&after);
    }

    if files.is_empty() && touched.is_empty() {
        return false;
    }

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let ore = Ore { schema: SCHEMA, stamp, before, after, files, touched };

    if let Err(err) = json::save_state(ORE, &ore) {
        warn!("Failed to save {}: {}", ORE, err);
        return false;
    }

    true
}

pub(crate) fn touch(file: &str, before: Option<u64>, after: u64) -> Option<FileTouch> {
    match before {
        None => Some(FileTouch { file: file.to_string(), status: Status::Baseline }),
        Some(held) if held != after => Some(FileTouch { file: file.to_string(), status: Status::Changed }),
        Some(_) => None,
    }
}

pub(crate) fn delta(file: &str, from: &str, region: &str, before: Option<&[u8]>, after: &[u8]) -> Option<FileDelta> {
    let keying = keying(file)?;
    let after_text = String::from_utf8_lossy(after).into_owned();
    let after_rows = rows(&after_text, keying);

    let Some(before) = before else {
        return Some(FileDelta {
            file: file.to_string(),
            from: from.to_string(),
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
        from: from.to_string(),
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
    // A unit that only ever existed as an art-less placeholder becomes a real release the
    // moment its art lands, which is a new unit rather than a changed one.
    #[test]
    fn a_first_roster_is_a_baseline_rather_than_a_wall_of_promotions() {
        let mut bedrock = Bedrock::default();
        assert!(bedrock.roster.is_empty());

        bedrock.roster = vec![1, 2, 3];
        let held: HashSet<u32> = bedrock.roster.iter().copied().collect();
        let promoted: Vec<u32> = [1, 2, 3, 4].iter().filter(|id| !held.contains(id)).copied().collect();

        assert_eq!(promoted, vec![4]);
    }

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

        let delta = delta("unitbuy.csv", "en", "en", Some(old), new).expect("changed");

        assert_eq!(delta.rows_before, 2);
        assert_eq!(delta.added(), 1);
        assert_eq!(delta.rows[0].key, "2", "the third line is unit two");
    }

    #[test]
    fn a_first_sighting_of_a_file_is_a_baseline_rather_than_a_wall_of_additions() {
        let delta = delta("SkillAcquisition.csv", "en", "en", None, OLD).expect("baseline");

        assert_eq!(delta.status, Status::Baseline);
        assert_eq!(delta.rows_after, 2);
        assert!(delta.rows.is_empty(), "a baseline must not report every row as new");
    }

    #[test]
    fn an_inserted_row_does_not_smear_the_rows_beneath_it() {
        // A mid-file insertion is the case a line diff misreads as a mass rewrite.
        let new = b"1,0,10,5,100,200,0,0,0,0,0,0,1,1,0\n7,0,99,5,1,2,0,0,0,0,0,0,3,3,0\n2,0,11,5,50,60,0,0,0,0,0,0,2,2,0\n";
        let delta = delta("SkillAcquisition.csv", "en", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.added(), 1);
        assert_eq!(delta.modified(), 0);
        assert_eq!(delta.removed(), 0);
        assert_eq!(delta.rows[0].key, "7");
    }

    #[test]
    fn an_edited_row_is_reported_with_both_sides() {
        let new = b"1,0,10,5,100,300,0,0,0,0,0,0,1,1,0\n2,0,11,5,50,60,0,0,0,0,0,0,2,2,0\n";
        let delta = delta("SkillAcquisition.csv", "en", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.modified(), 1);
        assert_eq!(delta.rows[0].before.as_deref(), Some("1,0,10,5,100,200,0,0,0,0,0,0,1,1,0"));
    }

    // The Files tab reads a rewrite as "changed" and a first sighting as "new"; an
    // untouched file must stay out of both lists even though the import rewrote its row.
    #[test]
    fn a_file_is_listed_only_when_its_bytes_actually_moved() {
        assert!(touch("000_f.png", Some(7), 7).is_none());
        assert!(matches!(touch("000_f.png", Some(7), 8).map(|held| held.status), Some(Status::Changed)));
        assert!(matches!(touch("000_f.png", None, 8).map(|held| held.status), Some(Status::Baseline)));
    }

    #[test]
    fn an_identical_reimport_yields_nothing_to_mine() {
        assert!(delta("SkillAcquisition.csv", "en", "en", Some(OLD), OLD).is_none());
    }

    #[test]
    fn a_vanished_row_is_kept_so_a_downgrade_can_be_called_out() {
        let new = b"1,0,10,5,100,200,0,0,0,0,0,0,1,1,0\n";
        let delta = delta("SkillAcquisition.csv", "en", "en", Some(OLD), new).expect("changed");

        assert_eq!(delta.removed(), 1);
        assert_eq!(delta.rows_after, 1);
    }
}
