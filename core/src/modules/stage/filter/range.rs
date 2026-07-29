use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StatRange {
    pub min: String,
    pub max: String,
}

impl StatRange {
    pub fn is_active(&self) -> bool {
        !self.min.trim().is_empty() || !self.max.trim().is_empty()
    }

    pub(crate) fn compile(&self, offset: i64) -> CompiledStatRange {
        let min_val = if self.min.trim().is_empty() {
            i64::MIN
        } else {
            self.min.trim().parse::<i64>().map(|val| val + offset).unwrap_or_else(|_| {
                trace!("Failed to parse min filter value: {}", self.min);
                i64::MIN
            })
        };

        let max_val = if self.max.trim().is_empty() {
            i64::MAX
        } else {
            self.max.trim().parse::<i64>().map(|val| val + offset).unwrap_or_else(|_| {
                trace!("Failed to parse max filter value: {}", self.max);
                i64::MAX
            })
        };

        CompiledStatRange {
            min: min_val,
            max: max_val,
            active: self.is_active(),
        }
    }
}

pub(crate) struct CompiledStatRange {
    pub min: i64,
    pub max: i64,
    pub active: bool,
}

impl CompiledStatRange {
    pub(crate) fn matches(&self, target_val: i64) -> bool {
        if !self.active { return true; }
        target_val >= self.min && target_val <= self.max
    }
}