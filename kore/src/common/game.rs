use std::fs;

use nyanko::files::Localizable;
use nyanko::files::Param;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::Vfs;

pub fn localizable(vfs: &Vfs) -> Localizable {
    info!("Initializing localizable dictionary load");

    let Some(file_path) = vfs.find("localizable.tsv") else {
        warn!("Could not find any localizable.tsv file in the given path");
        return Localizable::default();
    };

    debug!(path = %file_path.display(), "Located localizable file, reading raw bytes");

    let Ok(data) = fs::read(&file_path) else {
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

pub fn param(vfs: &Vfs) -> Option<Param> {
    info!("Initializing global parameters load");

    let Some(file_path) = vfs.find("param.tsv") else {
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