pub mod filter;
pub mod game;
pub mod paths;
pub mod patterns;
pub mod scanner;
pub mod waiter;

use std::collections::HashSet;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::common::formats::Model;
use crate::modules::enemy::game::registry::Magnification;
use crate::modules::enemy::scanner::EnemyEntry;
use crate::modules::settings::ScannerConfig;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy, Default)]
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
    #[serde(skip)] pub incoming_enemies: Vec<EnemyEntry>,
    #[serde(skip)] pub is_cold_scan: bool,
    #[serde(skip)] pub last_update_time: Option<Instant>,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
    pub selected_tab: EnemyDetailTab,
    pub mag_input: String,
    pub magnification: Magnification,
    #[serde(skip)] pub initialized: bool,
    #[serde(skip)] pub active_scan_ids: HashSet<u32>,
    #[serde(skip)] pub detail_key: String,
    #[serde(skip)] pub model_data: Option<Model>,
    #[serde(skip)] pub scan_receiver: Option<Receiver<EnemyEntry>>,
}

impl Default for EnemyDataState {
    fn default() -> Self {
        Self {
            enemies: Vec::new(),
            incoming_enemies: Vec::new(),
            is_cold_scan: false,
            last_update_time: None,
            selected_enemy: None,
            search_query: String::new(),
            selected_tab: EnemyDetailTab::default(),
            mag_input: "100".to_string(),
            magnification: Magnification::default(),
            initialized: false,
            active_scan_ids: HashSet::new(),
            detail_key: String::new(),
            model_data: None,
            scan_receiver: None,
        }
    }
}

impl EnemyDataState {
    pub fn update_data(&mut self) {
        scanner::update_data(self);
    }

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        scanner::restart_scan(self, config);
    }
}