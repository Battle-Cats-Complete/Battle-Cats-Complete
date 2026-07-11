use std::collections::HashMap;
use std::fs;
use std::path::Path;

use nyanko::chapter::map::{
    DropItem, DropItemEntry, ExOption, LockSkipData, LockSkipDataEntry, MapName
};
use nyanko::chapter::stage::{
    Battleground, CertificationPreset, CharaGroup, CharaGroupEntry, DifficultyLevel,
    DropChara, FixedFormation, FixedFormationEntry
};

use crate::global::resolver;

pub fn battleground(dir_path: &Path, filename: &str, priority: &[String]) -> Option<Battleground> {
    let resolved_path = resolver::get(dir_path, [filename], priority).into_iter().next()?;

    let bytes = fs::read(&resolved_path).ok()?;

    Battleground::parse(&bytes).ok()
}

pub fn certification_preset(dir: &Path, filename: &str, priority: &[String]) -> Option<CertificationPreset> {
    let resolved_path = resolver::get(dir, [filename], priority).into_iter().next()?;

    let bytes = fs::read(&resolved_path).ok()?;

    CertificationPreset::parse(&bytes).ok()
}

pub fn charagroup(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, CharaGroupEntry> {
    let Some(resolved_path) = resolver::get(dir, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    CharaGroup::parse(&bytes).map(|parsed| parsed.groups).unwrap_or_default()
}

pub fn difficulty_level(dir_path: &Path, filename: &str, priority: &[String]) -> HashMap<u32, Vec<u16>> {
    let Some(resolved_path) = resolver::get(dir_path, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    DifficultyLevel::parse(&bytes).map(|parsed| parsed.map_difficulties).unwrap_or_default()
}

pub fn drop_chara(dir_path: &Path, filename: &str, priority: &[String]) -> HashMap<u32, u32> {
    let Some(resolved_path) = resolver::get(dir_path, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    DropChara::parse(&bytes).map(|parsed| parsed.character_drops).unwrap_or_default()
}

pub fn dropitem(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, DropItemEntry> {
    let Some(resolved_path) = resolver::get(dir, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    DropItem::parse(&bytes).map(|parsed| parsed.map_drops).unwrap_or_default()
}

pub fn ex_option(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, u32> {
    let Some(resolved_path) = resolver::get(dir, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    ExOption::parse(&bytes).map(|parsed| parsed.map_to_ex_map).unwrap_or_default()
}

pub fn fixed_formation(dir: &Path, filename: &str, priority: &[String]) -> HashMap<(u32, u8, u32), FixedFormationEntry> {
    let Some(resolved_path) = resolver::get(dir, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    FixedFormation::parse(&bytes).map(|parsed| parsed.formations).unwrap_or_default()
}

pub fn lockskipdata(dir_path: &Path, filename: &str, priority: &[String]) -> HashMap<u32, LockSkipDataEntry> {
    let Some(resolved_path) = resolver::get(dir_path, [filename], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&resolved_path) else {
        return HashMap::new();
    };

    LockSkipData::parse(&bytes).map(|parsed| parsed.entries).unwrap_or_default()
}

pub fn map_name(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, String> {
    let mut final_map = HashMap::new();
    let paths = resolver::get(dir, [filename], priority);

    for path in paths.iter().rev() {
        let Ok(bytes) = fs::read(path) else { continue; };
        let Ok(parsed) = MapName::parse(&bytes) else { continue; };

        final_map.extend(parsed.names);
    }

    final_map
}