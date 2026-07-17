pub mod cache;
pub mod json;

use std::path::Path;
use std::path::PathBuf;

pub const DIR_SHEETS: &str = "game/sheets";
pub const DIR_UI: &str = "game/ui";
pub const DIR_TABLES: &str = "game/tables";

pub const DIR_IMG015: &str = "img015";
pub const DIR_IMG022: &str = "img022";
pub const DIR_GATYA_ITEM: &str = "gatyaitemD";
pub const DIR_LOCALIZABLE: &str = "localizable";

pub const ASSET_IMG015_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.png$";
pub const ASSET_015CUT_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.imgcut$";
pub const ASSET_IMG022_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.png$";
pub const ASSET_022CUT_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.imgcut$";
pub const LOCALIZEABLE_PATTERN: &str = r"^localizable(?:_([a-z]{2}))?\.tsv$";
pub const PARAM_PATTERN: &str = r"^param\.tsv$";

pub const AUDIO_OGG_PATTERN: &str = r"^.+\.ogg$";
pub const AUDIO_CAF_PATTERN: &str = r"^.+\.caf$";

pub const GATYA_ITEM_D_PATTERN: &str = r"^gatyaitemD_(\d{2,3})_([fz])\.png$";
pub const GATYA_ITEM_BUY_PATTERN: &str = r"^Gatyaitembuy\.csv$";
pub const GATYA_ITEM_NAME_PATTERN: &str = r"^GatyaitemName(?:_([a-z]{2}))?\.csv$";

pub const GLOBAL_CODES: &[&str] = &["de", "en", "es", "fr", "it", "th"];
pub const REGION_CODES: &[&str] = &["en", "jp", "kr", "tw"];

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

pub fn gatya_item_icon(root: &Path, id: i32) -> Option<PathBuf> {
    let base = root.join(DIR_UI).join(DIR_GATYA_ITEM);

    let p3 = base.join(format!("gatyaitemD_{:03}_f.png", id));
    if p3.exists() { return Some(p3); }

    let p2 = base.join(format!("gatyaitemD_{:02}_f.png", id));
    if p2.exists() { return Some(p2); }

    let p1 = base.join(format!("gatyaitemD_{}_f.png", id));
    if p1.exists() { return Some(p1); }

    None
}

pub fn param_tsv(root: &Path) -> PathBuf {
    root.join(DIR_TABLES).join("param.tsv")
}

pub fn localizable_folder(root: &Path) -> PathBuf {
    root.join(DIR_TABLES).join(DIR_LOCALIZABLE)
}