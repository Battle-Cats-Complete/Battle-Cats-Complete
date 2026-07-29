pub mod filter;
pub mod game;
pub mod paths;
pub(crate) mod patterns;
pub mod scanner;
pub mod waiter;

use serde::{Deserialize, Serialize};

use crate::common::formats::Model;
use crate::modules::enemy::game::registry::Magnification;
use crate::modules::enemy::scanner::EnemyEntry;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy, Default, Debug)]
pub enum EnemyDetailTab {
    #[default]
    Abilities,
    Details,
    Animation,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct EnemyDataState {
    #[serde(skip)] pub enemies: Vec<EnemyEntry>,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
    pub selected_tab: EnemyDetailTab,
    pub mag_input: String,
    pub magnification: Magnification,
    #[serde(skip)] pub detail_key: String,
    #[serde(skip)] pub model_data: Option<Model>,
}

impl Default for EnemyDataState {
    fn default() -> Self {
        Self {
            enemies: Vec::new(),
            selected_enemy: None,
            search_query: String::new(),
            selected_tab: EnemyDetailTab::default(),
            mag_input: "100".to_string(),
            magnification: Magnification::default(),
            detail_key: String::new(),
            model_data: None,
        }
    }
}
