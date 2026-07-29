pub mod filter;
pub mod game;
pub mod scanner;
pub mod paths;
pub(crate) mod patterns;
pub mod waiter;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::common::formats::Model;
use crate::modules::cat::scanner::CatEntry;

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct CatDataState {
    #[serde(skip)] pub cats: Vec<CatEntry>,
    pub selected_cat: Option<u32>,
    pub search_query: String,
    pub selected_form: usize,
    pub level_input: String,
    pub current_level: i32,
    #[serde(skip)] pub detail_key: String,
    #[serde(skip)] pub model_data: Option<Model>,
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    #[serde(skip)] pub saved_pre_ultra_level: Option<(i32, String)>,
    #[serde(skip)] pub is_in_ultra_state: bool,
}

impl Default for CatDataState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            selected_cat: None,
            search_query: String::new(),
            selected_form: 0,
            level_input: "50".to_string(),
            current_level: 50,
            detail_key: String::new(),
            model_data: None,
            talent_levels: HashMap::new(),
            saved_pre_ultra_level: None,
            is_in_ultra_state: false,
        }
    }
}