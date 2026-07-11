use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;
use crate::global::resolver;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CannonType {
    #[default]
    Basic,
    SlowBeam,
    IronWall,
    Thunderbolt,
    Waterblast,
    HolyBlast,
    Breakerblast,
    Curseblast,
    Unknown(u8),
}

impl From<u8> for CannonType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Basic,
            1 => Self::SlowBeam,
            2 => Self::IronWall,
            3 => Self::Thunderbolt,
            4 => Self::Waterblast,
            5 => Self::HolyBlast,
            6 => Self::Breakerblast,
            7 => Self::Curseblast,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityType {
    CatCannonAttack,
    CatCannonRange,
    CatCannonCharge,
    WorkerCatRate,
    WorkerCatWallet,
    BaseDefense,
    Research,
    BountyUp,
    Study,
    CatEnergy,
    Unknown(u8),
}

impl From<u8> for AbilityType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::CatCannonAttack,
            1 => Self::CatCannonRange,
            2 => Self::CatCannonCharge,
            3 => Self::WorkerCatRate,
            4 => Self::WorkerCatWallet,
            5 => Self::BaseDefense,
            6 => Self::Research,
            7 => Self::BountyUp,
            8 => Self::Study,
            9 => Self::CatEnergy,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreasureType {
    EoC1,
    EoC2,
    EoC3,
    ItF1,
    ItF2,
    ItF3,
    CotC1,
    CotC2,
    CotC3,
    Unknown(u8),
}

impl From<u8> for TreasureType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::EoC1,
            1 => Self::EoC2,
            2 => Self::EoC3,
            4 => Self::ItF1,
            5 => Self::ItF2,
            6 => Self::ItF3,
            7 => Self::CotC1,
            8 => Self::CotC2,
            9 => Self::CotC3,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvolutionForm {
    #[default]
    Normal,
    Evolved,
    True,
    Ultra,
    Unknown(u8),
}

impl From<u8> for EvolutionForm {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Normal,
            2 => Self::Evolved,
            3 => Self::True,
            4 => Self::Ultra,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetChara {
    pub evolution_form: EvolutionForm,
    pub level: u16,
    pub plus_level: u16,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetAbility {
    pub level: u16,
    pub plus_level: u16,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetTreasure {
    pub inferior_count: u8,
    pub normal_count: u8,
    pub superior_count: u8,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetLineup {
    pub characters: HashMap<u32, PresetChara>,
    pub slot_units: Vec<u32>,
    pub slot_cannon_type: CannonType,
    pub abilities: HashMap<AbilityType, PresetAbility>,
    pub cannon_levels: HashMap<CannonType, u16>,
    pub treasures: HashMap<TreasureType, PresetTreasure>,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> Option<PresetLineup> {
    let paths = resolver::get(dir, [filename], priority);
    let Some(target_path) = paths.first() else { return None; };
    let Ok(file_content) = fs::read_to_string(target_path) else {
        error!("Failed to read preset file at {:?}", target_path);
        return None;
    };

    let Ok(json_root) = serde_json::from_str::<Value>(&file_content) else {
        error!("Failed to deserialize preset JSON at {:?}", target_path);
        return None;
    };

    let mut lineup = PresetLineup::default();

    if let Some(chara_map) = json_root.get("chara").and_then(|base| base.get("data")).and_then(|data| data.as_object()) {
        for (unit_id_str, chara_value) in chara_map {
            if chara_value.get("remove").and_then(|remove_val| remove_val.as_bool()).unwrap_or(false) {
                continue;
            }

            let Ok(unit_id) = unit_id_str.parse::<u32>() else { continue; };
            let Some(evolution_str) = chara_value.get("evolution").and_then(|val| val.as_str()) else { continue; };
            let Some(level_str) = chara_value.get("level").and_then(|val| val.as_str()) else { continue; };
            let Some(plus_str) = chara_value.get("plus").and_then(|val| val.as_str()) else { continue; };

            let Ok(raw_evolution_id) = evolution_str.parse::<u8>() else { continue; };
            let Ok(level) = level_str.parse::<u16>() else { continue; };
            let Ok(plus_level) = plus_str.parse::<u16>() else { continue; };

            let evolution_form = EvolutionForm::from(raw_evolution_id);

            lineup.characters.insert(unit_id, PresetChara {
                evolution_form,
                level,
                plus_level,
            });
        }
    }

    if let Some(slot_zero) = json_root.get("slot").and_then(|base| base.get("data")).and_then(|data| data.get("0")) {
        if let Some(cannon_value) = slot_zero.get("cannon") {
            let cannon_id = match cannon_value {
                Value::Number(num) => num.as_u64().map(|val| val as u8),
                Value::String(string_val) => string_val.parse::<u8>().ok(),
                _ => None,
            };
            if let Some(id) = cannon_id {
                lineup.slot_cannon_type = CannonType::from(id);
            }
        }

        if let Some(char_array) = slot_zero.get("chara").and_then(|val| val.as_array()) {
            for char_value in char_array {
                let char_id = match char_value {
                    Value::Number(num) => num.as_u64().map(|val| val as u32),
                    Value::String(string_val) => string_val.parse::<u32>().ok(),
                    _ => None,
                };
                if let Some(id) = char_id {
                    lineup.slot_units.push(id);
                }
            }
        }
    }

    if let Some(ability_map) = json_root.get("ability").and_then(|base| base.get("data")).and_then(|data| data.as_object()) {
        for (ability_id_str, ability_value) in ability_map {
            let Ok(ability_id) = ability_id_str.parse::<u8>() else { continue; };
            let ability_type = AbilityType::from(ability_id);

            let level = ability_value.get("level").and_then(|val| val.as_str()).and_then(|string_val| string_val.parse::<u16>().ok()).unwrap_or(0);
            let plus_level = ability_value.get("plus").and_then(|val| val.as_str()).and_then(|string_val| string_val.parse::<u16>().ok()).unwrap_or(0);

            lineup.abilities.insert(ability_type, PresetAbility { level, plus_level });
        }
    }

    if let Some(cannon_map) = json_root.get("cannon").and_then(|base| base.get("data")).and_then(|data| data.as_object()) {
        for (cannon_id_str, cannon_value) in cannon_map {
            let Ok(cannon_id) = cannon_id_str.parse::<u8>() else { continue; };
            let cannon_type = CannonType::from(cannon_id);

            let Some(level_str) = cannon_value.get("level").and_then(|val| val.as_str()) else { continue; };
            let Ok(level) = level_str.parse::<u16>() else { continue; };

            lineup.cannon_levels.insert(cannon_type, level);
        }
    }

    if let Some(treasure_map) = json_root.get("treasure").and_then(|base| base.get("data")).and_then(|data| data.as_object()) {
        for (treasure_id_str, treasure_value) in treasure_map {
            let Ok(treasure_id) = treasure_id_str.parse::<u8>() else { continue; };
            let treasure_type = TreasureType::from(treasure_id);

            let Some(counts_array) = treasure_value.get("count").and_then(|val| val.as_array()) else { continue; };

            let Some(inferior_str) = counts_array.first().and_then(|val| val.as_str()) else { continue; };
            let Some(normal_str) = counts_array.get(1).and_then(|val| val.as_str()) else { continue; };
            let Some(superior_str) = counts_array.get(2).and_then(|val| val.as_str()) else { continue; };

            let Ok(inferior_count) = inferior_str.parse::<u8>() else { continue; };
            let Ok(normal_count) = normal_str.parse::<u8>() else { continue; };
            let Ok(superior_count) = superior_str.parse::<u8>() else { continue; };

            lineup.treasures.insert(treasure_type, PresetTreasure {
                inferior_count,
                normal_count,
                superior_count,
            });
        }
    }

    Some(lineup)
}