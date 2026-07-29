use std::path::{Path, PathBuf};

pub const DIR_ENEMIES: &str = "game/enemies";

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

pub fn stats(root: &Path) -> PathBuf {
    root.join("t_unit.csv")
}

pub fn icon(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id))
        .join(format!("enemy_icon_{:03}.png", id))
}

pub(crate) fn anim_folder(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id)).join("anim")
}

pub(crate) fn anim_base_filename(id: u32) -> String {
    format!("{:03}_e", id)
}

pub fn anim(root: &Path, id: u32, file_type: AnimType) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    let ext = file_type.ext();
    folder.join(format!("{}.{}", filename, ext))
}

pub fn maanim(root: &Path, id: u32, index: usize) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    folder.join(format!("{}{:02}.maanim", filename, index))
}

pub fn zombie_maanim(root: &Path, id: u32, index: usize) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    folder.join(format!("{}_zombie{:02}.maanim", filename, index))
}
