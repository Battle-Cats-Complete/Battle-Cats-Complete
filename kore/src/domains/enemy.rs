pub mod filter;
pub mod files;
pub(crate) mod patterns;
pub mod scanner;
pub mod statblock;

use serde::{Deserialize, Serialize};

use crate::domains::enemy::scanner::EnemyEntry;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct EnemyDataState {
    #[serde(skip)] pub enemies: Vec<EnemyEntry>,
}
