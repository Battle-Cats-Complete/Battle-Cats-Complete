use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::common::dirs;
use crate::common::io::json;

pub(crate) const LOOSE: &str = "";
pub(crate) const NONE: &str = "--";

const FILE: &str = "manifest.json";

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct FileRecord {
    pub winner: String,
    pub size: usize,
    pub encrypted: usize,
    pub checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Missing,
    Malformed,
}

#[derive(Deserialize)]
struct IndexRecord {
    #[serde(default)]
    files: HashMap<String, IgnoredAny>,
}

#[derive(Serialize, Deserialize, Default)]
struct PackRecord {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    checksums: HashMap<String, u64>,
    #[serde(default)]
    files: HashMap<String, FileRecord>,
}

type Stored = HashMap<String, PackRecord>;

#[derive(Clone)]
pub(crate) struct Placement {
    pub pack: String,
    pub record: FileRecord,
}

#[derive(Default)]
pub(crate) struct Ledger {
    packs: HashMap<String, HashMap<String, u64>>,
    files: HashMap<String, Placement>,
    faulted: bool,
}

impl Ledger {
    pub(crate) fn load() -> Self {
        let Some(path) = dirs::state().map(|directory| directory.join(FILE)).filter(|path| path.exists()) else {
            return Self::default();
        };

        let read = fs::read_to_string(&path).map_err(|error| error.to_string());
        let parsed = read.and_then(|data| serde_json::from_str::<Stored>(&data).map_err(|error| error.to_string()));

        match parsed {
            Ok(stored) => Self::from_stored(stored),
            Err(fault) => {
                warn!("Discarding {}: {}", FILE, fault);

                Self { faulted: true, ..Self::default() }
            }
        }
    }

    pub(crate) fn faulted(&self) -> bool {
        self.faulted
    }

    pub(crate) fn save(self) {
        if let Err(err) = json::save_state(FILE, &self.into_stored()) {
            warn!("Failed to save {}: {}", FILE, err);
        }
    }

    fn from_stored(stored: Stored) -> Self {
        let mut ledger = Self::default();

        for (pack, entry) in stored {
            if !entry.checksums.is_empty() {
                ledger.packs.insert(pack.clone(), entry.checksums);
            }

            for (filename, record) in entry.files {
                ledger.files.insert(filename, Placement { pack: pack.clone(), record });
            }
        }

        ledger
    }

    fn into_stored(self) -> Stored {

        let mut stored = Stored::new();

        for (pack, checksums) in self.packs {
            stored.entry(pack).or_default().checksums = checksums;
        }

        for (filename, placement) in self.files {
            stored.entry(placement.pack).or_default().files.insert(filename, placement.record);
        }

        stored
    }

    pub(crate) fn pack_checksum(&self, pack: &str, region: &str) -> Option<u64> {
        self.packs.get(pack).and_then(|regions| regions.get(region)).copied()
    }

    pub(crate) fn tracks_files(&self) -> bool {
        !self.files.is_empty()
    }

    pub(crate) fn placement(&self, filename: &str) -> Option<&Placement> {
        self.files.get(filename)
    }

    pub(crate) fn track(&mut self, pack: String, region: String, checksum: u64) -> bool {
        self.packs.entry(pack).or_default().insert(region, checksum) != Some(checksum)
    }

    pub(crate) fn place(&mut self, filename: String, placement: Placement) {
        self.files.insert(filename, placement);
    }
}

pub(crate) fn reset() {
    let Some(directory) = dirs::state() else {
        return;
    };

    let path = directory.join(FILE);

    if path.exists()
        && let Err(err) = fs::remove_file(&path)
    {
        warn!("Failed to reset {}: {}", FILE, err);
    }
}

pub(crate) fn index() -> Result<HashMap<String, String>, Fault> {
    let Some(directory) = dirs::state() else {
        return Err(Fault::Missing);
    };

    let Ok(data) = fs::read_to_string(directory.join(FILE)) else {
        return Err(Fault::Missing);
    };

    let stored: HashMap<String, IndexRecord> = match serde_json::from_str(&data) {
        Ok(stored) => stored,
        Err(error) => {
            warn!("Could not parse {}: {}", FILE, error);
            return Err(Fault::Malformed);
        }
    };

    Ok(stored
        .into_iter()
        .flat_map(|(pack, entry)| entry.files.into_keys().map(move |filename| (filename, pack.clone())))
        .collect())
}

pub(crate) fn stamp() -> Option<SystemTime> {
    dirs::state()
        .and_then(|directory| fs::metadata(directory.join(FILE)).ok())
        .and_then(|data| data.modified().ok())
}

pub(crate) fn hash(data: &[u8]) -> u64 {
    let mut current_hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        current_hash ^= byte as u64;
        current_hash = current_hash.wrapping_mul(0x100000001b3);
    }
    current_hash
}

pub(crate) fn holds(path: &Path, size: usize, checksum: u64) -> bool {
    fs::metadata(path).is_ok_and(|data| data.len() == size as u64)
        && hash_file(path).is_ok_and(|hash| hash == checksum)
}

pub(crate) fn hash_file(path: &Path) -> std::io::Result<u64> {
    let mut file = File::open(path)?;
    let mut current_hash: u64 = 0xcbf29ce484222325;
    let mut buffer = vec![0u8; 65536];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        for &byte in &buffer[..bytes_read] {
            current_hash ^= byte as u64;
            current_hash = current_hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(current_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(winner: &str, size: usize) -> FileRecord {
        FileRecord { winner: winner.to_string(), size, encrypted: size, checksum: size as u64 }
    }

    fn placed(pack: &str, winner: &str, size: usize) -> Placement {
        Placement { pack: pack.to_string(), record: record(winner, size) }
    }

    fn stored_with(pack: &str, checksums: &[(&str, u64)], files: &[(&str, &str, usize)]) -> Stored {
        let entry = PackRecord {
            checksums: checksums.iter().map(|(region, sum)| ((*region).to_string(), *sum)).collect(),
            files: files.iter().map(|(name, winner, size)| ((*name).to_string(), record(winner, *size))).collect(),
        };

        HashMap::from([(pack.to_string(), entry)])
    }

    // One pack name shipped by several regions stays a single entry, holding a
    // checksum per region, with each file naming the region that actually won it.
    // A workspace that already holds the exact bytes an import is about to write must be
    // recognised, so a manifest-less re-import records the placement without rewriting the file.
    #[test]
    fn a_file_already_holding_the_winning_bytes_is_recognised() {
        let directory = std::env::temp_dir().join("bcc-holds-probe");
        fs::create_dir_all(&directory).expect("probe dir");

        let path = directory.join("unitbuy.csv");
        let body = b"0,0,50\n0,1,60\n";
        fs::write(&path, body).expect("probe file");

        let checksum = hash(body);

        assert!(holds(&path, body.len(), checksum));
        assert!(!holds(&path, body.len() + 1, checksum), "a size mismatch is never a match");
        assert!(!holds(&path, body.len(), checksum ^ 1), "a checksum mismatch is never a match");
        assert!(!holds(&directory.join("absent.csv"), body.len(), checksum));

        let _ = fs::remove_dir_all(&directory);
    }

    // A re-import that finds every pack already recorded must not rewrite the manifest,
    // or the Import activity timer reads "moments ago" when nothing was imported.
    #[test]
    fn tracking_an_identical_pack_reports_no_change() {
        let mut ledger = Ledger::from_stored(stored_with("DownloadLocal.pack", &[("en", 11)], &[]));

        assert!(!ledger.track("DownloadLocal.pack".to_string(), "en".to_string(), 11));
        assert!(ledger.track("DownloadLocal.pack".to_string(), "en".to_string(), 22), "a new checksum is a change");
        assert!(ledger.track("DownloadLocal.pack".to_string(), "ja".to_string(), 11), "a new region is a change");
        assert!(ledger.track("AUnitServer.pack".to_string(), "en".to_string(), 11), "a new pack is a change");
    }

    #[test]
    fn a_shared_pack_name_keeps_one_entry_across_regions() {
        let mut ledger = Ledger::from_stored(stored_with(
            "DownloadLocal.pack",
            &[("ja", 11)],
            &[("real.png", "ja", 400), ("dummy.png", "ja", 4)],
        ));

        ledger.track("DownloadLocal.pack".to_string(), "en".to_string(), 22);
        ledger.place("dummy.png".to_string(), placed("DownloadLocal.pack", "en", 900));

        let stored = ledger.into_stored();

        assert_eq!(stored.len(), 1);

        let pack = &stored["DownloadLocal.pack"];
        assert_eq!(pack.checksums, HashMap::from([("ja".to_string(), 11), ("en".to_string(), 22)]));
        assert_eq!(pack.files["real.png"].winner, "ja");
        assert_eq!(pack.files["dummy.png"].winner, "en");
        assert_eq!(pack.files["dummy.png"].size, 900);
    }

    // A file served first by the APK's local pack and later by a server pack must
    // end up under the server pack only, never listed twice.
    #[test]
    fn moving_a_file_to_another_pack_leaves_no_duplicate() {
        let mut ledger = Ledger::from_stored(stored_with("DataLocal.pack", &[("en", 11)], &[("unit001.csv", "en", 40)]));

        ledger.place("unit001.csv".to_string(), placed("AUnitServer.pack", "en", 40));
        ledger.track("AUnitServer.pack".to_string(), "en".to_string(), 22);

        let stored = ledger.into_stored();

        assert!(stored["DataLocal.pack"].files.is_empty());
        assert_eq!(stored["DataLocal.pack"].checksums["en"], 11);
        assert_eq!(stored["AUnitServer.pack"].files.len(), 1);
        assert_eq!(stored["AUnitServer.pack"].checksums["en"], 22);
    }

    #[test]
    fn loose_files_round_trip_under_an_unnamed_pack_without_a_checksum() {
        let ledger = Ledger::from_stored(stored_with(LOOSE, &[], &[("loose.png", NONE, 5)]));

        assert_eq!(ledger.pack_checksum(LOOSE, NONE), None);
        assert_eq!(ledger.placement("loose.png").map(|p| p.pack.as_str()), Some(LOOSE));

        let json = serde_json::to_string(&ledger.into_stored()).expect("serialize");
        assert_eq!(json, r#"{"":{"files":{"loose.png":{"winner":"--","size":5,"encrypted":5,"checksum":5}}}}"#);
    }
}


