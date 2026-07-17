pub mod filter;
pub mod game;
pub mod scanner;
pub mod paths;
pub mod patterns;
pub mod waiter;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::common::formats::mamodel::Model;
use crate::modules::cat::scanner::CatEntry;
use crate::modules::settings::ScannerConfig;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy, Default)]
pub enum DetailTab {
    #[default]
    Abilities,
    Details,
    Talents,
    Animation,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct CatDataState {
    #[serde(skip)] pub cats: Vec<CatEntry>,
    #[serde(skip)] pub incoming_cats: Vec<CatEntry>,
    #[serde(skip)] pub is_cold_scan: bool,
    #[serde(skip)] pub last_update_time: Option<Instant>,
    pub selected_cat: Option<u32>,
    pub search_query: String,
    pub selected_form: usize,
    pub selected_detail_tab: DetailTab,
    pub level_input: String,
    pub current_level: i32,
    #[serde(skip)] pub initialized: bool,
    #[serde(skip)] pub active_scan_ids: HashSet<u32>,
    #[serde(skip)] pub detail_key: String,
    #[serde(skip)] pub model_data: Option<Model>,
    #[serde(skip)] pub scan_receiver: Option<Receiver<CatEntry>>,
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>,
    #[serde(skip)] pub saved_pre_ultra_level: Option<(i32, String)>,
    #[serde(skip)] pub is_in_ultra_state: bool,
}

impl Default for CatDataState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            incoming_cats: Vec::new(),
            is_cold_scan: false,
            last_update_time: None,
            selected_cat: None,
            search_query: String::new(),
            selected_form: 0,
            selected_detail_tab: DetailTab::default(),
            level_input: "50".to_string(),
            current_level: 50,
            initialized: false,
            active_scan_ids: HashSet::new(),
            detail_key: String::new(),
            model_data: None,
            scan_receiver: None,
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            saved_pre_ultra_level: None,
            is_in_ultra_state: false,
        }
    }
}

impl CatDataState {
    pub fn update_data(&mut self) {
        scanner::update_data(self);
    }

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        scanner::restart_scan(self, config);
    }
}