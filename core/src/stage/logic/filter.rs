use std::collections::HashMap;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::{Map, Stage};
use nyanko::chapter::map::{BonusType, LockSkipDataEntry, RuleType};
use nyanko::chapter::stage::{BattlegroundEntry, BossType, EnemyAmount, RewardStructure, ScatCpuSetting};
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::global::formats::gatyaitembuy::GatyaItemBuy;
use crate::global::formats::gatyaitemname::GatyaItemName;

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
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
            self.min.trim().parse::<i64>().map(|val| val + offset).unwrap_or_else(|_| {
                trace!("Failed to parse min filter value: {}", self.min);
                i64::MIN
            })
        };

        let max_val = if self.max.trim().is_empty() {
            i64::MAX
        } else {
            self.max.trim().parse::<i64>().map(|val| val + offset).unwrap_or_else(|_| {
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
    is_exclude: bool,
    name_or_id: String,
    parsed_id: Option<u32>,
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
    pub fn matches(&self, enemy: &BattlegroundEntry, enemy_name_registry: &[String]) -> bool {
        let internal_amount = if let EnemyAmount::Limit(val) = enemy.amount { val as i64 } else { 0 };

        let internal_boss_type = match enemy.boss_type {
            BossType::None => 0,
            BossType::Boss => 1,
            BossType::ScreenShake => 2,
            BossType::Unknown(val) => val,
        };

        if let Some(boss_type) = self.boss_type { if internal_boss_type != boss_type { return false; } }
        if let Some(is_base) = self.is_base { if enemy.is_base != is_base { return false; } }
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
        if self.parsed_id.map_or(false, |id| (enemy.enemy_id + 2) == id) { return true; }

        enemy_name_registry
            .get(enemy.enemy_id as usize)
            .filter(|name| !name.is_empty())
            .map_or_else(
                || format!("{:03}-e", enemy.enemy_id).contains(&self.name_or_id),
                |name| name.to_lowercase().contains(&self.name_or_id)
            )
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TreasureFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub amount: StatRange,
    pub chance: StatRange,
}

impl TreasureFilter {
    pub fn is_active(&self) -> bool {
        !self.name_or_id.trim().is_empty()
            || self.amount.is_active()
            || self.chance.is_active()
    }

    pub fn compile(&self) -> CompiledTreasureFilter {
        let name_or_id = self.name_or_id.trim().to_lowercase();
        let parsed_id = name_or_id.parse::<u32>().ok();

        CompiledTreasureFilter {
            is_exclude: self.is_exclude,
            name_or_id,
            parsed_id,
            amount: self.amount.compile(0),
            chance: self.chance.compile(0),
        }
    }
}

pub struct CompiledTreasureFilter {
    is_exclude: bool,
    name_or_id: String,
    parsed_id: Option<u32>,
    amount: CompiledStatRange,
    chance: CompiledStatRange,
}

impl CompiledTreasureFilter {
    #[allow(clippy::too_many_arguments)]
    pub fn matches_drop(
        &self,
        target_id: u32,
        drop_amt: u32,
        drop_chnc: u32,
        buy_reg: &HashMap<u32, GatyaItemBuy>,
        name_reg: &HashMap<usize, GatyaItemName>,
        drop_chara_reg: &HashMap<u32, u32>,
        unit_buy_reg: &HashMap<u32, UnitBuy>,
        cat_name_reg: &HashMap<u32, Vec<String>>,
    ) -> bool {
        if !self.amount.matches(drop_amt as i64) { return false; }
        if !self.chance.matches(drop_chnc as i64) { return false; }

        if self.name_or_id.is_empty() { return true; }
        if self.parsed_id == Some(target_id) { return true; }

        if let Some(item_buy) = buy_reg.get(&target_id) {
            return name_reg.get(&(item_buy.row_index as usize))
                .map_or(false, |name| name.name.to_lowercase().contains(&self.name_or_id));
        }

        if let Some(&chara_id) = drop_chara_reg.get(&target_id) {
            return cat_name_reg.get(&chara_id)
                .map_or(false, |names| names.iter().any(|name| name.contains(&self.name_or_id)));
        }

        if let Some((&unit_id, _)) = unit_buy_reg.iter().find(|(_, row)| row.true_form_id == target_id as i32) {
            return cat_name_reg.get(&unit_id)
                .map_or(false, |names| names.iter().any(|name| name.contains(&self.name_or_id)));
        }

        false
    }
}

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

    pub fn compile(&self) -> CompiledMaterialFilter {
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

pub struct CompiledMaterialFilter {
    is_exclude: bool,
    name_or_id: String,
    parsed_id: Option<u32>,
    amount: CompiledStatRange,
}

impl CompiledMaterialFilter {
    pub fn matches_material(
        &self,
        index: usize,
        drop_amount: u32,
        buy_reg: &HashMap<u32, GatyaItemBuy>,
        name_reg: &HashMap<usize, GatyaItemName>,
    ) -> bool {
        if !self.amount.matches(drop_amount as i64) { return false; }
        if self.name_or_id.is_empty() { return true; }

        let Some(&item_id) = crate::stage::logic::materials::MAT_IDS.get(index) else { return false; };
        if self.parsed_id == Some(item_id) { return true; }

        let Some(item_buy) = buy_reg.get(&item_id) else { return false; };
        let Some(name_entry) = name_reg.get(&(item_buy.row_index as usize)) else { return false; };

        name_entry.name.to_lowercase().contains(&self.name_or_id)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct LineupFilter {
    pub is_exclude: bool,
    pub name_or_id: String,
    pub level: StatRange,
}

impl LineupFilter {
    pub fn is_active(&self) -> bool {
        !self.name_or_id.trim().is_empty() || self.level.is_active()
    }

    pub fn compile(&self) -> CompiledLineupFilter {
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

pub struct CompiledLineupFilter {
    is_exclude: bool,
    name_or_id: String,
    parsed_id: Option<u32>,
    level: CompiledStatRange,
}

impl CompiledLineupFilter {
    pub fn matches_lineup(
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

                let name_match = cat_name_reg.get(&unit_id).map_or(false, |names| {
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StageFilterState {
    pub is_open: bool,

    pub category_name: String,
    pub map_name: String,
    pub stage_name: String,

    pub continues: Option<bool>,
    pub boss_guard: Option<bool>,
    pub use_super_cpu: Option<bool>,

    pub rule_trust_fund: Option<bool>,
    pub rule_cooldown_equality: Option<bool>,
    pub rule_rarity_limit: Option<bool>,
    pub rule_cheap_labor: Option<bool>,
    pub rule_cat_cost: Option<bool>,
    pub rule_cat_production: Option<bool>,
    pub rule_total_deploy_limit: Option<bool>,
    pub rule_more_than_one: Option<bool>,
    pub rule_mega_cat_cannon: Option<bool>,
    pub rule_uniform_motion: Option<bool>,
    pub invalid_combos: Option<bool>,

    pub bonus_weaken: Option<bool>,
    pub bonus_freeze: Option<bool>,
    pub bonus_slow: Option<bool>,
    pub bonus_knockback: Option<bool>,
    pub bonus_strong_attack: Option<bool>,
    pub bonus_massive_damage: Option<bool>,
    pub bonus_strong_defense: Option<bool>,
    pub bonus_resist: Option<bool>,

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
    pub treasures: Vec<TreasureFilter>,
    pub materials: Vec<MaterialFilter>,
    pub lineup_cats: Vec<LineupFilter>,
}

pub struct CompiledStageFilter {
    pub source_hash: u64,
    category_name: String,
    map_name: String,
    stage_name: String,

    continues: Option<bool>,
    boss_guard: Option<bool>,
    use_super_cpu: Option<bool>,

    rule_trust_fund: Option<bool>,
    rule_cooldown_equality: Option<bool>,
    rule_rarity_limit: Option<bool>,
    rule_cheap_labor: Option<bool>,
    rule_cat_cost: Option<bool>,
    rule_cat_production: Option<bool>,
    rule_total_deploy_limit: Option<bool>,
    rule_more_than_one: Option<bool>,
    rule_mega_cat_cannon: Option<bool>,
    rule_uniform_motion: Option<bool>,
    invalid_combos: Option<bool>,

    bonus_weaken: Option<bool>,
    bonus_freeze: Option<bool>,
    bonus_slow: Option<bool>,
    bonus_knockback: Option<bool>,
    bonus_strong_attack: Option<bool>,
    bonus_massive_damage: Option<bool>,
    bonus_strong_defense: Option<bool>,
    bonus_resist: Option<bool>,

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
    treasures: Vec<CompiledTreasureFilter>,
    materials: Vec<CompiledMaterialFilter>,
    lineup_cats: Vec<CompiledLineupFilter>,
}

impl StageFilterState {
    pub fn is_active(&self) -> bool {
        !self.category_name.trim().is_empty()
            || !self.map_name.trim().is_empty()
            || !self.stage_name.trim().is_empty()
            || self.continues.is_some()
            || self.boss_guard.is_some()
            || self.use_super_cpu.is_some()
            || self.rule_trust_fund.is_some()
            || self.rule_cooldown_equality.is_some()
            || self.rule_rarity_limit.is_some()
            || self.rule_cheap_labor.is_some()
            || self.rule_cat_cost.is_some()
            || self.rule_cat_production.is_some()
            || self.rule_total_deploy_limit.is_some()
            || self.rule_more_than_one.is_some()
            || self.rule_mega_cat_cannon.is_some()
            || self.rule_uniform_motion.is_some()
            || self.invalid_combos.is_some()
            || self.bonus_weaken.is_some()
            || self.bonus_freeze.is_some()
            || self.bonus_slow.is_some()
            || self.bonus_knockback.is_some()
            || self.bonus_strong_attack.is_some()
            || self.bonus_massive_damage.is_some()
            || self.bonus_strong_defense.is_some()
            || self.bonus_resist.is_some()
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
            || self.enemies.iter().any(|enemy| enemy.is_active())
            || self.treasures.iter().any(|treasure| treasure.is_active())
            || self.materials.iter().any(|material| material.is_active())
            || self.lineup_cats.iter().any(|lineup| lineup.is_active())
    }

    pub fn compile(&self) -> CompiledStageFilter {
        trace!("Compiling StageFilterState into CompiledStageFilter...");
        CompiledStageFilter {
            source_hash: 0,
            category_name: self.category_name.trim().to_lowercase(),
            map_name: self.map_name.trim().to_lowercase(),
            stage_name: self.stage_name.trim().to_lowercase(),

            continues: self.continues,
            boss_guard: self.boss_guard,
            use_super_cpu: self.use_super_cpu,

            rule_trust_fund: self.rule_trust_fund,
            rule_cooldown_equality: self.rule_cooldown_equality,
            rule_rarity_limit: self.rule_rarity_limit,
            rule_cheap_labor: self.rule_cheap_labor,
            rule_cat_cost: self.rule_cat_cost,
            rule_cat_production: self.rule_cat_production,
            rule_total_deploy_limit: self.rule_total_deploy_limit,
            rule_more_than_one: self.rule_more_than_one,
            rule_mega_cat_cannon: self.rule_mega_cat_cannon,
            rule_uniform_motion: self.rule_uniform_motion,
            invalid_combos: self.invalid_combos,

            bonus_weaken: self.bonus_weaken,
            bonus_freeze: self.bonus_freeze,
            bonus_slow: self.bonus_slow,
            bonus_knockback: self.bonus_knockback,
            bonus_strong_attack: self.bonus_strong_attack,
            bonus_massive_damage: self.bonus_massive_damage,
            bonus_strong_defense: self.bonus_strong_defense,
            bonus_resist: self.bonus_resist,

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

            enemies: self.enemies.iter().filter(|enemy| enemy.is_active()).map(|enemy| enemy.compile()).collect(),
            treasures: self.treasures.iter().filter(|treasure| treasure.is_active()).map(|treasure| treasure.compile()).collect(),
            materials: self.materials.iter().filter(|material| material.is_active()).map(|material| material.compile()).collect(),
            lineup_cats: self.lineup_cats.iter().filter(|lineup| lineup.is_active()).map(|lineup| lineup.compile()).collect(),
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
            || self.use_super_cpu.is_some()
            || self.rule_trust_fund.is_some()
            || self.rule_cooldown_equality.is_some()
            || self.rule_rarity_limit.is_some()
            || self.rule_cheap_labor.is_some()
            || self.rule_cat_cost.is_some()
            || self.rule_cat_production.is_some()
            || self.rule_total_deploy_limit.is_some()
            || self.rule_more_than_one.is_some()
            || self.rule_mega_cat_cannon.is_some()
            || self.rule_uniform_motion.is_some()
            || self.invalid_combos.is_some()
            || self.bonus_weaken.is_some()
            || self.bonus_freeze.is_some()
            || self.bonus_slow.is_some()
            || self.bonus_knockback.is_some()
            || self.bonus_strong_attack.is_some()
            || self.bonus_massive_damage.is_some()
            || self.bonus_strong_defense.is_some()
            || self.bonus_resist.is_some()
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
            || !self.treasures.is_empty()
            || !self.materials.is_empty()
            || !self.lineup_cats.is_empty()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn matches(
        &self,
        cat_name: &str,
        map: &Map,
        stage: &Stage,
        enemy_name_registry: &[String],
        lock_registry: &HashMap<u32, LockSkipDataEntry>,
        cpu_setting: &ScatCpuSetting,
        item_buy_registry: &HashMap<u32, GatyaItemBuy>,
        item_name_registry: &HashMap<usize, GatyaItemName>,
        drop_chara_registry: &HashMap<u32, u32>,
        unit_buy_registry: &HashMap<u32, UnitBuy>,
        cat_name_registry: &HashMap<u32, Vec<String>>,
    ) -> bool {
        if !self.is_active() { return true; }

        if let Some(has_continues) = self.continues { if stage.is_no_continues == has_continues { return false; } }
        if let Some(has_boss_guard) = self.boss_guard { if stage.is_base_indestructible != has_boss_guard { return false; } }

        if let Some(use_super_cpu) = self.use_super_cpu {
            let mut cpu_allowed = cpu_setting.super_cpu_consume_amount > 0;
            if let Some(global_id) = stage.category.global_map_id(stage.map_id) {
                if let Some(entry) = lock_registry.get(&global_id) {
                    if entry.excluded_map_id == global_id { cpu_allowed = false; }
                }
            }
            if cpu_allowed != use_super_cpu { return false; }
        }

        macro_rules! check_rule {
            ($opt:expr, $variant:pat) => {
                if let Some(expected) = $opt {
                    let has_rule = map.special_rules.as_ref()
                        .map_or(false, |special_rules| special_rules.rules.iter().any(|rule| matches!(rule, $variant)));
                    if has_rule != expected { return false; }
                }
            };
        }

        check_rule!(self.rule_trust_fund, RuleType::TrustFund(_));
        check_rule!(self.rule_cooldown_equality, RuleType::CooldownEquality(_));
        check_rule!(self.rule_rarity_limit, RuleType::RarityLimit(_));
        check_rule!(self.rule_cheap_labor, RuleType::CheapLabor(_));
        check_rule!(self.rule_cat_cost, RuleType::CatCost(_));
        check_rule!(self.rule_cat_production, RuleType::CatProduction(_));
        check_rule!(self.rule_total_deploy_limit, RuleType::TotalDeployLimit(_));
        check_rule!(self.rule_more_than_one, RuleType::MoreThanOne(_));
        check_rule!(self.rule_mega_cat_cannon, RuleType::MegaCatCannon(_));
        check_rule!(self.rule_uniform_motion, RuleType::UniformMotion(_));

        if let Some(has_invalid_combos) = self.invalid_combos {
            let has_ic = !map.invalid_combos.is_empty();
            if has_ic != has_invalid_combos { return false; }
        }

        macro_rules! check_bonus {
            ($opt:expr, $variant:pat) => {
                if let Some(expected) = $opt {
                    let has_bonus = map.score_bonuses.as_ref()
                        .map_or(false, |score_bonuses| score_bonuses.bonuses.iter().any(|bonus| matches!(bonus, $variant)));
                    if has_bonus != expected { return false; }
                }
            };
        }

        check_bonus!(self.bonus_weaken, BonusType::Weaken(_));
        check_bonus!(self.bonus_freeze, BonusType::Freeze(_));
        check_bonus!(self.bonus_slow, BonusType::Slow(_));
        check_bonus!(self.bonus_knockback, BonusType::Knockback(_));
        check_bonus!(self.bonus_strong_attack, BonusType::StrongAttack(_));
        check_bonus!(self.bonus_massive_damage, BonusType::MassiveDamage(_));
        check_bonus!(self.bonus_strong_defense, BonusType::StrongDefense(_));
        check_bonus!(self.bonus_resist, BonusType::Resist(_));

        if self.target_crowns.active && (stage.max_crowns as i64) < self.target_crowns.min {
            return false;
        }

        let mut actual_hp = stage.base_hp as i64;
        let target_crown = self.target_crowns.min.max(1) as u32;

        if stage.anim_base_id != 0 && target_crown > 1 {
            let magnification = match target_crown {
                2 => map.crown_2_mag.unwrap_or(100),
                3 => map.crown_3_mag.unwrap_or(100),
                4 => map.crown_4_mag.unwrap_or(100),
                _ => 100,
            };
            actual_hp = (actual_hp * magnification as i64) / 100;
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

        if !self.category_name.is_empty() && !cat_name.to_lowercase().contains(&self.category_name) {
            return false;
        }

        if !self.map_name.is_empty() && !map.name.to_lowercase().contains(&self.map_name) {
            return false;
        }

        if !self.stage_name.is_empty() && !stage.name.to_lowercase().contains(&self.stage_name) {
            return false;
        }

        if !self.enemies.is_empty() {
            for enemy_filter in &self.enemies {
                let found = stage.enemies.iter().any(|enemy| enemy_filter.matches(enemy, enemy_name_registry));
                if enemy_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.treasures.is_empty() {
            for treasure_filter in &self.treasures {
                let found = match &stage.rewards {
                    RewardStructure::Treasure { drops, .. } => {
                        drops.iter().any(|drop| treasure_filter.matches_drop(drop.item_id, drop.amount, drop.chance, item_buy_registry, item_name_registry, drop_chara_registry, unit_buy_registry, cat_name_registry))
                    }
                    RewardStructure::Timed(scores) => {
                        scores.iter().any(|score| treasure_filter.matches_drop(score.item_id, score.amount, 100, item_buy_registry, item_name_registry, drop_chara_registry, unit_buy_registry, cat_name_registry))
                    }
                    _ => false,
                };
                if treasure_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.materials.is_empty() {
            let drop_items_opt = map.drop_items.as_ref();
            for material_filter in &self.materials {
                let found = drop_items_opt.map_or(false, |drop_items| {
                    drop_items.material_drops.iter().enumerate().any(|(index, &amount)| {
                        amount > 0 && material_filter.matches_material(index, amount, item_buy_registry, item_name_registry)
                    })
                });
                if material_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.lineup_cats.is_empty() {
            for lineup_filter in &self.lineup_cats {
                if !lineup_filter.matches_lineup(stage, cat_name_registry) { return false; }
            }
        }

        true
    }
}