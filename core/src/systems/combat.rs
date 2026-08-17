pub mod abilities;
pub mod registry;

use std::collections::{HashMap, HashSet};

use nyanko::cat::unit::{LevelCurve, Talent};
use nyanko::combat::{AttrValue, Entity, Identity, REGISTRY};

use crate::common::context::GlobalContext;
use crate::systems::combat::registry::Magnification;

pub const ABILITY_X: f32 = 3.0;
pub const ABILITY_Y: f32 = 5.0;
pub const TRAIT_Y: f32 = 7.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum CustomIcon {
    #[default] None,
    Multihit,
    Kamikaze,
    BossWave,
    Dojo,
    StarredAlien,
    Burrow,
    Revive,
    Stop,
    DeathTimer,
    God,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct AbilityItem {
    pub icon_id: Option<usize>,
    pub text: String,
    pub custom_icon: CustomIcon,
    pub border_id: Option<usize>,
}

pub type AbilityGroups = (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>);

#[derive(Clone, Copy)]
pub struct RenderContext<'a> {
    pub global: GlobalContext<'a>,
    pub base_stats: &'a Entity,
    pub final_stats: &'a Entity,
    pub magnification: Magnification,
    pub current_level: i32,
    pub level_curve: Option<&'a LevelCurve>,
    pub talent_data: Option<&'a Talent>,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub is_conjure_unit: bool,
}

impl<'a> RenderContext<'a> {
    pub fn enemy(global: GlobalContext<'a>, stats: &'a Entity, magnification: Magnification) -> Self {
        Self {
            global,
            base_stats: stats,
            final_stats: stats,
            magnification,
            current_level: 0,
            level_curve: None,
            talent_data: None,
            talent_levels: None,
            is_conjure_unit: false,
        }
    }
}

pub fn present_identities<'a>(entities: impl Iterator<Item = &'a Entity>) -> HashSet<Identity> {
    let mut present = HashSet::new();

    for entity in entities {
        for ability in REGISTRY {
            if present.contains(&ability.identity) { continue; }

            if !(ability.attributes)(entity).is_empty() {
                present.insert(ability.identity);
            }
        }
    }

    present
}

pub fn talent_identities() -> HashSet<Identity> {
    REGISTRY.iter()
        .filter(|ability| ability.talent_id.is_some())
        .map(|ability| ability.identity)
        .collect()
}

pub(crate) fn comparable(value: AttrValue) -> i32 {
    match value {
        AttrValue::Finite(amount) => amount,
        AttrValue::Infinite => i32::MAX,
    }
}
