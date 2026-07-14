use serde::{Deserialize, Serialize};
use tracing::trace;

use nyanko::chapter::Stage;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatRange {
    pub min: String,
    pub max: String,
}

impl StatRange {
    pub fn is_active(&self) -> bool {
        !self.min.trim().is_empty() || !self.max.trim().is_empty()
    }

    pub fn matches(&self, target_val: u32) -> bool {
        if !self.is_active() {
            return true;
        }

        let min_val = if self.min.trim().is_empty() {
            0
        } else {
            match self.min.trim().parse::<u32>() {
                Ok(v) => v,
                Err(_) => {
                    trace!("Failed to parse min filter value: {}", self.min);
                    0
                }
            }
        };

        let max_val = if self.max.trim().is_empty() {
            u32::MAX
        } else {
            match self.max.trim().parse::<u32>() {
                Ok(v) => v,
                Err(_) => {
                    trace!("Failed to parse max filter value: {}", self.max);
                    u32::MAX
                }
            }
        };

        target_val >= min_val && target_val <= max_val
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageFilterState {
    pub is_open: bool,
    pub no_continues: Option<bool>,
    pub indestructible_base: Option<bool>,

    pub width: StatRange,
    pub base_hp: StatRange,
    pub max_enemies: StatRange,
    pub time_limit: StatRange,
    pub energy: StatRange,
    pub xp: StatRange,
    pub min_cost: StatRange,
    pub max_cost: StatRange,
}

impl StageFilterState {
    pub fn is_active(&self) -> bool {
        self.no_continues.is_some()
            || self.indestructible_base.is_some()
            || self.width.is_active()
            || self.base_hp.is_active()
            || self.max_enemies.is_active()
            || self.time_limit.is_active()
            || self.energy.is_active()
            || self.xp.is_active()
            || self.min_cost.is_active()
            || self.max_cost.is_active()
    }

    pub fn matches_stage(&self, stage: &Stage) -> bool {
        if !self.is_active() {
            return true;
        }

        if let Some(nc) = self.no_continues {
            if stage.is_no_continues != nc {
                return false;
            }
        }

        if let Some(ib) = self.indestructible_base {
            if stage.is_base_indestructible != ib {
                return false;
            }
        }

        if !self.width.matches(stage.width) {
            return false;
        }
        if !self.base_hp.matches(stage.base_hp) {
            return false;
        }
        if !self.max_enemies.matches(stage.max_enemies) {
            return false;
        }
        if !self.time_limit.matches(stage.time_limit) {
            return false;
        }
        if !self.energy.matches(stage.energy) {
            return false;
        }
        if !self.xp.matches(stage.xp) {
            return false;
        }
        if !self.min_cost.matches(stage.min_cost) {
            return false;
        }
        if !self.max_cost.matches(stage.max_cost) {
            return false;
        }

        true
    }
}