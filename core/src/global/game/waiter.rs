use std::fs;
use std::path::Path;

use nyanko::common::data::{Localizable, Param};
use tracing::{debug, error, info, warn};

use crate::global::resolver;

pub fn localizable(dir: &Path, priority: &[String]) -> Localizable {
    info!("Initializing localizable dictionary load");

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

    debug!("Parsing localizable TSV bytes via nyanko API");

    let Ok(parsed_data) = Localizable::parse(&data) else {
        error!("Failed to parse localizable data");
        return Localizable::default();
    };

    info!(path = %file_path.display(), "Successfully loaded and indexed localization data");

    parsed_data
}

pub fn param(data_directory: &Path, priority: &[String]) -> Option<Param> {
    info!("Initializing global parameters load");

    let Some(file_path) = resolver::get(data_directory, ["param.tsv"], priority).into_iter().next() else {
        warn!("Could not find param.tsv in the given path");
        return None;
    };

    debug!(path = %file_path.display(), "Located param file, reading raw bytes");

    let Ok(bytes) = fs::read(&file_path) else {
        error!(path = %file_path.display(), "Found param.tsv, but failed to read byte data");
        return None;
    };

    debug!("Parsing parameter TSV bytes via nyanko API");

    let Ok(parsed_data) = Param::parse(&bytes) else {
        error!(path = %file_path.display(), "Failed to parse param data");
        return None;
    };

    info!(path = %file_path.display(), "Successfully loaded global parameters");

    Some(parsed_data)
}