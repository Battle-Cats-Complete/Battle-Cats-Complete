use crate::core::files::skillacquisition::{TalentRaw, TalentGroupRaw};

#[derive(Debug, Clone)]
pub struct CatTalents {
    pub unit_id: u16,
    pub implicit_targets: Vec<TalentTarget>,
    pub normal: Vec<SingleTalent>,
    pub ultra: Vec<SingleTalent>,
}

#[derive(Debug, Clone)]
pub struct SingleTalent {
    pub ability_id: u8,
    pub max_level: u8,
    pub params: Vec<(u16, u16)>, // (min, max) pairs
    pub description_id: u8,
    pub cost_id: u8,
    pub name_id: i16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TalentTarget {
    Red = 0,
    Floating = 1,
    Black = 2,
    Metal = 3,
    Angel = 4,
    Alien = 5,
    Zombie = 6,
    Relic = 7,
    Traitless = 8,
    Witch = 9,  // Added
    Eva = 10,   // Added
    Aku = 11,
    Unknown = 99,
}

impl CatTalents {
    pub fn from_raw(raw: &TalentRaw) -> Self {
        let mut normal = Vec::new();
        let mut ultra = Vec::new();

        for group in &raw.groups {
            let talent = SingleTalent::from_raw(group);
            // limit: 0 = Normal, 1 = Ultra
            if group.limit == 1 {
                ultra.push(talent);
            } else {
                normal.push(talent);
            }
        }

        Self {
            unit_id: raw.id,
            implicit_targets: parse_targets(raw.type_id),
            normal,
            ultra,
        }
    }
}

impl SingleTalent {
    pub fn from_raw(group: &TalentGroupRaw) -> Self {
        let mut params = Vec::new();
        // Collect valid min/max pairs (0,0 is skipped)
        for (min, max) in [
            (group.min_1, group.max_1),
            (group.min_2, group.max_2),
            (group.min_3, group.max_3),
            (group.min_4, group.max_4),
        ] {
            if min != 0 || max != 0 {
                params.push((min, max));
            }
        }

        Self {
            ability_id: group.ability_id,
            max_level: group.max_level,
            params,
            description_id: group.text_id,
            cost_id: group.cost_id,
            name_id: group.name_id,
        }
    }
}

fn parse_targets(mask: u16) -> Vec<TalentTarget> {
    let mut targets = Vec::new();
    for i in 0..12 {
        if (mask & (1 << i)) != 0 {
            let t = match i {
                0 => TalentTarget::Red,
                1 => TalentTarget::Floating,
                2 => TalentTarget::Black,
                3 => TalentTarget::Metal,
                4 => TalentTarget::Angel,
                5 => TalentTarget::Alien,
                6 => TalentTarget::Zombie,
                7 => TalentTarget::Relic,
                8 => TalentTarget::Traitless,
                9 => TalentTarget::Witch, // Added
                10 => TalentTarget::Eva,  // Added
                11 => TalentTarget::Aku,
                _ => TalentTarget::Unknown,
            };
            if t != TalentTarget::Unknown {
                targets.push(t);
            }
        }
    }
    targets
}