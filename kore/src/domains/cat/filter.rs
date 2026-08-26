pub mod evaluation;
pub mod icons;
pub mod stats;

use std::collections::{HashMap, HashSet};

use nyanko::files::img015;

use crate::systems::combat::CustomIcon;
use crate::systems::combat::registry::AbilityIcon;

pub const ATTACK_TYPE_ICONS: &[AbilityIcon] = &[
    AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
    AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
    AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
    AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
    AbilityIcon::Custom(CustomIcon::Multihit),
];

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum TalentFilterMode {
    #[default]
    Ignore,
    Consider,
    Only,
}

impl TalentFilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            TalentFilterMode::Ignore => "Ignore",
            TalentFilterMode::Consider => "Consider",
            TalentFilterMode::Only => "Only",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum MatchMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, PartialEq, Default)]
pub struct RangeInput {
    pub min: String,
    pub max: String,
}

#[derive(Default)]
pub(crate) struct FilterCounts {
    pub(crate) active: i32,
    pub(crate) passed: i32,
    pub(crate) failed: i32,
}

#[derive(Clone, PartialEq)]
pub struct CatFilterState {
    pub is_open: bool,
    pub active_icons: HashSet<AbilityIcon>,
    pub rarities: [bool; 6],
    pub forms: [bool; 4],
    pub match_mode: MatchMode,
    pub talent_mode: TalentFilterMode,
    pub ultra_talent_mode: TalentFilterMode,
    pub adv_ranges: HashMap<AbilityIcon, HashMap<&'static str, RangeInput>>,
    pub level_input: String,
    pub stat_ranges: HashMap<&'static str, RangeInput>,
    pub combo_effects: HashSet<i32>,
    pub combo_units: HashSet<u32>,
}

impl Default for CatFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_icons: HashSet::new(),
            rarities: [false; 6],
            forms: [false; 4],
            match_mode: MatchMode::And,
            talent_mode: TalentFilterMode::Ignore,
            ultra_talent_mode: TalentFilterMode::Ignore,
            adv_ranges: HashMap::new(),
            level_input: String::new(),
            stat_ranges: HashMap::new(),
            combo_effects: HashSet::new(),
            combo_units: HashSet::new(),
        }
    }
}

impl CatFilterState {
    pub fn is_active(&self) -> bool {
        let has_icons = !self.active_icons.is_empty();
        let has_rarities = self.rarities.iter().any(|&is_selected| is_selected);
        let has_forms = self.forms.iter().any(|&is_selected| is_selected);
        let normal_talent_required = self.talent_mode == TalentFilterMode::Only;
        let ultra_talent_required = self.ultra_talent_mode == TalentFilterMode::Only;
        let has_stat_ranges = self.stat_ranges.values().any(|range_input| !range_input.min.is_empty() || !range_input.max.is_empty());

        let has_combos = !self.combo_effects.is_empty();

        has_icons || has_rarities || has_forms || normal_talent_required || ultra_talent_required || has_stat_ranges || has_combos
    }
}