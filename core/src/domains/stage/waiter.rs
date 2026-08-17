use std::collections::HashMap;
use std::fs;

use nyanko::chapter::map::{LockSkipData, LockSkipDataEntry};
use nyanko::chapter::stage::{
    Battleground, CertificationPreset, DropChara, MapStageData, MapStageDataEntry, ScatCpuSetting,
    StageName, StageNameEntry,
};

use crate::Vfs;

pub fn battleground(vfs: &Vfs, filename: &str) -> Option<Battleground> {
    let bytes = read(vfs, filename)?;

    Battleground::parse(&bytes).ok()
}

pub(crate) fn certification_preset(vfs: &Vfs, filename: &str) -> Option<CertificationPreset> {
    let bytes = read(vfs, filename)?;

    CertificationPreset::parse(&bytes).ok()
}

pub(crate) fn drop_chara(vfs: &Vfs, filename: &str) -> HashMap<u32, u32> {
    read(vfs, filename)
        .and_then(|bytes| DropChara::parse(&bytes).ok())
        .map(|parsed| parsed.character_drops)
        .unwrap_or_default()
}

pub(crate) fn lockskipdata(vfs: &Vfs, filename: &str) -> HashMap<u32, LockSkipDataEntry> {
    read(vfs, filename)
        .and_then(|bytes| LockSkipData::parse(&bytes).ok())
        .map(|parsed| parsed.entries)
        .unwrap_or_default()
}

pub(crate) fn mapstagedata(vfs: &Vfs, filename: &str) -> Vec<MapStageDataEntry> {
    read(vfs, filename)
        .and_then(|bytes| MapStageData::parse(&bytes).ok())
        .map(|parsed| parsed.entries)
        .unwrap_or_default()
}

pub(crate) fn scatcpusetting(vfs: &Vfs, filename: &str) -> ScatCpuSetting {
    read(vfs, filename)
        .and_then(|bytes| ScatCpuSetting::parse(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn stagename(vfs: &Vfs, filename: &str) -> HashMap<u32, StageNameEntry> {
    let mut final_map: HashMap<u32, StageNameEntry> = HashMap::new();
    let paths = vfs.list(filename);

    for path in paths.iter().rev() {
        let Ok(bytes) = fs::read(path) else { continue; };
        let Ok(parsed) = StageName::parse(&bytes) else { continue; };

        for (map_id, entry) in parsed.entries {
            let existing = final_map.entry(map_id).or_insert(StageNameEntry { names: Vec::new() });

            if existing.names.len() < entry.names.len() {
                existing.names.resize(entry.names.len(), String::new());
            }

            for (i, name) in entry.names.into_iter().enumerate() {
                if !name.is_empty() {
                    existing.names[i] = name;
                }
            }
        }
    }

    final_map
}

fn read(vfs: &Vfs, filename: &str) -> Option<Vec<u8>> {
    let path = vfs.find(filename)?;

    fs::read(path).ok()
}
