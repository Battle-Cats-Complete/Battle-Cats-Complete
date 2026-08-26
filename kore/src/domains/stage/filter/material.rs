
use serde::{Deserialize, Serialize};


use crate::{ItemStore, Vfs};

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

    pub(crate) fn compile(&self) -> CompiledMaterialFilter {
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

pub(crate) struct CompiledMaterialFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub parsed_id: Option<u32>,
    pub amount: CompiledStatRange,
}

impl CompiledMaterialFilter {
    pub(crate) fn matches_material(
        &self,
        index: usize,
        drop_amount: u32,
        items: &ItemStore,
        vfs: &Vfs,
    ) -> bool {
        if !self.amount.matches(drop_amount as i64) { return false; }
        if self.name_or_id.is_empty() { return true; }

        let Some(&item_id) = MAT_IDS.get(index) else { return false; };
        if self.parsed_id == Some(item_id) { return true; }

        items.name(vfs, item_id).is_some_and(|name| name.to_lowercase().contains(&self.name_or_id))
    }
}