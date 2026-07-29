use std::path::{Path, PathBuf};

pub(crate) const UNIT_BUY: &str = "unitbuy.csv";
pub(crate) const UNIT_LEVEL: &str = "unitlevel.csv";
pub(crate) const SKILL_ACQUISITION: &str = "SkillAcquisition.csv";
pub(crate) const SKILL_LEVEL: &str = "SkillLevel.csv";

pub const DIR_CATS: &str = "game/cats";
const DIR_ANIM: &str = "anim";
const DIR_LANG: &str = "lang";
pub(crate) const DIR_UNIT_EVOLVE: &str = "unitevolve";
pub const DIR_SKILL_NAME: &str = "game/cats/Skill_name";
pub(crate) const DIR_SKILL_DESCRIPTIONS: &str = "SkillDescriptions";

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum AssetType {
    Icon,
    Banner,
}

#[derive(Copy, Clone, PartialEq)]
pub enum AnimType {
    Mamodel,
    Imgcut,
    Png,
}

impl AnimType {
    pub fn ext(&self) -> &str {
        match self {
            AnimType::Mamodel => "mamodel",
            AnimType::Imgcut => "imgcut",
            AnimType::Png => "png",
        }
    }
}

pub(crate) fn anim_folder(root: &Path, id: u32, form: usize, egg_ids: (i32, i32)) -> PathBuf {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        root.join(format!("egg_{:03}", egg_norm)).join(DIR_ANIM)
    } else if form == 1 && egg_evol != -1 {
        root.join(format!("egg_{:03}", egg_evol)).join(DIR_ANIM)
    } else {
        root.join(format!("{:03}", id)).join(form_char).join(DIR_ANIM)
    }
}

pub(crate) fn anim_base_filename(id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        format!("{:03}_m", egg_norm)
    } else if form == 1 && egg_evol != -1 {
        format!("{:03}_m", egg_evol)
    } else {
        format!("{:03}_{}", id, form_char)
    }
}

pub fn folder(root: &Path, id: u32, form: usize, egg_ids: (i32, i32)) -> PathBuf {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        return root.join(format!("egg_{:03}", egg_norm)).join(form_char);
    }
    if form == 1 && egg_evol != -1 {
        return root.join(format!("egg_{:03}", egg_evol)).join(form_char);
    }
    root.join(format!("{:03}", id)).join(form_char)
}

pub(crate) fn image_stem(asset_type: AssetType, id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    let (egg_norm, egg_evol) = egg_ids;
    let prefix = match asset_type { AssetType::Icon => "uni", AssetType::Banner => "udi" };
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        return format!("{}{:03}_m00", prefix, egg_norm);
    }
    if form == 1 && egg_evol != -1 {
        return format!("{}{:03}_m01", prefix, egg_evol);
    }
    match asset_type {
        AssetType::Icon => format!("{}{:03}_{}00", prefix, id, form_char),
        AssetType::Banner => format!("{}{:03}_{}", prefix, id, form_char),
    }
}
pub fn anim(root: &Path, id: u32, form: usize, egg_ids: (i32, i32), file_type: AnimType) -> PathBuf {
    let folder = anim_folder(root, id, form, egg_ids);
    let filename = anim_base_filename(id, form, egg_ids);
    let ext = file_type.ext();
    folder.join(format!("{}.{}", filename, ext))
}

pub fn maanim(root: &Path, id: u32, form: usize, egg_ids: (i32, i32), index: usize) -> PathBuf {
    let folder = anim_folder(root, id, form, egg_ids);
    let filename = anim_base_filename(id, form, egg_ids);
    folder.join(format!("{}{:02}.maanim", filename, index))
}

pub fn stats(root: &Path, id: u32) -> PathBuf {
    let file_id = id + 1;
    root.join(format!("{:03}", id))
        .join(format!("unit{:03}.csv", file_id))
}

pub fn lang(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id)).join(DIR_LANG)
}

pub fn explanation(root: &Path, id: u32) -> PathBuf {
    let file_id = id + 1;
    lang(root, id).join(format!("Unit_Explanation{}.csv", file_id))
}
