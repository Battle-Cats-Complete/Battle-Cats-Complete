pub mod cache;
pub mod json;

use std::path::Path;
use std::path::PathBuf;

const DIR_SHEETS: &str = "game/sheets";

pub(crate) const DIR_GATYA_ITEM: &str = "game/ui/gatyaitemD";

const DIR_IMG015: &str = "img015";
const DIR_IMG022: &str = "img022";

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

pub fn img015_folder(root: &Path) -> PathBuf {
    root.join(DIR_SHEETS).join(DIR_IMG015)
}

pub fn img022_folder(root: &Path) -> PathBuf {
    root.join(DIR_SHEETS).join(DIR_IMG022)
}

pub fn gatya_item_icon(id: i32, langs: &[String]) -> Option<PathBuf> {
    let names = [
        format!("gatyaitemD_{:03}_f.png", id),
        format!("gatyaitemD_{:02}_f.png", id),
        format!("gatyaitemD_{}_f.png", id),
    ];

    crate::common::get(Path::new(DIR_GATYA_ITEM), names, langs).into_iter().next()
}
