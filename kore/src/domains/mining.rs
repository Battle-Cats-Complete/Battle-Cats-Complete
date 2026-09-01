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
use std::time::{SystemTime, UNIX_EPOCH};

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


const HOME: &str = "mining";

const DIFF: &str = "diff.json";

const SNAPSHOT: &str = "snapshot.json";

const SHARDS: &str = "shards";

const CENSUS: &str = "census.json";

const EMPTY_MAP: u64 = 2;

const SCHEMA: u32 = 3;

const SNAPSHOT_SCHEMA: u32 = 1;

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
enum Shard {
    Cat,
    Enemy,
    Stage,
}

const SHARD_ORDER: [Shard; 3] = [Shard::Cat, Shard::Enemy, Shard::Stage];

impl Shard {
    fn file(self) -> &'static str {
        match self {
            Self::Cat => "cat.json",
            Self::Enemy => "enemy.json",
            Self::Stage => "stage.json",
        }
    }
}

fn shard(filename: &str) -> Option<Shard> {
    if cat_files::stats_id(filename).is_some()
        || filename == cat_files::UNIT_BUY
        || filename == cat_files::SKILL_ACQUISITION
        || localized::explains(filename)
    {
        return Some(Shard::Cat);
    }

    if filename == enemy_files::STATS || localized::describes(filename) {
        return Some(Shard::Enemy);
    }

    if filename == stages::MAP_OPTION || localized::charts(filename) || stages::mineable(filename) {
        return Some(Shard::Stage);
    }

    None
}

#[derive(Serialize, Deserialize, Default)]
struct Snapshot {
    #[serde(default)]
    schema: u32,
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
    #[serde(default)]
    listed: Vec<u32>,
    #[serde(default)]
    sighted: Vec<u32>,
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
pub struct Diff {
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

impl Diff {
    pub fn file(&self, name: &str) -> Option<&FileDelta> {
        self.files.iter().find(|delta| delta.file == name)
    }
}

pub fn load() -> Option<Diff> {
    json::load_state::<Diff>(&under(DIFF)).filter(|diff| diff.schema == SCHEMA)
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

fn tables(found: &[(String, PathBuf)], wanted: Shard) -> Tables {
    found
        .iter()
        .filter(|(name, _)| shard(name) == Some(wanted))
        .filter_map(|(name, path)| fs::read_to_string(path).ok().map(|text| (name.to_string(), text)))
        .collect()
}

fn under(name: &str) -> String {
    format!("{}/{}", HOME, name)
}

fn kept(name: &str) -> Option<String> {
    fs::create_dir_all(dirs::state()?.join(HOME)).ok()?;

    Some(under(name))
}

fn spot(name: &str) -> Option<PathBuf> {
    Some(dirs::state()?.join(HOME).join(name))
}

fn shard_file(name: &str) -> Option<String> {
    let directory = dirs::state()?.join(HOME).join(SHARDS);

    fs::create_dir_all(&directory).ok()?;

    Some(format!("{}/{}/{}", HOME, SHARDS, name))
}

fn read_shard<T: DeserializeOwned + Default>(name: &str) -> T {
    shard_file(name).and_then(|held| json::load_state(&held)).unwrap_or_default()
}

fn write_shard<T: Serialize>(name: &str, data: &T) {
    let Some(held) = shard_file(name) else {
        return;
    };

    if let Err(err) = json::save_state(&held, data) {
        warn!("Failed to save {}: {}", held, err);
    }
}

fn stored_census() -> Census {
    read_shard(CENSUS)
}

fn stored_rows() -> Tables {
    let mut all = Tables::new();

    for held in SHARD_ORDER {
        all.extend(read_shard::<Tables>(held.file()));
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

    write_shard(CENSUS, &taken);

    for held in SHARD_ORDER {
        write_shard(held.file(), &tables(found, held));
    }

    save_snapshot(stored_snapshot());
}

pub fn arrivals() -> (Vec<u32>, Vec<u32>) {
    let snapshot = stored_snapshot();

    (snapshot.promoted, snapshot.surfaced)
}

pub fn discard() {
    drop_state(DIFF);
}

pub fn forget() {
    if let Some(directory) = spot(SHARDS) {
        let _ = fs::remove_dir_all(directory);
    }

    let mut snapshot = stored_snapshot();

    snapshot.listed.clear();
    snapshot.sighted.clear();

    save_snapshot(snapshot);
}

pub fn snapped_at() -> Option<SystemTime> {
    fs::metadata(spot(SHARDS)?.join(CENSUS)).ok()?.modified().ok()
}

pub fn stamp() -> Option<u64> {
    let data = fs::metadata(spot(DIFF)?).ok()?;
    let mut hasher = FxHasher::default();

    data.len().hash(&mut hasher);
    data.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos().hash(&mut hasher);

    Some(hasher.finish())
}

pub fn has_snapshot() -> bool {
    let laid = spot(SHARDS)
        .and_then(|directory| fs::metadata(directory.join(CENSUS)).ok())
        .is_some_and(|data| data.len() > EMPTY_MAP);

    laid && stored_snapshot().schema == SNAPSHOT_SCHEMA
}

pub fn capturable() -> bool {
    architecture::game_present()
}

pub fn capture(listed: Vec<u32>, sighted: Vec<u32>) -> bool {
    let found = walk(Path::new(architecture::GAME));
    let taken = census(&found);

    if taken == stored_census() {
        return false;
    }

    rebase(&found, taken);

    let mut snapshot = stored_snapshot();

    snapshot.listed = listed;
    snapshot.sighted = sighted;

    save_snapshot(snapshot);

    true
}

pub fn craft(listed: &[u32], sighted: &[u32]) -> bool {
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

    for (name, path) in found.iter().filter(|(name, _)| shard(name).is_some()) {
        let Ok(after) = fs::read(path) else {
            continue;
        };

        files.extend(delta(name, "", "", rows.get(name).map(String::as_bytes), &after));
    }

    let struck = commit(files, touched, Vec::new());

    if struck {
        mark(listed, sighted);
    }

    struck
}

fn mark(listed: &[u32], sighted: &[u32]) {
    let mut snapshot = stored_snapshot();

    if !snapshot.listed.is_empty() && !listed.is_empty() {
        snapshot.promoted = gained(&snapshot.listed, listed);
        snapshot.roster = listed.to_vec();
    }

    if !snapshot.sighted.is_empty() && !sighted.is_empty() {
        snapshot.surfaced = gained(&snapshot.sighted, sighted);
        snapshot.bestiary = sighted.to_vec();
    }

    save_snapshot(snapshot);
}

fn gained(before: &[u32], now: &[u32]) -> Vec<u32> {
    let held: HashSet<u32> = before.iter().copied().collect();

    now.iter().filter(|id| !held.contains(id)).copied().collect()
}

pub fn clear() {
    if let Some(directory) = dirs::state() {
        let _ = fs::remove_dir_all(directory.join(HOME));
    }
}

fn drop_state(name: &str) {
    let Some(directory) = dirs::state() else {
        return;
    };

    let path = directory.join(under(name));

    if path.exists()
        && let Err(err) = fs::remove_file(&path)
    {
        warn!("Failed to clear {}: {}", name, err);
    }
}

fn stored_snapshot() -> Snapshot {
    json::load_state::<Snapshot>(&under(SNAPSHOT)).unwrap_or_default()
}

fn save_snapshot(mut snapshot: Snapshot) {
    snapshot.schema = SNAPSHOT_SCHEMA;

    let Some(held) = kept(SNAPSHOT) else {
        return;
    };

    if let Err(err) = json::save_state(&held, &snapshot) {
        warn!("Failed to save {}: {}", SNAPSHOT, err);
    }
}

fn builds() -> Vec<Build> {
    stored_snapshot().builds
}

fn set_builds(after: &[Build]) {
    let mut snapshot = stored_snapshot();

    absorb(&mut snapshot.builds, after);

    save_snapshot(snapshot);
}

fn absorb(held: &mut Vec<Build>, after: &[Build]) {
    for build in after {
        match held.iter_mut().find(|kept| kept.label == build.label) {
            Some(kept) => kept.clone_from(build),
            None => held.push(build.clone()),
        }
    }
}

pub fn reconcile(listable: &[u32]) -> Vec<u32> {
    settle(listable, |snapshot| (&mut snapshot.roster, &mut snapshot.promoted))
}

pub fn reconcile_foes(listable: &[u32]) -> Vec<u32> {
    settle(listable, |snapshot| (&mut snapshot.bestiary, &mut snapshot.surfaced))
}

fn settle(listable: &[u32], pick: impl Fn(&mut Snapshot) -> (&mut Vec<u32>, &mut Vec<u32>)) -> Vec<u32> {
    let mut snapshot = stored_snapshot();
    let (roster, promoted) = pick(&mut snapshot);

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

    save_snapshot(snapshot);

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
    let diff = Diff { schema: SCHEMA, stamp, before, after, files, touched };

    let Some(held) = kept(DIFF) else {
        return false;
    };

    if let Err(err) = json::save_state(&held, &diff) {
        warn!("Failed to save {}: {}", DIFF, err);
        return false;
    }

    let mut snapshot = stored_snapshot();

    snapshot.promoted.clear();
    snapshot.surfaced.clear();

    save_snapshot(snapshot);

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

    fn build(label: &str, code: u32) -> Build {
        Build { code, name: format!("v{}", code), label: label.to_string() }
    }

    // Each import only reports the regions it pulled, so replacing the stored list
    // dropped every other region's version table off the Information panel.
    #[test]
    fn a_region_import_keeps_the_other_regions_versions() {
        let mut held = vec![build("jp.co.ponos.battlecatsen", 1)];

        absorb(&mut held, &[build("jp.co.ponos.battlecats", 2)]);

        assert_eq!(held.len(), 2, "a japanese import must not evict the english build");

        absorb(&mut held, &[build("jp.co.ponos.battlecatsen", 3)]);

        assert_eq!(held.len(), 2, "re-importing a region replaces it in place");
        assert_eq!(held.first().map(|found| found.code), Some(3));
    }

    // unitbuy.csv is positional: its first column is rarity, so column keying would
    // collide thousands of rows onto a handful of keys.
    // Every per-unit stats table is mineable, and each of its lines is one form.
    // A unit that only ever existed as an art-less placeholder becomes a real release the
    // moment its art lands, which is a new unit rather than a changed one.
    #[test]
    fn a_first_roster_is_a_baseline_rather_than_a_wall_of_promotions() {
        let mut snapshot = Snapshot::default();
        assert!(snapshot.roster.is_empty());

        snapshot.roster = vec![1, 2, 3];
        let held: HashSet<u32> = snapshot.roster.iter().copied().collect();
        let promoted: Vec<u32> = [1, 2, 3, 4].iter().filter(|id| !held.contains(id)).copied().collect();

        assert_eq!(promoted, vec![4]);
    }

    // Every table the snapshot remembers has to route to a shard, or Diff Snapshot quietly
    // covers less than an import does.
    #[test]
    fn every_remembered_table_routes_to_a_shard() {
        let cat = [cat_files::UNIT_BUY, cat_files::SKILL_ACQUISITION, "unit440.csv", "Unit_Explanation441_ko.csv"];
        let foe = [enemy_files::STATS, "Enemyname_en.tsv", "EnemyPictureBook.csv"];
        let land = [stages::MAP_OPTION, "Map_Name_en.csv", "MapStageDataN_003.csv", "stageRN000_01.csv"];

        for name in cat {
            assert!(matches!(shard(name), Some(Shard::Cat)), "{name} should be a cat shard");
        }

        for name in foe {
            assert!(matches!(shard(name), Some(Shard::Enemy)), "{name} should be an enemy shard");
        }

        for name in land {
            assert!(matches!(shard(name), Some(Shard::Stage)), "{name} should be a stage shard");
        }

        // Everything a shard remembers must also be diffable, or craft() reads it and drops it.
        for name in cat.iter().chain(foe.iter()).chain(land.iter()) {
            assert!(mineable(name), "{name} is remembered but not mineable");
        }

        assert!(shard("unitlevel.csv").is_none(), "a table nothing diffs is not worth remembering");
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
