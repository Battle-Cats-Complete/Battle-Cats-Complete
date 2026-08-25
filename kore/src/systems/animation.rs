pub mod export;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nyanko::graphics::rig::Animation;
use nyanko::graphics::tools::math;

use crate::Vfs;

const LOOP_CEILING: i32 = 999_999;

const MAANIM_EXT: &str = ".maanim";

const SLOT_WALK: usize = 0;
const SLOT_IDLE: usize = 1;
const SLOT_ATTACK: usize = 2;
const SLOT_KNOCKBACK: usize = 3;
pub const SLOT_MODEL: usize = 7;

const MODEL_NAME: &str = "Model";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    Walk,
    Idle,
    Attack,
    Knockback,
}

impl Role {
    pub(crate) fn loops(self) -> bool {
        matches!(self, Role::Walk | Role::Idle)
    }
}

pub struct Rigging {
    pub id: String,
    pub png: PathBuf,
    pub cut: PathBuf,
    pub model: PathBuf,
}

pub struct Clip {
    pub name: Option<String>,
    pub slot: Option<usize>,
    pub role: Option<Role>,
    pub loops: bool,
    pub rig: Arc<Rigging>,
    pub anim: Option<PathBuf>,
}

impl Clip {
    pub(crate) fn model(rig: Arc<Rigging>) -> Self {
        Self {
            name: Some(MODEL_NAME.to_string()),
            slot: Some(SLOT_MODEL),
            role: None,
            loops: false,
            rig,
            anim: None,
        }
    }

    pub fn label(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }

        self.anim
            .as_deref()
            .and_then(Path::file_stem)
            .and_then(OsStr::to_str)
            .map_or_else(|| self.rig.id.clone(), str::to_string)
    }

    pub fn slug(&self) -> String {
        let Some(name) = &self.name else {
            return self.label();
        };

        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect()
    }
}

pub(crate) fn standard(index: usize) -> Option<(&'static str, usize, Role)> {
    match index {
        0 => Some(("Walk", SLOT_WALK, Role::Walk)),
        1 => Some(("Idle", SLOT_IDLE, Role::Idle)),
        2 => Some(("Attack", SLOT_ATTACK, Role::Attack)),
        3 => Some(("Knockback", SLOT_KNOCKBACK, Role::Knockback)),
        _ => None,
    }
}

pub(crate) fn rigging(vfs: &Vfs, id: &str, bases: &[String]) -> Option<Arc<Rigging>> {
    let resolve = |ext: &str| -> Option<PathBuf> {
        let names: Vec<String> = bases.iter().map(|base| format!("{}.{}", base, ext)).collect();
        vfs.find(names.as_slice())
    };

    Some(Arc::new(Rigging {
        id: id.to_string(),
        png: resolve("png")?,
        cut: resolve("imgcut")?,
        model: resolve("mamodel")?,
    }))
}

fn owns(suffix: &str) -> bool {
    suffix.starts_with('_') || suffix.bytes().all(|byte| byte.is_ascii_digit())
}

pub(crate) fn maanims(vfs: &Vfs, bases: &[String]) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();

    for base in bases {
        for name in vfs.glob(base) {
            let canonical = vfs.stripped(&name).unwrap_or_else(|| name.to_string());

            let Some(suffix) = canonical.strip_prefix(base.as_str()).and_then(|rest| rest.strip_suffix(MAANIM_EXT))
            else {
                continue;
            };

            if !owns(suffix) {
                continue;
            }

            if found.iter().any(|(existing, _)| existing == suffix) {
                continue;
            }

            if let Some(path) = vfs.find(&canonical) {
                found.push((suffix.to_string(), path));
            }
        }
    }

    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

pub fn loop_frame(animation: &Animation, frame: f32) -> f32 {
    let boundary = true_loop(animation).unwrap_or_else(|| furthest_frame(animation));

    if boundary > 0 { frame.rem_euclid(boundary as f32 + 1.0) } else { frame }
}

pub fn furthest_frame(animation: &Animation) -> i32 {
    animation.modifications.iter()
        .filter_map(|modification| modification.keyframes.last())
        .fold(0, |furthest, keyframe| furthest.max(keyframe.frame))
}

pub fn true_loop(animation: &Animation) -> Option<i32> {
    let furthest = furthest_frame(animation);
    let mut combined: i32 = 1;
    let mut looping = false;

    for modification in &animation.modifications {
        if modification.loop_count == 1 { return None; }

        let (Some(first), Some(last)) = (modification.keyframes.first(), modification.keyframes.last()) else {
            continue;
        };

        let span = last.frame - first.frame;
        if span <= 0 { continue; }

        combined = (combined / math::gcd(combined, span)).checked_mul(span)?;
        if combined > LOOP_CEILING { return None; }

        looping = true;
    }

    if !looping { return Some(furthest); }

    (combined >= furthest).then_some(combined)
}

#[inline(always)]
pub fn multiply_mat3(matrix_a: &[f32; 9], matrix_b: &[f32; 9]) -> [f32; 9] {
    [
        matrix_a[0]*matrix_b[0] + matrix_a[3]*matrix_b[1] + matrix_a[6]*matrix_b[2],
        matrix_a[1]*matrix_b[0] + matrix_a[4]*matrix_b[1] + matrix_a[7]*matrix_b[2],
        matrix_a[2]*matrix_b[0] + matrix_a[5]*matrix_b[1] + matrix_a[8]*matrix_b[2],

        matrix_a[0]*matrix_b[3] + matrix_a[3]*matrix_b[4] + matrix_a[6]*matrix_b[5],
        matrix_a[1]*matrix_b[3] + matrix_a[4]*matrix_b[4] + matrix_a[7]*matrix_b[5],
        matrix_a[2]*matrix_b[3] + matrix_a[5]*matrix_b[4] + matrix_a[8]*matrix_b[5],

        matrix_a[0]*matrix_b[6] + matrix_a[3]*matrix_b[7] + matrix_a[6]*matrix_b[8],
        matrix_a[1]*matrix_b[6] + matrix_a[4]*matrix_b[7] + matrix_a[7]*matrix_b[8],
        matrix_a[2]*matrix_b[6] + matrix_a[5]*matrix_b[7] + matrix_a[8]*matrix_b[8],
    ]
}



#[cfg(test)]
mod tests {
    use super::owns;

    #[test]
    fn a_longer_name_sharing_the_prefix_is_not_ours() {
        // 008_charaawa.maanim starts with 008_c but belongs to a different unit.
        assert!(!owns("haraawa"));
        assert!(!owns("haraawa_zombie00"));
    }

    #[test]
    fn numbered_and_underscored_suffixes_are_ours() {
        assert!(owns("00"));
        assert!(owns("14"));
        assert!(owns("_zombie02"));
        assert!(owns(""));
    }
}
