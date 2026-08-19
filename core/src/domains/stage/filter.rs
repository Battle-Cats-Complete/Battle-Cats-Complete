pub mod enemy;
pub mod lineup;
pub mod material;
pub mod range;
pub mod treasure;

use std::collections::HashMap;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::map::{BonusType, LockSkipDataEntry, RuleType};
use nyanko::chapter::stage::{RewardStructure, ScatCpuSetting};
use nyanko::chapter::{Map, Stage};
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;

use super::StageDataState;

use self::enemy::{CompiledEnemyFilter, EnemyFilter};
use self::lineup::{CompiledLineupFilter, LineupFilter};
use self::material::{CompiledMaterialFilter, MaterialFilter};
use self::range::{CompiledStatRange, StatRange};
use self::treasure::{CompiledTreasureFilter, TreasureFilter};

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

pub struct StageLookupContext<'a> {
    pub enemy_name_registry: &'a [String],
    pub lock_registry: &'a HashMap<u32, LockSkipDataEntry>,
    pub cpu_setting: &'a ScatCpuSetting,
    pub item_buy_registry: &'a HashMap<u32, GatyaItemBuy>,
    pub item_name_registry: &'a HashMap<usize, GatyaItemName>,
    pub drop_chara_registry: &'a HashMap<u32, u32>,
    pub unit_buy_registry: &'a HashMap<u32, UnitBuy>,
    pub cat_name_registry: &'a HashMap<u32, Vec<String>>,
}

impl<'a> StageLookupContext<'a> {
    pub fn from_data(data: &'a StageDataState) -> Self {
        Self {
            enemy_name_registry: &data.enemy_name_registry,
            lock_registry: &data.lock_skip_registry,
            cpu_setting: &data.scat_cpu_setting,
            item_buy_registry: &data.item_buy_registry,
            item_name_registry: &data.item_name_registry,
            drop_chara_registry: &data.drop_chara_registry,
            unit_buy_registry: &data.unit_buy_registry,
            cat_name_registry: &data.cat_name_registry,
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

    pub fn matches(&self, cat_name: &str, map: &Map, stage: &Stage, ctx: &StageLookupContext) -> bool {
        if !self.is_active() { return true; }

        if let Some(has_continues) = self.continues && stage.is_no_continues == has_continues { return false; }
        if let Some(has_boss_guard) = self.boss_guard && stage.is_base_indestructible != has_boss_guard { return false; }

        if let Some(use_super_cpu) = self.use_super_cpu {
            let mut cpu_allowed = ctx.cpu_setting.super_cpu_consume_amount > 0;
            if let Some(global_id) = stage.category.global_map_id(stage.map_id)
                && let Some(entry) = ctx.lock_registry.get(&global_id)
                && entry.excluded_map_id == global_id { cpu_allowed = false; }
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
        let target_crown = self.target_crowns.min.clamp(1, i64::from(u8::MAX)) as u8;

        if stage.anim_base_id != 0 && target_crown > 1 {
            let magnification = stage.crown_magnification(target_crown).unwrap_or(100);
            actual_hp = (actual_hp * magnification as i64) / 100;
        }

        if !self.base_hp.matches(actual_hp) { return false; }
        if !self.anim_base_id.matches(stage.anim_base_id as i64) { return false; }
        if !self.width.matches(stage.width as i64) { return false; }
        if !self.max_enemies.matches(stage.max_enemies as i64) { return false; }
        if !self.time_limit.matches(stage.time_limit as i64) { return false; }
        if !self.energy.matches(stage.cost as i64) { return false; }
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
                let found = stage.enemies.iter().any(|enemy| enemy_filter.matches(enemy, ctx.enemy_name_registry));
                if enemy_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.treasures.is_empty() {
            for treasure_filter in &self.treasures {
                let found = match &stage.rewards {
                    RewardStructure::Treasure { drops, .. } => {
                        drops.iter().any(|drop| treasure_filter.matches_drop(drop.item_id, drop.amount, drop.chance, ctx.item_buy_registry, ctx.item_name_registry, ctx.drop_chara_registry, ctx.unit_buy_registry, ctx.cat_name_registry))
                    }
                    RewardStructure::Timed(scores) => {
                        scores.iter().any(|score| treasure_filter.matches_drop(score.item_id, score.amount, 100, ctx.item_buy_registry, ctx.item_name_registry, ctx.drop_chara_registry, ctx.unit_buy_registry, ctx.cat_name_registry))
                    }
                    _ => false,
                };
                if treasure_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.materials.is_empty() {
            let drop_items_opt = map.drop_items.as_ref();
            for material_filter in &self.materials {
                let found = drop_items_opt.is_some_and(|drop_items| {
                    drop_items.material_drops.iter().enumerate().any(|(index, &amount)| {
                        amount > 0 && material_filter.matches_material(index, amount, ctx.item_buy_registry, ctx.item_name_registry)
                    })
                });
                if material_filter.is_exclude { if found { return false; } } else if !found { return false; }
            }
        }

        if !self.lineup_cats.is_empty() {
            for lineup_filter in &self.lineup_cats {
                if !lineup_filter.matches_lineup(stage, ctx.cat_name_registry) { return false; }
            }
        }

        true
    }
}