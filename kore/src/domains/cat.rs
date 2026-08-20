pub mod filter;
pub mod game;
pub mod scanner;
pub mod statblock;
pub mod files;
pub(crate) mod patterns;
pub mod waiter;

use serde::{Deserialize, Serialize};

use crate::domains::cat::scanner::CatEntry;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CatDataState {
    #[serde(skip)] pub cats: Vec<CatEntry>,
}