use std::path::{Path, PathBuf};

pub const DIR_SHEETS: &str = "game/sheets";
pub const DIR_UI: &str = "game/ui";
pub const DIR_TABLES: &str = "game/tables";

pub const DIR_IMG015: &str = "img015";
pub const DIR_IMG022: &str = "img022";
pub const DIR_GATYA_ITEM: &str = "gatyaitemD";
pub const DIR_LOCALIZABLE: &str = "localizable";

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