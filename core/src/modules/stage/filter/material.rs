use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;

use super::super::materials::MAT_IDS;
use super::range::{CompiledStatRange, StatRange};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct MaterialFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub amount: StatRange,
}

impl MaterialFilter {
    pub fn is_active(&self) -> bool {
        !self.name_or_id.trim().is_empty() || self.amount.is_active()
    }

    pub fn compile(&self) -> CompiledMaterialFilter {
        let name_or_id = self.name_or_id.trim().to_lowercase();
        let parsed_id = name_or_id.parse::<u32>().ok();

        CompiledMaterialFilter {
            is_exclude: self.is_exclude,
            name_or_id,
            parsed_id,
            amount: self.amount.compile(0),
        }
    }
}

pub struct CompiledMaterialFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub parsed_id: Option<u32>,
    pub amount: CompiledStatRange,
}

impl CompiledMaterialFilter {
    pub fn matches_material(
        &self,
        index: usize,
        drop_amount: u32,
        buy_reg: &HashMap<u32, GatyaItemBuy>,
        name_reg: &HashMap<usize, GatyaItemName>,
    ) -> bool {
        if !self.amount.matches(drop_amount as i64) { return false; }
        if self.name_or_id.is_empty() { return true; }

        let Some(&item_id) = MAT_IDS.get(index) else { return false; };
        if self.parsed_id == Some(item_id) { return true; }

        let Some(item_buy) = buy_reg.get(&item_id) else { return false; };
        let Some(name_entry) = name_reg.get(&{ item_buy.row_index }) else { return false; };

        name_entry.name.to_lowercase().contains(&self.name_or_id)
    }
}