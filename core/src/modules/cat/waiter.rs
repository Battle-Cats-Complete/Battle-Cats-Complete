use std::fs;

use nyanko::cat::unit::{Battle, UnitExplanation};
use tracing::trace;

use crate::modules::cat::paths;
use crate::Vfs;

pub fn unitexplanation(vfs: &Vfs, cat_id: u32) -> UnitExplanation {
    trace!(cat_id = cat_id, "loading unit localized explanations");
    let mut final_explanation = UnitExplanation::default();
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);

    for file_path in vfs.list(&base_filename) {
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

    final_explanation
}

pub fn unitid(vfs: &Vfs, cat_id: i32) -> Option<Vec<Battle>> {
    trace!(cat_id = cat_id, "fetching individual unit battle layout");
    let file_name = paths::stats_file(cat_id as u32);

    let resolved_path = vfs.list(&file_name).into_iter().next()?;

    let bytes = fs::read(resolved_path).ok()?;

    Battle::parse(&bytes).ok()
}
