pub mod cache;
pub mod json;

use std::path::PathBuf;

use crate::Vfs;

pub(crate) const ASSET_IMG015_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.png$";
pub(crate) const ASSET_015CUT_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.imgcut$";
pub(crate) const ASSET_IMG022_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.png$";
pub(crate) const ASSET_022CUT_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.imgcut$";
pub(crate) const LOCALIZEABLE_PATTERN: &str = r"^localizable(?:_([a-z]{2}))?\.tsv$";
pub(crate) const PARAM_PATTERN: &str = r"^param\.tsv$";

pub(crate) const AUDIO_OGG_PATTERN: &str = r"^.+\.ogg$";
pub(crate) const AUDIO_CAF_PATTERN: &str = r"^.+\.caf$";

pub(crate) const GATYA_ITEM_D_PATTERN: &str = r"^gatyaitemD_(\d{2,3})_([fz])\.png$";
pub(crate) const GATYA_ITEM_BUY_PATTERN: &str = r"^Gatyaitembuy\.csv$";
pub(crate) const GATYA_ITEM_NAME_PATTERN: &str = r"^GatyaitemName(?:_([a-z]{2}))?\.csv$";


pub const APP_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ja", "Japanese"),
    ("tw", "Taiwanese"),
    ("ko", "Korean"),
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];

pub fn gatya_item_icon(vfs: &Vfs, id: i32) -> Option<PathBuf> {
    let names = [
        format!("gatyaitemD_{:03}_f.png", id),
        format!("gatyaitemD_{:02}_f.png", id),
        format!("gatyaitemD_{}_f.png", id),
    ];

    let references: Vec<&str> = names.iter().map(String::as_str).collect();

    vfs.list_any(&references).into_iter().next()
}
