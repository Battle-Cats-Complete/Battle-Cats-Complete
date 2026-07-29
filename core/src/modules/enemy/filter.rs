pub mod evaluation;

use std::collections::{HashMap, HashSet};

use nyanko::enemy::abilities::Identity;

pub const ATTACK_TYPE_IDENTITIES: &[Identity] = &[
    Identity::SingleAttack,
    Identity::AreaAttack,
    Identity::OmniStrike,
    Identity::LongDistance,
    Identity::MultiHit,
];

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum MatchMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, PartialEq, Default)]
pub struct RangeInput {
    pub min: String,
    pub max: String,
}

#[derive(Clone, PartialEq)]
pub struct EnemyFilterState {
    pub is_open: bool,
    pub active_identities: HashSet<Identity>,
    pub match_mode: MatchMode,
    pub adv_ranges: HashMap<Identity, HashMap<&'static str, RangeInput>>,
    pub mag_input: String,
    pub stat_ranges: HashMap<&'static str, RangeInput>,
}

impl Default for EnemyFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_identities: HashSet::new(),
            match_mode: MatchMode::And,
            adv_ranges: HashMap::new(),
            mag_input: String::new(),
            stat_ranges: HashMap::new(),
        }
    }
}

impl EnemyFilterState {
    pub fn is_active(&self) -> bool {
        !self.active_identities.is_empty()
            || self.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty())
    }
}