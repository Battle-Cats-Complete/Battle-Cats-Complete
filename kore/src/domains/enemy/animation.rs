use crate::systems::animation::{self, Clip};
use crate::Vfs;

use super::files;
use super::scanner::EnemyEntry;

const ZOMBIE_MARK: &str = "_zombie";

pub fn set_id(enemy: &EnemyEntry) -> String {
    enemy.id_str()
}

pub fn clips(enemy: &EnemyEntry, vfs: &Vfs) -> Vec<Clip> {
    let plain = files::anim_base_filename(enemy.id);
    let bases = vec![plain.clone(), format!("i{}", plain)];

    let Some(rig) = animation::rigging(vfs, &plain, &bases) else {
        return Vec::new();
    };

    let mut clips = Vec::new();

    for (suffix, path) in animation::maanims(vfs, &bases) {
        let zombie = suffix.strip_prefix(ZOMBIE_MARK);
        let index = zombie.unwrap_or(&suffix).parse::<usize>().ok();

        let named = if zombie.is_some() { None } else { index.and_then(animation::standard) };

        clips.push(Clip {
            name: named
                .map(|(name, _, _)| name.to_string())
                .or_else(|| zombie.and(index).and_then(zombie_name).map(str::to_string)),
            slot: named.map(|(_, slot, _)| slot),
            role: named.map(|(_, _, role)| role),
            loops: named.is_some_and(|(_, _, role)| role.loops()),
            rig: rig.clone(),
            anim: Some(path),
        });
    }

    clips.push(Clip::model(rig));
    clips
}

fn zombie_name(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("Burrow"),
        1 => Some("Dig"),
        2 => Some("Surface"),
        _ => None,
    }
}
