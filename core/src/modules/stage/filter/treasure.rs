use std::collections::HashMap;

use nyanko::cat::unit::UnitBuy;
use serde::{Deserialize, Serialize};

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;

use super::range::{CompiledStatRange, StatRange};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TreasureFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub amount: StatRange,
    pub chance: StatRange,
}

impl TreasureFilter {
    pub fn is_active(&self) -> bool {
        !self.name_or_id.trim().is_empty()
            || self.amount.is_active()
            || self.chance.is_active()
    }

    pub(crate) fn compile(&self) -> CompiledTreasureFilter {
        let name_or_id = self.name_or_id.trim().to_lowercase();
        let parsed_id = name_or_id.parse::<u32>().ok();

        CompiledTreasureFilter {
            is_exclude: self.is_exclude,
            name_or_id,
            parsed_id,
            amount: self.amount.compile(0),
            chance: self.chance.compile(0),
        }
    }
}

pub(crate) struct CompiledTreasureFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub parsed_id: Option<u32>,
    pub amount: CompiledStatRange,
    pub chance: CompiledStatRange,
}

impl CompiledTreasureFilter {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_drop(
        &self,
        target_id: u32,
        drop_amt: u32,
        drop_chnc: u32,
        buy_reg: &HashMap<u32, GatyaItemBuy>,
        name_reg: &HashMap<usize, GatyaItemName>,
        drop_chara_reg: &HashMap<u32, u32>,
        unit_buy_reg: &HashMap<u32, UnitBuy>,
        cat_name_reg: &HashMap<u32, Vec<String>>,
    ) -> bool {
        if !self.amount.matches(drop_amt as i64) { return false; }
        if !self.chance.matches(drop_chnc as i64) { return false; }

        if self.name_or_id.is_empty() { return true; }
        if self.parsed_id == Some(target_id) { return true; }

        if let Some(item_buy) = buy_reg.get(&target_id) {
            return name_reg.get(&{ item_buy.row_index })
                .is_some_and(|name| name.name.to_lowercase().contains(&self.name_or_id));
        }

        if let Some(&chara_id) = drop_chara_reg.get(&target_id) {
            return cat_name_reg.get(&chara_id)
                .is_some_and(|names| names.iter().any(|name| name.contains(&self.name_or_id)));
        }

        if let Some((&unit_id, _)) = unit_buy_reg.iter().find(|(_, row)| row.true_form_id == target_id as i32) {
            return cat_name_reg.get(&unit_id)
                .is_some_and(|names| names.iter().any(|name| name.contains(&self.name_or_id)));
        }

        false
    }
}