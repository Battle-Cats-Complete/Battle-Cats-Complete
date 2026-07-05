use std::collections::HashMap;
use std::fs;
use std::path::Path;

use nyanko::common::csv::scrub;
use tracing::{debug, error, info, trace, warn};

use crate::global::resolver;

#[derive(Default, Debug, Clone)]
pub struct Localizable {
    map: HashMap<String, String>,
}

impl Localizable {
    pub fn lookup(&self, key: &str) -> Option<String> {
        trace!(key, "Starting localizable lookup");

        let result = self.map.get(key).cloned();

        if result.is_some() {
            trace!(key, "Localization match found");
        } else {
            trace!(key, "No localization match found");
        }

        result
    }
}

pub fn load_localizable(dir: &Path, priority: &[String]) -> Localizable {
    info!("Initializing localizable dictionary load");
    trace!(directory = %dir.display(), "Attempting to load localizable.tsv");

    let paths = resolver::get(dir, ["localizable.tsv"], priority);

    let Some(file_path) = paths.first() else {
        warn!("Could not find any localizable.tsv file in the given path");
        return Localizable::default();
    };

    debug!(path = %file_path.display(), "Located localizable file, reading raw bytes");

    let Ok(data) = fs::read(file_path) else {
        error!(path = %file_path.display(), "Found localizable.tsv, but failed to read byte data");
        return Localizable::default();
    };

    debug!("Scrubbing raw bytes and building lookup index");
    let content = scrub(&data);

    // A conservative divisor of 50 intentionally overestimates the line count.
    // This slightly over-allocates memory upfront (extremely cheap) to guarantee
    // absolutely zero reallocations during the loop (extremely expensive).
    let estimated_entries = data.len() / 50;
    let mut map = HashMap::with_capacity(estimated_entries);

    for line in content.lines() {
        let clean_line = line.split("//").next().unwrap_or("").trim();

        if clean_line.is_empty() {
            continue;
        }

        let Some(tab_index) = clean_line.find('\t') else {
            continue;
        };

        let current_key = clean_line[..tab_index].trim().to_string();
        let value = clean_line[tab_index..].trim().to_string();

        map.insert(current_key, value);
    }

    info!(entries = map.len(), path = %file_path.display(), "Successfully loaded and indexed localization data");

    Localizable { map }
}