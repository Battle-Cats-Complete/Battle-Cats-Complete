use serde::{Deserialize, Serialize};
use tracing::trace;

use nyanko::chapter::{Map, Stage};
use nyanko::chapter::stage::{BattlegroundEntry, BossType, EnemyAmount};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StatRange {
    pub min: String,
    pub max: String,
}

impl StatRange {
    pub fn is_active(&self) -> bool {
        !self.min.trim().is_empty() || !self.max.trim().is_empty()
    }

    pub fn compile(&self, offset: i64) -> CompiledStatRange {
        let min_val = if self.min.trim().is_empty() {
            i64::MIN
        } else {
            self.min.trim().parse::<i64>().map(|v| v + offset).unwrap_or_else(|_| {
                trace!("Failed to parse min filter value: {}", self.min);
                i64::MIN
            })
        };

        let max_val = if self.max.trim().is_empty() {
            i64::MAX
        } else {
            self.max.trim().parse::<i64>().map(|v| v + offset).unwrap_or_else(|_| {
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
pub struct EnemyFilter {
    pub enemy_id: StatRange,
    pub amount: StatRange,
    pub start_frame: StatRange,
    pub respawn_min: StatRange,
    pub respawn_max: StatRange,
    pub base_hp_perc: StatRange,
    pub layer_min: StatRange,
    pub layer_max: StatRange,
    pub magnification: StatRange,
    pub atk_magnification: StatRange,
    pub score: StatRange,
    pub time_flag: StatRange,
    pub kill_count: StatRange,
    pub boss_type: Option<u32>,
    pub is_base: Option<bool>,
}

impl EnemyFilter {
    pub fn is_active(&self) -> bool {
        self.enemy_id.is_active()
            || self.amount.is_active()
            || self.start_frame.is_active()
            || self.respawn_min.is_active()
            || self.respawn_max.is_active()
            || self.base_hp_perc.is_active()
            || self.layer_min.is_active()
            || self.layer_max.is_active()
            || self.magnification.is_active()
            || self.atk_magnification.is_active()
            || self.score.is_active()
            || self.time_flag.is_active()
            || self.kill_count.is_active()
            || self.boss_type.is_some()
            || self.is_base.is_some()
    }

    pub fn compile(&self) -> CompiledEnemyFilter {
        CompiledEnemyFilter {
            enemy_id: self.enemy_id.compile(2),
            amount: self.amount.compile(0),
            start_frame: self.start_frame.compile(0),
            respawn_min: self.respawn_min.compile(0),
            respawn_max: self.respawn_max.compile(0),
            base_hp_perc: self.base_hp_perc.compile(0),
            layer_min: self.layer_min.compile(0),
            layer_max: self.layer_max.compile(0),
            magnification: self.magnification.compile(0),
            atk_magnification: self.atk_magnification.compile(0),
            score: self.score.compile(0),
            time_flag: self.time_flag.compile(0),
            kill_count: self.kill_count.compile(0),
            boss_type: self.boss_type,
            is_base: self.is_base,
        }
    }
}

pub struct CompiledEnemyFilter {
    enemy_id: CompiledStatRange,
    amount: CompiledStatRange,
    start_frame: CompiledStatRange,
    respawn_min: CompiledStatRange,
    respawn_max: CompiledStatRange,
    base_hp_perc: CompiledStatRange,
    layer_min: CompiledStatRange,
    layer_max: CompiledStatRange,
    magnification: CompiledStatRange,
    atk_magnification: CompiledStatRange,
    score: CompiledStatRange,
    time_flag: CompiledStatRange,
    kill_count: CompiledStatRange,
    boss_type: Option<u32>,
    is_base: Option<bool>,
}

impl CompiledEnemyFilter {
    pub fn matches(&self, enemy: &BattlegroundEntry) -> bool {
        let internal_amount = match enemy.amount {
            EnemyAmount::Infinite => 0,
            EnemyAmount::Limit(v) => v as i64,
        };

        let internal_boss_type = match enemy.boss_type {
            BossType::None => 0,
            BossType::Boss => 1,
            BossType::ScreenShake => 2,
            BossType::Unknown(v) => v,
        };

        if let Some(bt) = self.boss_type {
            if internal_boss_type != bt { return false; }
        }

        if let Some(ib) = self.is_base {
            if enemy.is_base != ib { return false; }
        }

        if !self.enemy_id.matches((enemy.enemy_id + 2) as i64) { return false; }
        if !self.amount.matches(internal_amount) { return false; }
        if !self.start_frame.matches(enemy.start_frame as i64) { return false; }
        if !self.respawn_min.matches(enemy.respawn_min as i64) { return false; }
        if !self.respawn_max.matches(enemy.respawn_max as i64) { return false; }
        if !self.base_hp_perc.matches(enemy.base_hp_perc as i64) { return false; }
        if !self.layer_min.matches(enemy.layer_min as i64) { return false; }
        if !self.layer_max.matches(enemy.layer_max as i64) { return false; }
        if !self.magnification.matches(enemy.magnification as i64) { return false; }
        if !self.atk_magnification.matches(enemy.atk_magnification as i64) { return false; }
        if !self.score.matches(enemy.score as i64) { return false; }
        if !self.time_flag.matches(enemy.time_flag as i64) { return false; }
        if !self.kill_count.matches(enemy.kill_count as i64) { return false; }

        true
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

    pub enemies: Vec<EnemyFilter>,
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

    enemies: Vec<CompiledEnemyFilter>,
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
            || !self.enemies.is_empty()
    }

    pub fn compile(&self) -> CompiledStageFilter {
        CompiledStageFilter {
            category_name: self.category_name.trim().to_lowercase(),
            map_name: self.map_name.trim().to_lowercase(),
            stage_name: self.stage_name.trim().to_lowercase(),
            continues: self.continues,
            boss_guard: self.boss_guard,
            width: self.width.compile(0),
            base_hp: self.base_hp.compile(0),
            max_enemies: self.max_enemies.compile(0),
            time_limit: self.time_limit.compile(0),
            energy: self.energy.compile(0),
            xp: self.xp.compile(0),
            min_spawn: self.min_spawn.compile(0),
            max_spawn: self.max_spawn.compile(0),
            difficulty: self.difficulty.compile(0),
            max_crowns: self.max_crowns.compile(0),
            target_crowns: self.target_crowns.compile(0),
            min_cost: self.min_cost.compile(0),
            max_cost: self.max_cost.compile(0),
            deploy_limit: self.deploy_limit.compile(0),
            allowed_rows: self.allowed_rows.compile(0),
            base_id: self.base_id.compile(0),
            anim_base_id: self.anim_base_id.compile(2),
            background_id: self.background_id.compile(0),
            init_track: self.init_track.compile(0),
            boss_track: self.boss_track.compile(0),
            bgm_change_percent: self.bgm_change_percent.compile(0),
            enemies: self.enemies.iter().map(|e| e.compile()).collect(),
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
            || !self.enemies.is_empty()
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
        if !self.anim_base_id.matches(stage.anim_base_id as i64) { return false; }

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

        if !self.enemies.is_empty() {
            for enemy_filter in &self.enemies {
                let mut found_match = false;
                for stage_enemy in &stage.enemies {
                    if enemy_filter.matches(stage_enemy) {
                        found_match = true;
                        break;
                    }
                }
                if !found_match {
                    return false;
                }
            }
        }

        true
    }
}