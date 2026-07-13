use std::path::{Path, PathBuf};
use nyanko::chapter::Category;

pub const DIR_STAGES: &str = "game/stages";
pub const DIR_CATEGORIES: &str = "game/stages/categories";
pub const DIR_CATS: &str = "game/cats";
pub const DIR_ENEMIES: &str = "game/enemies";
pub const DIR_TABLES: &str = "game/tables";
pub const DIR_GATYA_ITEM: &str = "game/ui/gatyaitemD";

pub fn stage_sub(sub: &str) -> PathBuf {
    Path::new(DIR_STAGES).join(sub)
}

pub fn gatya_name_dir() -> PathBuf {
    Path::new(DIR_TABLES).join("GatyaitemName")
}

pub fn gatya_item_img(id: u32) -> String {
    format!("gatyaitemD_{:02}_f.png", id)
}

pub fn cat_folder(id: u32) -> PathBuf {
    Path::new(DIR_CATS).join(format!("{:03}", id))
}

pub fn cat_form_folder(id: u32, form: &str) -> PathBuf {
    cat_folder(id).join(form)
}

pub fn cat_form_img(id: u32, form: &str) -> String {
    format!("uni{:03}_{}00.png", id, form)
}

pub fn empty_cat_icon() -> String {
    "uni.png".to_string()
}

pub fn map_folder(root: &Path, cat: &Category, map_id: u32) -> PathBuf {
    let cat_dir = if root.ends_with("categories") {
        root.to_path_buf()
    } else {
        root.join("categories")
    };

    cat_dir.join(cat.map_prefix()).join(format!("{:03}", map_id))
}

pub fn stage_folder(root: &Path, cat: &Category, map_id: u32, stage_id: u32) -> PathBuf {
    map_folder(root, cat, map_id).join(format!("{:02}", stage_id))
}

pub fn stage_name_image(root: &Path, cat: &Category, map_id: u32, stage_id: u32, lang: &str) -> PathBuf {
    let norm = cat.map_prefix();
    let lang_sfx = if lang.is_empty() { String::new() } else { format!("_{}", lang) };

    let cat_dir = if root.ends_with("categories") {
        root.to_path_buf()
    } else {
        root.join("categories")
    };

    if matches!(norm.as_str(), "EC" | "W" | "Space" | "PT") || (norm == "M" && stage_id == 50 && map_id == 0) {
        let file_pfx = match norm.as_str() {
            "W" => "wc",
            "Space" => "sc",
            _ => "ec",
        };

        let mut img_id = stage_id;
        if norm == "M" && stage_id == 50 { img_id = 48; }
        if norm == "PT" { img_id = stage_id.saturating_sub(2); }

        return cat_dir.join(&norm)
            .join("000")
            .join(format!("{:02}", stage_id))
            .join(format!("{}{:03}_n{}.png", file_pfx, img_id, lang_sfx));
    }

    cat_dir.join(&norm)
        .join(format!("{:03}", map_id))
        .join(format!("{:02}", stage_id))
        .join(format!("mapsn{:03}_{:02}_{}{}.png", map_id, stage_id, cat.image_prefix(), lang_sfx))
}