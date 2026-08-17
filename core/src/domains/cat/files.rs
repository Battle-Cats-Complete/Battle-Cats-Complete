pub(crate) const UNIT_BUY: &str = "unitbuy.csv";
pub(crate) const UNIT_LEVEL: &str = "unitlevel.csv";
pub(crate) const SKILL_ACQUISITION: &str = "SkillAcquisition.csv";
pub(crate) const SKILL_LEVEL: &str = "SkillLevel.csv";
pub(crate) const SKILL_DESCRIPTIONS: &str = "SkillDescriptions.csv";
pub(crate) const UNIT_EVOLVE: &str = "unitevolve.csv";

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

fn anim_base_filename(id: u32, form: usize, egg_ids: (i32, i32)) -> String {
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

pub fn anim_file(id: u32, form: usize, egg_ids: (i32, i32), file_type: AnimType) -> String {
    format!("{}.{}", anim_base_filename(id, form, egg_ids), file_type.ext())
}

pub fn maanim_file(id: u32, form: usize, egg_ids: (i32, i32), index: usize) -> String {
    format!("{}{:02}.maanim", anim_base_filename(id, form, egg_ids), index)
}

pub fn explanation_file(id: u32) -> String {
    format!("Unit_Explanation{}.csv", id + 1)
}

pub fn stats_file(id: u32) -> String {
    format!("unit{:03}.csv", id + 1)
}
