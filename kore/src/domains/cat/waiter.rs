use std::fs;
use std::path::PathBuf;

use nyanko::cat::unitid;
use nyanko::cat::unit::UnitExplanation;
use nyanko::combat::Entity;
use tracing::trace;

use crate::domains::cat::files;
use crate::Vfs;

pub fn unitexplanation(vfs: &Vfs, cat_id: u32) -> UnitExplanation {
    trace!(cat_id = cat_id, "loading unit localized explanations");
    let mut final_explanation = UnitExplanation::default();
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);

    for file_path in vfs.list(&base_filename) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed_explanation) = UnitExplanation::parse(&bytes, Some(delimiter(&file_path))) else {
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

pub fn unitexplanation_source(vfs: &Vfs, cat_id: u32, form: usize) -> Option<PathBuf> {
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);
    let mut fallback = None;

    for file_path in vfs.list(&base_filename) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed) = UnitExplanation::parse(&bytes, Some(delimiter(&file_path))) else {
            continue;
        };

        if parsed.names.get(form).is_some_and(Option::is_some)
            || parsed.descriptions.get(form).is_some_and(Option::is_some)
        {
            return Some(file_path);
        }

        if fallback.is_none() {
            fallback = Some(file_path);
        }
    }

    fallback.or_else(|| vfs.find(&base_filename))
}

pub fn unitid(vfs: &Vfs, cat_id: i32) -> Option<Vec<Entity>> {
    trace!(cat_id = cat_id, "fetching individual unit battle layout");
    let file_name = files::stats_file(cat_id as u32);

    let resolved_path = vfs.find(&file_name)?;

    let bytes = fs::read(resolved_path).ok()?;

    unitid::parse(&bytes, None).ok()
}

fn delimiter(path: &std::path::Path) -> nyanko::combat::Separator {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    crate::common::region::text_separator(&name)
}
