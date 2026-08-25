pub const STATS: &str = "t_unit.csv";
pub const NAMES: &str = "Enemyname.tsv";
pub const PICTURE_BOOK: &str = "EnemyPictureBook.csv";

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

pub fn anim_base_filename(id: u32) -> String {
    format!("{:03}_e", id)
}

pub(crate) fn icon_file(id: u32) -> String {
    format!("enemy_icon_{:03}.png", id)
}

pub fn anim_file(id: u32, file_type: AnimType) -> String {
    format!("{}.{}", anim_base_filename(id), file_type.ext())
}

pub fn maanim_file(id: u32, index: usize) -> String {
    format!("{}{:02}.maanim", anim_base_filename(id), index)
}

pub fn zombie_maanim_file(id: u32, index: usize) -> String {
    format!("{}_zombie{:02}.maanim", anim_base_filename(id), index)
}
