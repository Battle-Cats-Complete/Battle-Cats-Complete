use std::path::{Path, PathBuf};
use regex::Regex;
use crate::features::enemy::patterns;

pub struct EnemyMatcher {
    stats: Regex,
    icon: Regex,
    anim_base: Regex,
    maanim: Regex,
}

impl EnemyMatcher {
    pub fn new() -> Self {
        Self {
            stats: Regex::new(patterns::ENEMY_STATS).unwrap(),
            icon: Regex::new(patterns::ENEMY_ICON).unwrap(),
            anim_base: Regex::new(patterns::ENEMY_ANIM_BASE).unwrap(),
            maanim: Regex::new(patterns::ENEMY_MAANIM).unwrap(),
        }
    }

    pub fn get_dest(&self, name: &str, enemy_dir: &Path) -> Option<PathBuf> {
        // game/enemies/t_unit.csv
        if self.stats.is_match(name) {
            return Some(enemy_dir.to_path_buf());
        }
        
        // game/enemies/{id}/enemy_icon_{id}.png
        if let Some(caps) = self.icon.captures(name) {
            return Some(enemy_dir.join(&caps[1]));
        }
        
        // game/enemies/{id}/anim/{id}_e.(imgcut|mamodel|png)
        if let Some(caps) = self.anim_base.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }
        
        // game/enemies/{id}/anim/{id}_e...maanim
        if let Some(caps) = self.maanim.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }
        
        None
    }
}