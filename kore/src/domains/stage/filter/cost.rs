use nyanko::chapter::Stage;
use nyanko::chapter::stage::{CataminGrade, CostType};
use serde::{Deserialize, Serialize};

use super::range::{CompiledStatRange, StatRange};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct CostFilter {
    pub cost_type: Option<CostType>,
    pub energy: StatRange,
    pub item_cost: StatRange,
    pub item_id: StatRange,
    pub catamin_cost: StatRange,
    pub catamin_grade: Option<CataminGrade>,
}

impl CostFilter {
    pub fn is_active(&self) -> bool {
        self.cost_type.is_some()
    }

    pub(crate) fn compile(&self) -> CompiledCostFilter {
        let amount = match self.cost_type {
            Some(CostType::Item) => &self.item_cost,
            Some(CostType::Catamin) => &self.catamin_cost,
            _ => &self.energy,
        };

        CompiledCostFilter {
            active: self.is_active(),
            cost_type: self.cost_type,
            amount: amount.compile(0),
            item_id: self.item_id.compile(0),
            catamin_grade: self.catamin_grade,
        }
    }
}

pub(crate) struct CompiledCostFilter {
    pub active: bool,
    cost_type: Option<CostType>,
    amount: CompiledStatRange,
    item_id: CompiledStatRange,
    catamin_grade: Option<CataminGrade>,
}

impl CompiledCostFilter {
    pub(crate) fn matches(&self, stage: &Stage) -> bool {
        let Some(wanted_type) = self.cost_type else { return true; };

        if stage.cost_type() != wanted_type { return false; }

        let resolved = stage.resolved_cost();

        if !self.amount.matches(i64::from(resolved.value)) { return false; }

        match wanted_type {
            CostType::Item => self.item_id.matches(resolved.id.map_or(-1, i64::from)),
            CostType::Catamin => self.catamin_grade
                .is_none_or(|wanted| resolved.id.map(CataminGrade::from_key) == Some(wanted)),
            _ => true,
        }
    }
}
