use std::path::{Path, PathBuf};
use nyanko::chapter::Category;

pub(crate) const DIR_STAGES: &str = "game/stages";
pub const DIR_CATEGORIES: &str = "game/stages/categories";
pub const DIR_CATS: &str = "game/cats";
pub const DIR_ENEMIES: &str = "game/enemies";
pub(crate) const DIR_TABLES: &str = "game/tables";
pub(crate) const DIR_GATYA_ITEM: &str = "game/ui/gatyaitemD";

pub(crate) fn stage_sub(sub: &str) -> PathBuf {
    Path::new(DIR_STAGES).join(sub)
}

pub(crate) fn gatya_name_dir() -> PathBuf {
    Path::new(DIR_TABLES).join("GatyaitemName")
}

pub(crate) fn gatya_item_img(id: u32) -> String {
    format!("gatyaitemD_{:02}_f.png", id)
}

pub fn cat_folder(id: u32) -> PathBuf {
    Path::new(DIR_CATS).join(format!("{:03}", id))
}

pub(crate) fn cat_form_folder(id: u32, form: &str) -> PathBuf {
    cat_folder(id).join(form)
}

pub(crate) fn cat_form_img(id: u32, form: &str) -> String {
    format!("uni{:03}_{}00.png", id, form)
}

pub(crate) fn empty_cat_icon() -> String {
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
pub(crate) fn stage_name_targets(cat_path: &Path, cat_prefix: &str) -> Vec<(PathBuf, String)> {
    let mut targets = Vec::new();
    let base_name_dir = cat_path.join("StageName");

    if base_name_dir.exists() {
        targets.push((base_name_dir.clone(), format!("StageName_{}.csv", cat_prefix)));
        targets.push((base_name_dir.clone(), format!("StageName_R{}.csv", cat_prefix)));
    }

    if cat_prefix == "EC" {
        targets.push((cat_path.join("StageName"), "StageName.csv".to_string()));
    }

    let num_target = match cat_prefix {
        "EC" => Some((cat_path.join("StageName0"), "StageName0.csv".to_string())),
        "W" => Some((cat_path.join("StageName1"), "StageName1.csv".to_string())),
        "Space" => Some((cat_path.join("StageName2"), "StageName2.csv".to_string())),
        _ => None,
    };

    if let Some(target) = num_target {
        targets.push(target);
    }

    targets
}