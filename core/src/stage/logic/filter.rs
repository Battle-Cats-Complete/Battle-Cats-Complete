use serde::{Deserialize, Serialize};
use tracing::trace;

use nyanko::chapter::{Map, Stage};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatRange {
    pub min: String,
    pub max: String,
}

impl StatRange {
    pub fn is_active(&self) -> bool {
        !self.min.trim().is_empty() || !self.max.trim().is_empty()
    }

    pub fn compile(&self) -> CompiledStatRange {
        let min_val = if self.min.trim().is_empty() {
            i64::MIN
        } else {
            self.min.trim().parse::<i64>().unwrap_or_else(|_| {
                trace!("Failed to parse min filter value: {}", self.min);
                i64::MIN
            })
        };

        let max_val = if self.max.trim().is_empty() {
            i64::MAX
        } else {
            self.max.trim().parse::<i64>().unwrap_or_else(|_| {
                trace!("Failed to parse max filter value: {}", self.max);
                i64::MAX
            })
        };

        CompiledStatRange {
            min: min_val,
            max: max_val,
            active: self.is_active(),
        }
    }
}

pub struct CompiledStatRange {
    pub min: i64,
    pub max: i64,
    pub active: bool,
}

impl CompiledStatRange {
    pub fn matches(&self, target_val: i64) -> bool {
        if !self.active { return true; }
        target_val >= self.min && target_val <= self.max
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageFilterState {
    pub is_open: bool,

    pub category_name: String,
    pub map_name: String,
    pub stage_name: String,

    pub continues: Option<bool>,
    pub boss_guard: Option<bool>,

    pub width: StatRange,
    pub base_hp: StatRange,
    pub max_enemies: StatRange,
    pub time_limit: StatRange,
    pub energy: StatRange,
    pub xp: StatRange,

    pub min_spawn: StatRange,
    pub max_spawn: StatRange,
    pub difficulty: StatRange,
    pub max_crowns: StatRange,
    pub target_crowns: StatRange,

    pub min_cost: StatRange,
    pub max_cost: StatRange,
    pub deploy_limit: StatRange,
    pub allowed_rows: StatRange,

    pub base_id: StatRange,
    pub anim_base_id: StatRange,
    pub background_id: StatRange,
    pub init_track: StatRange,
    pub boss_track: StatRange,
    pub bgm_change_percent: StatRange,
}

pub struct CompiledStageFilter {
    category_name: String,
    map_name: String,
    stage_name: String,

    continues: Option<bool>,
    boss_guard: Option<bool>,
    width: CompiledStatRange,
    base_hp: CompiledStatRange,
    max_enemies: CompiledStatRange,
    time_limit: CompiledStatRange,
    energy: CompiledStatRange,
    xp: CompiledStatRange,
    min_spawn: CompiledStatRange,
    max_spawn: CompiledStatRange,
    difficulty: CompiledStatRange,
    max_crowns: CompiledStatRange,
    target_crowns: CompiledStatRange,
    min_cost: CompiledStatRange,
    max_cost: CompiledStatRange,
    deploy_limit: CompiledStatRange,
    allowed_rows: CompiledStatRange,
    base_id: CompiledStatRange,
    anim_base_id: CompiledStatRange,
    background_id: CompiledStatRange,
    init_track: CompiledStatRange,
    boss_track: CompiledStatRange,
    bgm_change_percent: CompiledStatRange,
}

impl StageFilterState {
    pub fn is_active(&self) -> bool {
        !self.category_name.trim().is_empty()
            || !self.map_name.trim().is_empty()
            || !self.stage_name.trim().is_empty()
            || self.continues.is_some()
            || self.boss_guard.is_some()
            || self.width.is_active()
            || self.base_hp.is_active()
            || self.max_enemies.is_active()
            || self.time_limit.is_active()
            || self.energy.is_active()
            || self.xp.is_active()
            || self.min_cost.is_active()
            || self.max_cost.is_active()
            || self.min_spawn.is_active()
            || self.max_spawn.is_active()
            || self.difficulty.is_active()
            || self.max_crowns.is_active()
            || self.target_crowns.is_active()
            || self.deploy_limit.is_active()
            || self.allowed_rows.is_active()
            || self.base_id.is_active()
            || self.anim_base_id.is_active()
            || self.background_id.is_active()
            || self.init_track.is_active()
            || self.boss_track.is_active()
            || self.bgm_change_percent.is_active()
    }

    pub fn compile(&self) -> CompiledStageFilter {
        CompiledStageFilter {
            category_name: self.category_name.trim().to_lowercase(),
            map_name: self.map_name.trim().to_lowercase(),
            stage_name: self.stage_name.trim().to_lowercase(),
            continues: self.continues,
            boss_guard: self.boss_guard,
            width: self.width.compile(),
            base_hp: self.base_hp.compile(),
            max_enemies: self.max_enemies.compile(),
            time_limit: self.time_limit.compile(),
            energy: self.energy.compile(),
            xp: self.xp.compile(),
            min_spawn: self.min_spawn.compile(),
            max_spawn: self.max_spawn.compile(),
            difficulty: self.difficulty.compile(),
            max_crowns: self.max_crowns.compile(),
            target_crowns: self.target_crowns.compile(),
            min_cost: self.min_cost.compile(),
            max_cost: self.max_cost.compile(),
            deploy_limit: self.deploy_limit.compile(),
            allowed_rows: self.allowed_rows.compile(),
            base_id: self.base_id.compile(),
            anim_base_id: self.anim_base_id.compile(),
            background_id: self.background_id.compile(),
            init_track: self.init_track.compile(),
            boss_track: self.boss_track.compile(),
            bgm_change_percent: self.bgm_change_percent.compile(),
        }
    }
}

impl CompiledStageFilter {
    pub fn is_active(&self) -> bool {
        !self.category_name.is_empty()
            || !self.map_name.is_empty()
            || !self.stage_name.is_empty()
            || self.continues.is_some()
            || self.boss_guard.is_some()
            || self.width.active
            || self.base_hp.active
            || self.max_enemies.active
            || self.time_limit.active
            || self.energy.active
            || self.xp.active
            || self.min_cost.active
            || self.max_cost.active
            || self.min_spawn.active
            || self.max_spawn.active
            || self.difficulty.active
            || self.max_crowns.active
            || self.target_crowns.active
            || self.deploy_limit.active
            || self.allowed_rows.active
            || self.base_id.active
            || self.anim_base_id.active
            || self.background_id.active
            || self.init_track.active
            || self.boss_track.active
            || self.bgm_change_percent.active
    }

    pub fn matches(&self, cat_name: &str, map: &Map, stage: &Stage) -> bool {
        if !self.is_active() { return true; }

        if !self.category_name.is_empty() && !cat_name.to_lowercase().contains(&self.category_name) {
            return false;
        }

        if !self.map_name.is_empty() && !map.name.to_lowercase().contains(&self.map_name) {
            return false;
        }

        if !self.stage_name.is_empty() && !stage.name.to_lowercase().contains(&self.stage_name) {
            return false;
        }

        if let Some(c) = self.continues {
            if stage.is_no_continues == c { return false; }
        }

        if let Some(bg) = self.boss_guard {
            if stage.is_base_indestructible != bg { return false; }
        }

        if self.target_crowns.active && (stage.max_crowns as i64) < self.target_crowns.min {
            return false;
        }

        let mut actual_hp = stage.base_hp as i64;
        let target_crown = self.target_crowns.min.max(1) as u32;

        if stage.anim_base_id != 0 && target_crown > 1 {
            let mag = match target_crown {
                2 => map.crown_2_mag.unwrap_or(100),
                3 => map.crown_3_mag.unwrap_or(100),
                4 => map.crown_4_mag.unwrap_or(100),
                _ => 100,
            };
            actual_hp = (actual_hp * mag as i64) / 100;
        }

        if !self.base_hp.matches(actual_hp) { return false; }

        let anim_shift = if stage.anim_base_id >= 2 { stage.anim_base_id - 2 } else { stage.anim_base_id };
        if !self.anim_base_id.matches(anim_shift as i64) { return false; }

        if !self.width.matches(stage.width as i64) { return false; }
        if !self.max_enemies.matches(stage.max_enemies as i64) { return false; }
        if !self.time_limit.matches(stage.time_limit as i64) { return false; }
        if !self.energy.matches(stage.energy as i64) { return false; }
        if !self.xp.matches(stage.xp as i64) { return false; }
        if !self.min_cost.matches(stage.min_cost as i64) { return false; }
        if !self.max_cost.matches(stage.max_cost as i64) { return false; }
        if !self.deploy_limit.matches(stage.deploy_limit as i64) { return false; }
        if !self.allowed_rows.matches(stage.allowed_rows as i64) { return false; }
        if !self.min_spawn.matches(stage.min_spawn as i64) { return false; }
        if !self.max_spawn.matches(stage.max_spawn as i64) { return false; }
        if !self.difficulty.matches(stage.difficulty as i64) { return false; }
        if !self.max_crowns.matches(stage.max_crowns as i64) { return false; }

        if !self.base_id.matches(stage.base_id as i64) { return false; }
        if !self.background_id.matches(stage.background_id as i64) { return false; }
        if !self.init_track.matches(stage.init_track as i64) { return false; }
        if !self.boss_track.matches(stage.boss_track as i64) { return false; }
        if !self.bgm_change_percent.matches(stage.bgm_change_percent as i64) { return false; }

        true
    }
}