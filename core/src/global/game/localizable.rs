use std::fs;
use std::path::Path;

use nyanko::common::csv::scrub;
use tracing::{debug, error, info, trace, warn};

use crate::global::resolver;

#[derive(Default, Debug, Clone)]
pub struct Localizable {
    data: Vec<u8>,
}

impl Localizable {
    pub fn lookup(&self, key: &str) -> Option<String> {
        trace!(key, "Starting localizable lookup");

        if self.data.is_empty() {
            warn!("Localizable data is empty, cannot look up key: {}", key);
            return None;
        }

        let content = scrub(&self.data);

        for line in content.lines() {
            let clean_line = line.split("//").next().unwrap_or("").trim();

            if clean_line.is_empty() {
                continue;
            }

            let Some(tab_index) = clean_line.find('\t') else {
                continue;
            };

            let current_key = clean_line[..tab_index].trim();

            if current_key != key {
                continue;
            }

            let value = clean_line[tab_index..].trim().to_string();
            trace!(key = current_key, value = %value, "Localization match found");

            return Some(value);
        }

        trace!(key, "No localization match found");
        None
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

    info!(size = data.len(), path = %file_path.display(), "Successfully loaded localization bytes");

    Localizable { data }
}