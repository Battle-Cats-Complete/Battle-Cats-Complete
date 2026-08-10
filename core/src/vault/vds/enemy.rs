use std::sync::Arc;

use nyanko::enemy::unit::{Battle, EnemyName, EnemyPictureBook};
use serde::{Deserialize, Serialize};

use crate::Vfs;

use super::Slot;

const ENEMY_STATS: &str = "t_unit.csv";
const ENEMY_NAME: &str = "Enemyname.tsv";
const ENEMY_PICTURE_BOOK: &str = "EnemyPictureBook.csv";

#[derive(Default, Serialize, Deserialize)]
pub struct EnemyStore {
    stats: Slot<Vec<Battle>>,
    names: Slot<Vec<String>>,
    descriptions: Slot<Vec<Vec<String>>>,
}

impl Clone for EnemyStore {
    fn clone(&self) -> Self {
        Self {
            stats: super::snapshot(&self.stats),
            names: super::snapshot(&self.names),
            descriptions: super::snapshot(&self.descriptions),
        }
    }
}

impl EnemyStore {
    pub fn stats(&self, vfs: &Vfs) -> Arc<Vec<Battle>> {
        super::cached(&self.stats, || {
            super::parsed(vfs, ENEMY_STATS, Battle::parse_all).unwrap_or_default()
        })
    }

    pub fn names(&self, vfs: &Vfs) -> Arc<Vec<String>> {
        super::cached(&self.names, || {
            let mut merged: Vec<String> = Vec::new();

            for bytes in super::layered(vfs, ENEMY_NAME) {
                let Ok(parsed) = EnemyName::parse_all(bytes) else {
                    continue;
                };

                for (index, enemy) in parsed.into_iter().enumerate() {
                    if index >= merged.len() {
                        merged.push(enemy.name.unwrap_or_default());
                        continue;
                    }

                    if merged[index].is_empty()
                        && let Some(name) = enemy.name
                    {
                        merged[index] = name;
                    }
                }
            }

            merged
        })
    }

    pub fn descriptions(&self, vfs: &Vfs) -> Arc<Vec<Vec<String>>> {
        super::cached(&self.descriptions, || {
            let mut merged: Vec<Vec<String>> = Vec::new();

            for bytes in super::layered(vfs, ENEMY_PICTURE_BOOK) {
                let Ok(parsed) = EnemyPictureBook::parse_all(bytes) else {
                    continue;
                };

                for (index, enemy) in parsed.into_iter().enumerate() {
                    if index >= merged.len() {
                        merged.push(enemy.description.unwrap_or_default());
                        continue;
                    }

                    if merged[index].is_empty()
                        && let Some(description) = enemy.description
                    {
                        merged[index] = description;
                    }
                }
            }

            merged
        })
    }

    pub(super) fn evict(&self, filename: &str) {
        match filename {
            ENEMY_STATS => super::reset(&self.stats),
            ENEMY_NAME => super::reset(&self.names),
            ENEMY_PICTURE_BOOK => super::reset(&self.descriptions),
            _ => (),
        }
    }

    pub(super) fn clear(&self) {
        super::reset(&self.stats);
        super::reset(&self.names);
        super::reset(&self.descriptions);
    }
}
