use std::collections::HashMap;

use nyanko::chapter::Stage;
use serde::{Deserialize, Serialize};

use super::range::{CompiledStatRange, StatRange};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct LineupFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub level: StatRange,
}

impl LineupFilter {
    pub fn is_active(&self) -> bool {
        true
    }

    pub(crate) fn compile(&self) -> CompiledLineupFilter {
        let name_or_id = self.name_or_id.trim().to_lowercase();
        let parsed_id = name_or_id.parse::<u32>().ok();

        CompiledLineupFilter {
            is_exclude: self.is_exclude,
            name_or_id,
            parsed_id,
            level: self.level.compile(0),
        }
    }
}

pub(crate) struct CompiledLineupFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub parsed_id: Option<u32>,
    pub level: CompiledStatRange,
}

impl CompiledLineupFilter {
    pub(crate) fn matches_lineup(
        &self,
        stage: &Stage,
        cat_name_reg: &HashMap<u32, Vec<String>>
    ) -> bool {
        for preset in stage.fixed_lineups.values() {
            for &unit_id in &preset.slot_units {
                let Some(chara) = preset.characters.get(&unit_id) else {
                    continue;
                };

                let total_level = chara.level as i64 + chara.plus_level as i64;
                if !self.level.matches(total_level) {
                    continue;
                }

                if self.name_or_id.is_empty() {
                    return !self.is_exclude;
                }

                if self.parsed_id == Some(unit_id) {
                    return !self.is_exclude;
                }

                let name_match = cat_name_reg.get(&unit_id).is_some_and(|names| {
                    names.iter().any(|name| name.contains(&self.name_or_id))
                });

                if name_match {
                    return !self.is_exclude;
                }
            }
        }

        self.is_exclude
    }
}