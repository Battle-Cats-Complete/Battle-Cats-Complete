pub mod filter;
pub mod game;
pub mod files;
pub(crate) mod patterns;
pub mod scanner;

use serde::{Deserialize, Serialize};

use crate::modules::enemy::scanner::EnemyEntry;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct EnemyDataState {
    #[serde(skip)] pub enemies: Vec<EnemyEntry>,
}
