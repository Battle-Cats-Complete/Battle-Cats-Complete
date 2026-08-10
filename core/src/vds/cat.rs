use std::collections::HashMap;
use std::sync::Arc;

use nyanko::cat::unit::{
    LevelCurve, SkillDescriptions, Talent, TalentCost, UnitBuy, UnitEvolve,
};
use serde::{Deserialize, Serialize};

use crate::modules::cat::paths;
use crate::Vfs;

use super::Slot;

const SKILL_DESCRIPTIONS: &str = "SkillDescriptions.csv";
const UNIT_EVOLVE: &str = "unitevolve.csv";

#[derive(Default, Serialize, Deserialize)]
pub struct CatStore {
    talents: Slot<HashMap<u16, Talent>>,
    talent_costs: Slot<HashMap<u8, TalentCost>>,
    descriptions: Slot<Vec<String>>,
    unitbuy: Slot<HashMap<u32, UnitBuy>>,
    evolve: Slot<HashMap<u32, UnitEvolve>>,
    curves: Slot<Vec<LevelCurve>>,
}

impl Clone for CatStore {
    fn clone(&self) -> Self {
        Self {
            talents: super::snapshot(&self.talents),
            talent_costs: super::snapshot(&self.talent_costs),
            descriptions: super::snapshot(&self.descriptions),
            unitbuy: super::snapshot(&self.unitbuy),
            evolve: super::snapshot(&self.evolve),
            curves: super::snapshot(&self.curves),
        }
    }
}

impl CatStore {
    pub fn talents(&self, vfs: &Vfs) -> Arc<HashMap<u16, Talent>> {
        super::cached(&self.talents, || {
            super::parsed(vfs, paths::SKILL_ACQUISITION, Talent::parse).unwrap_or_default()
        })
    }

    pub fn talent_costs(&self, vfs: &Vfs) -> Arc<HashMap<u8, TalentCost>> {
        super::cached(&self.talent_costs, || {
            super::parsed(vfs, paths::SKILL_LEVEL, TalentCost::parse).unwrap_or_default()
        })
    }

    pub fn descriptions(&self, vfs: &Vfs) -> Arc<Vec<String>> {
        super::cached(&self.descriptions, || {
            super::parsed(vfs, SKILL_DESCRIPTIONS, SkillDescriptions::parse)
                .map(|parsed| parsed.texts)
                .unwrap_or_default()
        })
    }

    pub fn unitbuy(&self, vfs: &Vfs) -> Arc<HashMap<u32, UnitBuy>> {
        super::cached(&self.unitbuy, || {
            super::parsed(vfs, paths::UNIT_BUY, UnitBuy::parse).unwrap_or_default()
        })
    }

    pub fn curves(&self, vfs: &Vfs) -> Arc<Vec<LevelCurve>> {
        super::cached(&self.curves, || {
            super::parsed(vfs, paths::UNIT_LEVEL, LevelCurve::parse).unwrap_or_default()
        })
    }

    pub fn evolve(&self, vfs: &Vfs) -> Arc<HashMap<u32, UnitEvolve>> {
        super::cached(&self.evolve, || {
            let mut merged: HashMap<u32, UnitEvolve> = HashMap::new();

            for bytes in super::layered(vfs, UNIT_EVOLVE) {
                let Ok(parsed) = UnitEvolve::parse(bytes) else {
                    continue;
                };

                for (cat_id, evolve) in parsed {
                    let entry = merged.entry(cat_id).or_default();

                    for index in 0..4 {
                        if entry.texts[index].is_none() {
                            entry.texts[index] = evolve.texts[index].clone();
                        }
                    }
                }
            }

            merged
        })
    }

    pub(super) fn evict(&self, filename: &str) {
        match filename {
            paths::SKILL_ACQUISITION => super::reset(&self.talents),
            paths::SKILL_LEVEL => super::reset(&self.talent_costs),
            SKILL_DESCRIPTIONS => super::reset(&self.descriptions),
            paths::UNIT_BUY => super::reset(&self.unitbuy),
            paths::UNIT_LEVEL => super::reset(&self.curves),
            UNIT_EVOLVE => super::reset(&self.evolve),
            _ => (),
        }
    }

    pub(super) fn clear(&self) {
        super::reset(&self.talents);
        super::reset(&self.talent_costs);
        super::reset(&self.descriptions);
        super::reset(&self.unitbuy);
        super::reset(&self.evolve);
        super::reset(&self.curves);
    }
}
