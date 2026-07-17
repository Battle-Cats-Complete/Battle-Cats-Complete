use nyanko::chapter::stage::{BattlegroundEntry, BossType, EnemyAmount};
use serde::{Deserialize, Serialize};

use super::range::{CompiledStatRange, StatRange};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct EnemyFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
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
        !self.name_or_id.trim().is_empty()
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
        let name_or_id = self.name_or_id.trim().to_lowercase();
        let parsed_id = name_or_id.parse::<u32>().ok();

        CompiledEnemyFilter {
            is_exclude: self.is_exclude,
            name_or_id,
            parsed_id,
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
    pub is_exclude: bool,
    pub name_or_id: String,
    pub parsed_id: Option<u32>,
    pub amount: CompiledStatRange,
    pub start_frame: CompiledStatRange,
    pub respawn_min: CompiledStatRange,
    pub respawn_max: CompiledStatRange,
    pub base_hp_perc: CompiledStatRange,
    pub layer_min: CompiledStatRange,
    pub layer_max: CompiledStatRange,
    pub magnification: CompiledStatRange,
    pub atk_magnification: CompiledStatRange,
    pub score: CompiledStatRange,
    pub time_flag: CompiledStatRange,
    pub kill_count: CompiledStatRange,
    pub boss_type: Option<u32>,
    pub is_base: Option<bool>,
}

impl CompiledEnemyFilter {
    pub fn matches(&self, enemy: &BattlegroundEntry, enemy_name_registry: &[String]) -> bool {
        let internal_amount = if let EnemyAmount::Limit(val) = enemy.amount { val as i64 } else { 0 };

        let internal_boss_type = match enemy.boss_type {
            BossType::None => 0,
            BossType::Boss => 1,
            BossType::ScreenShake => 2,
            BossType::Unknown(val) => val,
        };

        if let Some(boss_type) = self.boss_type && internal_boss_type != boss_type { return false; }
        if let Some(is_base) = self.is_base && enemy.is_base != is_base { return false; }
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

        if self.name_or_id.is_empty() { return true; }
        if self.parsed_id == Some(enemy.enemy_id) { return true; }

        enemy_name_registry
            .get(enemy.enemy_id as usize)
            .filter(|name| !name.is_empty())
            .map_or_else(
                || format!("{:03}-e", enemy.enemy_id).contains(&self.name_or_id),
                |name| name.to_lowercase().contains(&self.name_or_id)
            )
    }
}