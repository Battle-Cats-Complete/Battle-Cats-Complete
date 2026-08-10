use std::fs;

use nyanko::common::data::Localizable;
use nyanko::common::data::Param;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::Vfs;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum CustomIcon {
    #[default] None,
    Multihit,
    Kamikaze,
    BossWave,
    Dojo,
    StarredAlien,
    Burrow,
    Revive,
    Stop,
    DeathTimer,
    God,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct AbilityItem {
    pub icon_id: Option<usize>,
    pub text: String,
    pub custom_icon: CustomIcon,
    pub border_id: Option<usize>,
}

pub type AbilityGroups = (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>);

pub const ABILITY_X: f32 = 3.0;
pub const ABILITY_Y: f32 = 5.0;
pub const TRAIT_Y: f32 = 7.0;

pub fn localizable(vfs: &Vfs) -> Localizable {
    info!("Initializing localizable dictionary load");

    let paths = vfs.list("localizable.tsv");

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

pub fn param(vfs: &Vfs) -> Option<Param> {
    info!("Initializing global parameters load");

    let Some(file_path) = vfs.list("param.tsv").into_iter().next() else {
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