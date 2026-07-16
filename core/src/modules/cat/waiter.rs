use std::collections::HashMap;
use std::fs;
use std::path::Path;

use nyanko::cat::unit::{
    Battle, LevelCurve, SkillDescriptions, Talent,
    TalentCost, UnitBuy, UnitEvolve, UnitExplanation,
};
use tracing::trace;

use crate::common::resolver;
use crate::modules::cat::paths;

pub fn skillacquisition(cats_directory: &Path, priority: &[String]) -> HashMap<u16, Talent> {
    trace!("loading skill acquisition data");
    let mut map = HashMap::new();

    let Some(file_path) = resolver::get(cats_directory, [paths::SKILL_ACQUISITION], priority).into_iter().next() else {
        return map;
    };

    if let Ok(bytes) = fs::read(&file_path)
        && let Ok(parsed_data) = Talent::parse(&bytes) {
        map = parsed_data;
    }

    map
}

pub fn skilldescriptions(cats_directory: &Path, priority: &[String]) -> Vec<String> {
    trace!("loading skill descriptions");
    let base_dir = cats_directory.join(paths::DIR_SKILL_DESCRIPTIONS);

    let Some(file_path) = resolver::get(&base_dir, ["SkillDescriptions.csv"], priority).into_iter().next() else {
        return Vec::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return Vec::new();
    };

    let Ok(parsed_data) = SkillDescriptions::parse(&bytes) else {
        return Vec::new();
    };

    parsed_data.texts
}

pub fn skilllevel(cats_directory: &Path, priority: &[String]) -> HashMap<u8, TalentCost> {
    trace!("loading skill levels map");
    let Some(file_path) = resolver::get(cats_directory, [paths::SKILL_LEVEL], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return HashMap::new();
    };

    let Ok(parsed_data) = TalentCost::parse(&bytes) else {
        return HashMap::new();
    };

    parsed_data
}

pub fn unitbuy(cats_directory: &Path, priority: &[String]) -> HashMap<u32, UnitBuy> {
    trace!("loading unit buy matrix");
    let Some(file_path) = resolver::get(cats_directory, [paths::UNIT_BUY], priority).into_iter().next() else {
        return HashMap::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return HashMap::new();
    };

    let Ok(parsed_data) = UnitBuy::parse(&bytes) else {
        return HashMap::new();
    };

    parsed_data
}

pub fn unitevolve(cats_directory: &Path, priority: &[String]) -> HashMap<u32, UnitEvolve> {
    trace!("loading unit evolution mappings");
    let mut final_map: HashMap<u32, UnitEvolve> = HashMap::new();
    let base_directory = cats_directory.join(paths::DIR_UNIT_EVOLVE);

    for file_path in resolver::get(&base_directory, ["unitevolve.csv"], priority) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed_map) = UnitEvolve::parse(&bytes) else {
            continue;
        };

        for (cat_id, parsed_evolve) in parsed_map {
            let entry = final_map.entry(cat_id).or_default();

            for index in 0..4 {
                if entry.texts[index].is_none() && parsed_evolve.texts[index].is_some() {
                    entry.texts[index] = parsed_evolve.texts[index].clone();
                }
            }
        }
    }

    final_map
}

pub fn unitexplanation(cat_id: u32, original_folder_path: &Path, priority: &[String]) -> UnitExplanation {
    trace!(cat_id = cat_id, "loading unit localized explanations");
    let mut final_explanation = UnitExplanation::default();
    let cats_root_dir = Path::new(paths::DIR_CATS);
    let lang_directory = paths::lang(cats_root_dir, cat_id);
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);

    let mut search_dirs = Vec::new();
    if lang_directory.exists() {
        search_dirs.push(lang_directory);
    }
    search_dirs.push(original_folder_path.to_path_buf());

    for dir in search_dirs {
        let resolved_paths = resolver::get(&dir, [base_filename.as_str()], priority);

        for file_path in resolved_paths {
            let Ok(bytes) = fs::read(&file_path) else {
                continue;
            };

            let Ok(parsed_explanation) = UnitExplanation::parse(&bytes) else {
                continue;
            };

            for index in 0..4 {
                if final_explanation.names[index].is_none() && parsed_explanation.names[index].is_some() {
                    final_explanation.names[index] = parsed_explanation.names[index].clone();
                    final_explanation.descriptions[index] = parsed_explanation.descriptions[index].clone();
                }
            }
        }

        if final_explanation.names.iter().any(|name| name.is_some()) {
            break;
        }
    }

    final_explanation
}

pub fn unitlevel(cats_directory: &Path, priority: &[String]) -> Vec<LevelCurve> {
    trace!("loading global unit level curves");
    let Some(file_path) = resolver::get(cats_directory, [paths::UNIT_LEVEL], priority).into_iter().next() else {
        return Vec::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return Vec::new();
    };

    let Ok(parsed_data) = LevelCurve::parse(&bytes) else {
        return Vec::new();
    };

    parsed_data
}

pub fn unitid(cat_id: i32, priority: &[String]) -> Option<Vec<Battle>> {
    trace!(cat_id = cat_id, "fetching individual unit battle layout");
    let path_object = paths::stats(Path::new(paths::DIR_CATS), cat_id as u32);

    let base_dir = path_object.parent()?;
    let file_name = path_object.file_name()?.to_str()?;

    let resolved_path = resolver::get(base_dir, [file_name], priority).into_iter().next()?;

    let bytes = fs::read(resolved_path).ok()?;

    Battle::parse(&bytes).ok()
}