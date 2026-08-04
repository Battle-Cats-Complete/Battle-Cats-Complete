use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, pick_list, row, scrollable, stack, text, text_input, Space};
use iced::{Element, Length, Size, Theme};

use core::modules::stage::filter::enemy::EnemyFilter;
use core::modules::stage::filter::lineup::LineupFilter;
use core::modules::stage::filter::material::MaterialFilter;
use core::modules::stage::filter::range::StatRange;
use core::modules::stage::filter::treasure::TreasureFilter;
use core::modules::stage::filter::StageFilterState;

use crate::app::theme;
use crate::widget::{popup, range_row, section};


const POPUP_SIZE: Size = Size::new(720.0, 528.0);
const CLEAR_BTN_CLEARANCE: f32 = 56.0;
const CONTENT_PADDING: f32 = 20.0;
const SCROLLBAR_GAP: f32 = 2.0;
const SECTION_SPACING: f32 = 12.0;
const CARD_SPACING: f32 = 8.0;
const FIELD_SPACING: f32 = 8.0;
const PAIR_SPACING: f32 = 16.0;
const CONTROL_TEXT_SIZE: f32 = 13.0;
const TRISTATE_WIDTH: f32 = 60.0;
const FLAG_LABEL_WIDTH: f32 = 130.0;
const FLAG_COLUMNS: usize = 3;
const REMOVE_BTN_WIDTH: f32 = 28.0;
const LABEL_WIDTH: f32 = 90.0;
const NAME_LABEL_WIDTH: f32 = 70.0;
const NAME_INPUT_WIDTH: f32 = 320.0;
const VALUE_INPUT_WIDTH: f32 = 260.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Continues,
    BossGuard,
    UseSuperCpu,
    RuleTrustFund,
    RuleCooldownEquality,
    RuleRarityLimit,
    RuleCheapLabor,
    RuleCatCost,
    RuleCatProduction,
    RuleTotalDeployLimit,
    RuleMoreThanOne,
    RuleMegaCatCannon,
    RuleUniformMotion,
    InvalidCombos,
    BonusWeaken,
    BonusFreeze,
    BonusSlow,
    BonusKnockback,
    BonusStrongAttack,
    BonusMassiveDamage,
    BonusStrongDefense,
    BonusResist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Width,
    BaseHp,
    MaxEnemies,
    TimeLimit,
    Energy,
    Xp,
    MinSpawn,
    MaxSpawn,
    Difficulty,
    MaxCrowns,
    TargetCrowns,
    MinCost,
    MaxCost,
    DeployLimit,
    AllowedRows,
    BaseId,
    AnimBaseId,
    BackgroundId,
    InitTrack,
    BossTrack,
    BgmChangePercent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyRange {
    Amount,
    StartFrame,
    RespawnMin,
    RespawnMax,
    BaseHpPerc,
    LayerMin,
    LayerMax,
    Magnification,
    AtkMagnification,
    Score,
    TimeFlag,
    KillCount,
}

fn flag_mut(state: &mut StageFilterState, flag: Flag) -> &mut Option<bool> {
    match flag {
        Flag::Continues => &mut state.continues,
        Flag::BossGuard => &mut state.boss_guard,
        Flag::UseSuperCpu => &mut state.use_super_cpu,
        Flag::RuleTrustFund => &mut state.rule_trust_fund,
        Flag::RuleCooldownEquality => &mut state.rule_cooldown_equality,
        Flag::RuleRarityLimit => &mut state.rule_rarity_limit,
        Flag::RuleCheapLabor => &mut state.rule_cheap_labor,
        Flag::RuleCatCost => &mut state.rule_cat_cost,
        Flag::RuleCatProduction => &mut state.rule_cat_production,
        Flag::RuleTotalDeployLimit => &mut state.rule_total_deploy_limit,
        Flag::RuleMoreThanOne => &mut state.rule_more_than_one,
        Flag::RuleMegaCatCannon => &mut state.rule_mega_cat_cannon,
        Flag::RuleUniformMotion => &mut state.rule_uniform_motion,
        Flag::InvalidCombos => &mut state.invalid_combos,
        Flag::BonusWeaken => &mut state.bonus_weaken,
        Flag::BonusFreeze => &mut state.bonus_freeze,
        Flag::BonusSlow => &mut state.bonus_slow,
        Flag::BonusKnockback => &mut state.bonus_knockback,
        Flag::BonusStrongAttack => &mut state.bonus_strong_attack,
        Flag::BonusMassiveDamage => &mut state.bonus_massive_damage,
        Flag::BonusStrongDefense => &mut state.bonus_strong_defense,
        Flag::BonusResist => &mut state.bonus_resist,
    }
}

fn flag_label(flag: Flag) -> &'static str {
    match flag {
        Flag::Continues => "Continues",
        Flag::BossGuard => "Boss Guard",
        Flag::UseSuperCpu => "Use Super CPU",
        Flag::RuleTrustFund => "Trust Fund",
        Flag::RuleCooldownEquality => "Cooldown Equality",
        Flag::RuleRarityLimit => "Rarity Limit",
        Flag::RuleCheapLabor => "Cheap Labor",
        Flag::RuleCatCost => "Cat Cost",
        Flag::RuleCatProduction => "Cat Production",
        Flag::RuleTotalDeployLimit => "Total Deploy Limit",
        Flag::RuleMoreThanOne => "More Than One",
        Flag::RuleMegaCatCannon => "Mega Cat Cannon",
        Flag::RuleUniformMotion => "Uniform Motion",
        Flag::InvalidCombos => "Invalid Combos",
        Flag::BonusWeaken => "Weaken",
        Flag::BonusFreeze => "Freeze",
        Flag::BonusSlow => "Slow",
        Flag::BonusKnockback => "Knockback",
        Flag::BonusStrongAttack => "Strong Attack",
        Flag::BonusMassiveDamage => "Massive Damage",
        Flag::BonusStrongDefense => "Strong Defense",
        Flag::BonusResist => "Resist",
    }
}

fn flag_ref(state: &StageFilterState, flag: Flag) -> Option<bool> {
    match flag {
        Flag::Continues => state.continues,
        Flag::BossGuard => state.boss_guard,
        Flag::UseSuperCpu => state.use_super_cpu,
        Flag::RuleTrustFund => state.rule_trust_fund,
        Flag::RuleCooldownEquality => state.rule_cooldown_equality,
        Flag::RuleRarityLimit => state.rule_rarity_limit,
        Flag::RuleCheapLabor => state.rule_cheap_labor,
        Flag::RuleCatCost => state.rule_cat_cost,
        Flag::RuleCatProduction => state.rule_cat_production,
        Flag::RuleTotalDeployLimit => state.rule_total_deploy_limit,
        Flag::RuleMoreThanOne => state.rule_more_than_one,
        Flag::RuleMegaCatCannon => state.rule_mega_cat_cannon,
        Flag::RuleUniformMotion => state.rule_uniform_motion,
        Flag::InvalidCombos => state.invalid_combos,
        Flag::BonusWeaken => state.bonus_weaken,
        Flag::BonusFreeze => state.bonus_freeze,
        Flag::BonusSlow => state.bonus_slow,
        Flag::BonusKnockback => state.bonus_knockback,
        Flag::BonusStrongAttack => state.bonus_strong_attack,
        Flag::BonusMassiveDamage => state.bonus_massive_damage,
        Flag::BonusStrongDefense => state.bonus_strong_defense,
        Flag::BonusResist => state.bonus_resist,
    }
}

fn range_mut(state: &mut StageFilterState, range: Range) -> &mut StatRange {
    match range {
        Range::Width => &mut state.width,
        Range::BaseHp => &mut state.base_hp,
        Range::MaxEnemies => &mut state.max_enemies,
        Range::TimeLimit => &mut state.time_limit,
        Range::Energy => &mut state.energy,
        Range::Xp => &mut state.xp,
        Range::MinSpawn => &mut state.min_spawn,
        Range::MaxSpawn => &mut state.max_spawn,
        Range::Difficulty => &mut state.difficulty,
        Range::MaxCrowns => &mut state.max_crowns,
        Range::TargetCrowns => &mut state.target_crowns,
        Range::MinCost => &mut state.min_cost,
        Range::MaxCost => &mut state.max_cost,
        Range::DeployLimit => &mut state.deploy_limit,
        Range::AllowedRows => &mut state.allowed_rows,
        Range::BaseId => &mut state.base_id,
        Range::AnimBaseId => &mut state.anim_base_id,
        Range::BackgroundId => &mut state.background_id,
        Range::InitTrack => &mut state.init_track,
        Range::BossTrack => &mut state.boss_track,
        Range::BgmChangePercent => &mut state.bgm_change_percent,
    }
}

fn range_label(range: Range) -> &'static str {
    match range {
        Range::Width => "Width",
        Range::BaseHp => "Base HP",
        Range::MaxEnemies => "Max Enemies",
        Range::TimeLimit => "Time Limit (f)",
        Range::Energy => "Energy Cost",
        Range::Xp => "XP Reward",
        Range::MinSpawn => "Min Spawn",
        Range::MaxSpawn => "Max Spawn",
        Range::Difficulty => "Difficulty",
        Range::MaxCrowns => "Max Crowns",
        Range::TargetCrowns => "Target Crowns",
        Range::MinCost => "Min Cost",
        Range::MaxCost => "Max Cost",
        Range::DeployLimit => "Deploy Limit",
        Range::AllowedRows => "Allowed Rows",
        Range::BaseId => "Base ID",
        Range::AnimBaseId => "Anim Base ID",
        Range::BackgroundId => "Background ID",
        Range::InitTrack => "Init Track",
        Range::BossTrack => "Boss Track",
        Range::BgmChangePercent => "BGM Change (%)",
    }
}

fn enemy_range_mut(filter: &mut EnemyFilter, range: EnemyRange) -> &mut StatRange {
    match range {
        EnemyRange::Amount => &mut filter.amount,
        EnemyRange::StartFrame => &mut filter.start_frame,
        EnemyRange::RespawnMin => &mut filter.respawn_min,
        EnemyRange::RespawnMax => &mut filter.respawn_max,
        EnemyRange::BaseHpPerc => &mut filter.base_hp_perc,
        EnemyRange::LayerMin => &mut filter.layer_min,
        EnemyRange::LayerMax => &mut filter.layer_max,
        EnemyRange::Magnification => &mut filter.magnification,
        EnemyRange::AtkMagnification => &mut filter.atk_magnification,
        EnemyRange::Score => &mut filter.score,
        EnemyRange::TimeFlag => &mut filter.time_flag,
        EnemyRange::KillCount => &mut filter.kill_count,
    }
}

fn enemy_range_label(range: EnemyRange) -> &'static str {
    match range {
        EnemyRange::Amount => "Amount",
        EnemyRange::StartFrame => "Start Frame",
        EnemyRange::RespawnMin => "Respawn Min",
        EnemyRange::RespawnMax => "Respawn Max",
        EnemyRange::BaseHpPerc => "Base HP (%)",
        EnemyRange::LayerMin => "Layer Min",
        EnemyRange::LayerMax => "Layer Max",
        EnemyRange::Magnification => "Magnification",
        EnemyRange::AtkMagnification => "Atk Mag",
        EnemyRange::Score => "Score",
        EnemyRange::TimeFlag => "Time Flag",
        EnemyRange::KillCount => "Kill Count",
    }
}

const ENEMY_RANGES: [EnemyRange; 12] = [
    EnemyRange::Amount, EnemyRange::StartFrame, EnemyRange::RespawnMin, EnemyRange::RespawnMax,
    EnemyRange::BaseHpPerc, EnemyRange::LayerMin, EnemyRange::LayerMax, EnemyRange::Magnification,
    EnemyRange::AtkMagnification, EnemyRange::Score, EnemyRange::TimeFlag, EnemyRange::KillCount,
];

fn cycle_tristate(value: Option<bool>) -> Option<bool> {
    match value {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Toggle,
    Clear,

    CategoryChanged(String),
    MapChanged(String),
    StageChanged(String),

    FlagToggled(Flag),
    RangeMinChanged(Range, String),
    RangeMaxChanged(Range, String),

    AddEnemy,
    RemoveEnemy(usize),
    EnemyModeChanged(usize, bool),
    EnemyNameChanged(usize, String),
    EnemyBossTypeChanged(usize, Option<u32>),
    EnemyIsBaseToggled(usize),
    EnemyRangeMinChanged(usize, EnemyRange, String),
    EnemyRangeMaxChanged(usize, EnemyRange, String),

    AddLineup,
    RemoveLineup(usize),
    LineupModeChanged(usize, bool),
    LineupNameChanged(usize, String),
    LineupLevelMinChanged(usize, String),
    LineupLevelMaxChanged(usize, String),

    AddTreasure,
    RemoveTreasure(usize),
    TreasureModeChanged(usize, bool),
    TreasureNameChanged(usize, String),
    TreasureAmountMinChanged(usize, String),
    TreasureAmountMaxChanged(usize, String),
    TreasureChanceMinChanged(usize, String),
    TreasureChanceMaxChanged(usize, String),

    AddMaterial,
    RemoveMaterial(usize),
    MaterialModeChanged(usize, bool),
    MaterialNameChanged(usize, String),
    MaterialAmountMinChanged(usize, String),
    MaterialAmountMaxChanged(usize, String),
}

#[derive(Default)]
pub struct State {
    pub filter_state: StageFilterState,
    popup: popup::State,
}

impl State {
    pub fn update(&mut self, message: Message) {
        let state = &mut self.filter_state;

        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    state.is_open = false;
                }
            }
            Message::Toggle => state.is_open = !state.is_open,
            Message::Clear => *state = StageFilterState { is_open: state.is_open, ..Default::default() },

            Message::CategoryChanged(value) => state.category_name = value,
            Message::MapChanged(value) => state.map_name = value,
            Message::StageChanged(value) => state.stage_name = value,

            Message::FlagToggled(flag) => {
                let target = flag_mut(state, flag);
                *target = cycle_tristate(*target);
            }
            Message::RangeMinChanged(range, value) => range_mut(state, range).min = value,
            Message::RangeMaxChanged(range, value) => range_mut(state, range).max = value,

            Message::AddEnemy => state.enemies.push(EnemyFilter::default()),
            Message::RemoveEnemy(idx) => { if idx < state.enemies.len() { state.enemies.remove(idx); } }
            Message::EnemyModeChanged(idx, is_exclude) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy.is_exclude = is_exclude; }
            }
            Message::EnemyNameChanged(idx, value) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy.name_or_id = value; }
            }
            Message::EnemyBossTypeChanged(idx, value) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy.boss_type = value; }
            }
            Message::EnemyIsBaseToggled(idx) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy.is_base = cycle_tristate(enemy.is_base); }
            }
            Message::EnemyRangeMinChanged(idx, range, value) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy_range_mut(enemy, range).min = value; }
            }
            Message::EnemyRangeMaxChanged(idx, range, value) => {
                if let Some(enemy) = state.enemies.get_mut(idx) { enemy_range_mut(enemy, range).max = value; }
            }

            Message::AddLineup => state.lineup_cats.push(LineupFilter::default()),
            Message::RemoveLineup(idx) => { if idx < state.lineup_cats.len() { state.lineup_cats.remove(idx); } }
            Message::LineupModeChanged(idx, is_exclude) => {
                if let Some(cat) = state.lineup_cats.get_mut(idx) { cat.is_exclude = is_exclude; }
            }
            Message::LineupNameChanged(idx, value) => {
                if let Some(cat) = state.lineup_cats.get_mut(idx) { cat.name_or_id = value; }
            }
            Message::LineupLevelMinChanged(idx, value) => {
                if let Some(cat) = state.lineup_cats.get_mut(idx) { cat.level.min = value; }
            }
            Message::LineupLevelMaxChanged(idx, value) => {
                if let Some(cat) = state.lineup_cats.get_mut(idx) { cat.level.max = value; }
            }

            Message::AddTreasure => state.treasures.push(TreasureFilter::default()),
            Message::RemoveTreasure(idx) => { if idx < state.treasures.len() { state.treasures.remove(idx); } }
            Message::TreasureModeChanged(idx, is_exclude) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.is_exclude = is_exclude; }
            }
            Message::TreasureNameChanged(idx, value) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.name_or_id = value; }
            }
            Message::TreasureAmountMinChanged(idx, value) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.amount.min = value; }
            }
            Message::TreasureAmountMaxChanged(idx, value) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.amount.max = value; }
            }
            Message::TreasureChanceMinChanged(idx, value) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.chance.min = value; }
            }
            Message::TreasureChanceMaxChanged(idx, value) => {
                if let Some(treasure) = state.treasures.get_mut(idx) { treasure.chance.max = value; }
            }

            Message::AddMaterial => state.materials.push(MaterialFilter::default()),
            Message::RemoveMaterial(idx) => { if idx < state.materials.len() { state.materials.remove(idx); } }
            Message::MaterialModeChanged(idx, is_exclude) => {
                if let Some(material) = state.materials.get_mut(idx) { material.is_exclude = is_exclude; }
            }
            Message::MaterialNameChanged(idx, value) => {
                if let Some(material) = state.materials.get_mut(idx) { material.name_or_id = value; }
            }
            Message::MaterialAmountMinChanged(idx, value) => {
                if let Some(material) = state.materials.get_mut(idx) { material.amount.min = value; }
            }
            Message::MaterialAmountMaxChanged(idx, value) => {
                if let Some(material) = state.materials.get_mut(idx) { material.amount.max = value; }
            }
        }
    }

    pub fn view(&self, window: Size) -> Element<'_, Message> {
        self.popup.view("Advanced Stage Filter", POPUP_SIZE, window, Message::Popup, move || self.content_view())
    }

    fn content_view(&self) -> Element<'_, Message> {
        let name_grid = column![
            name_field("Category:", &self.filter_state.category_name, Message::CategoryChanged),
            name_field("Map:", &self.filter_state.map_name, Message::MapChanged),
            name_field("Stage:", &self.filter_state.stage_name, Message::StageChanged),
        ].spacing(6);

        let general_rules = wrap_pairs([Flag::Continues, Flag::BossGuard, Flag::UseSuperCpu], &self.filter_state);

        let special_rules = wrap_pairs([
            Flag::RuleTrustFund, Flag::RuleCooldownEquality, Flag::RuleRarityLimit, Flag::RuleCheapLabor,
            Flag::RuleCatCost, Flag::RuleCatProduction, Flag::RuleTotalDeployLimit, Flag::RuleMoreThanOne,
            Flag::RuleMegaCatCannon, Flag::RuleUniformMotion, Flag::InvalidCombos,
        ], &self.filter_state);

        let score_bonuses = wrap_pairs([
            Flag::BonusWeaken, Flag::BonusFreeze, Flag::BonusSlow, Flag::BonusKnockback,
            Flag::BonusStrongAttack, Flag::BonusMassiveDamage, Flag::BonusStrongDefense, Flag::BonusResist,
        ], &self.filter_state);

        let stats = range_pairs([
            Range::BaseHp, Range::Width, Range::TimeLimit, Range::MaxEnemies, Range::Energy, Range::Xp,
            Range::Difficulty, Range::MaxCrowns, Range::TargetCrowns, Range::MinSpawn, Range::MaxSpawn,
        ], &self.filter_state);

        let restrictions = range_pairs([
            Range::DeployLimit, Range::AllowedRows, Range::MinCost, Range::MaxCost,
        ], &self.filter_state);

        let ids_audio = range_pairs([
            Range::BaseId, Range::AnimBaseId, Range::BackgroundId, Range::InitTrack, Range::BossTrack, Range::BgmChangePercent,
        ], &self.filter_state);

        let mut enemies_col = column![].spacing(8);
        for (idx, enemy) in self.filter_state.enemies.iter().enumerate() {
            enemies_col = enemies_col.push(enemy_card(idx, enemy));
        }
        enemies_col = enemies_col.push(add_button("+ Add New Enemy", Message::AddEnemy));

        let mut lineup_col = column![].spacing(CARD_SPACING);
        for (idx, cat) in self.filter_state.lineup_cats.iter().enumerate() {
            lineup_col = lineup_col.push(lineup_card(idx, cat));
        }
        lineup_col = lineup_col.push(add_button("+ Add Lineup Cat", Message::AddLineup));

        let mut treasure_col = column![].spacing(CARD_SPACING);
        for (idx, treasure) in self.filter_state.treasures.iter().enumerate() {
            treasure_col = treasure_col.push(treasure_card(idx, treasure));
        }
        treasure_col = treasure_col.push(add_button("+ Add New Treasure", Message::AddTreasure));

        let mut material_col = column![].spacing(CARD_SPACING);
        for (idx, material) in self.filter_state.materials.iter().enumerate() {
            material_col = material_col.push(material_card(idx, material));
        }
        material_col = material_col.push(add_button("+ Add New Material", Message::AddMaterial));

        let content = column![
            section("Name", Length::Fill, name_grid),
            section("General Rules", Length::Fill, general_rules),
            section("Special Map Rules", Length::Fill, special_rules),
            section("Score Bonuses", Length::Fill, score_bonuses),
            section("Stats", Length::Fill, stats),
            section("Restrictions", Length::Fill, restrictions),
            section("IDs & Audio", Length::Fill, ids_audio),
            section("Battleground", Length::Fill, enemies_col),
            section("Fixed Lineup Cats", Length::Fill, lineup_col),
            section("Treasures", Length::Fill, treasure_col),
            section("Materials", Length::Fill, material_col),
            Space::new().height(Length::Fixed(CLEAR_BTN_CLEARANCE)),
        ].spacing(SECTION_SPACING).padding(CONTENT_PADDING);

        let scroll_layer = scrollable(content).width(Length::Fill).height(Length::Fill).spacing(SCROLLBAR_GAP);

        let clear_btn_layer = container(
            button(text("Clear Filter")).on_press(Message::Clear).padding([8, 16]).style(button::danger)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Bottom)
            .padding(16);

        stack![scroll_layer, clear_btn_layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn name_field<'a>(label: &'a str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(NAME_LABEL_WIDTH)),
        text_input("Any", value).on_input(on_input).width(Length::Fixed(NAME_INPUT_WIDTH)).style(theme::rounded_input),
    ].spacing(FIELD_SPACING).align_y(Vertical::Center).into()
}

fn add_button<'a>(label: &'a str, on_press: Message) -> Element<'a, Message> {
    button(text(label).size(CONTROL_TEXT_SIZE))
        .on_press(on_press)
        .padding([6, 12])
        .style(theme::primary_button)
        .into()
}

fn tristate_button<'a>(value: Option<bool>, on_press: Message) -> Element<'a, Message> {
    let label = match value {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Any",
    };

    button(theme::button_label(label).size(CONTROL_TEXT_SIZE))
        .width(Length::Fixed(TRISTATE_WIDTH))
        .on_press(on_press)
        .style(move |theme: &Theme, status| match value {
            Some(true) => theme::primary_button(theme, status),
            Some(false) => theme::danger_button(theme, status),
            None => theme::neutral_button(theme, status),
        })
        .into()
}

fn wrap_pairs<'a>(flags: impl IntoIterator<Item = Flag>, filter_state: &'a StageFilterState) -> Element<'a, Message> {
    let mut col = column![].spacing(6);
    let mut current = row![].spacing(PAIR_SPACING).align_y(Vertical::Center);
    let mut count = 0;

    for flag in flags {
        let value = flag_ref(filter_state, flag);
        current = current.push(
            row![
                text(format!("{}:", flag_label(flag))).size(CONTROL_TEXT_SIZE).width(Length::Fixed(FLAG_LABEL_WIDTH)),
                tristate_button(value, Message::FlagToggled(flag)),
            ]
                .spacing(FIELD_SPACING)
                .align_y(Vertical::Center)
        );
        count += 1;

        if count % FLAG_COLUMNS == 0 {
            col = col.push(current);
            current = row![].spacing(PAIR_SPACING).align_y(Vertical::Center);
        }
    }

    if !count.is_multiple_of(FLAG_COLUMNS) {
        col = col.push(current);
    }

    col.into()
}

fn range_field<'a>(range: Range, value: &'a StatRange) -> Element<'a, Message> {
    range_row(
        range_label(range),
        &value.min,
        &value.max,
        move |v| Message::RangeMinChanged(range, v),
        move |v| Message::RangeMaxChanged(range, v),
    )
}

fn range_pairs<'a>(ranges: impl IntoIterator<Item = Range>, filter_state: &'a StageFilterState) -> Element<'a, Message> {
    let mut col = column![].spacing(6);
    let mut current = row![].spacing(PAIR_SPACING);
    let mut count = 0;

    for range in ranges {
        current = current.push(range_field(range, range_ref(filter_state, range)));
        count += 1;

        if count % 2 == 0 {
            col = col.push(current);
            current = row![].spacing(PAIR_SPACING);
        }
    }

    if count % 2 != 0 {
        col = col.push(current);
    }

    col.into()
}

fn range_ref(filter_state: &StageFilterState, range: Range) -> &StatRange {
    match range {
        Range::Width => &filter_state.width,
        Range::BaseHp => &filter_state.base_hp,
        Range::MaxEnemies => &filter_state.max_enemies,
        Range::TimeLimit => &filter_state.time_limit,
        Range::Energy => &filter_state.energy,
        Range::Xp => &filter_state.xp,
        Range::MinSpawn => &filter_state.min_spawn,
        Range::MaxSpawn => &filter_state.max_spawn,
        Range::Difficulty => &filter_state.difficulty,
        Range::MaxCrowns => &filter_state.max_crowns,
        Range::TargetCrowns => &filter_state.target_crowns,
        Range::MinCost => &filter_state.min_cost,
        Range::MaxCost => &filter_state.max_cost,
        Range::DeployLimit => &filter_state.deploy_limit,
        Range::AllowedRows => &filter_state.allowed_rows,
        Range::BaseId => &filter_state.base_id,
        Range::AnimBaseId => &filter_state.anim_base_id,
        Range::BackgroundId => &filter_state.background_id,
        Range::InitTrack => &filter_state.init_track,
        Range::BossTrack => &filter_state.boss_track,
        Range::BgmChangePercent => &filter_state.bgm_change_percent,
    }
}

fn card_frame<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content.into())
        .padding(8)
        .style(theme::card_container)
        .into()
}

fn mode_pick_list<'a>(is_exclude: bool, on_change: impl Fn(bool) -> Message + 'a) -> Element<'a, Message> {
    let label = if is_exclude { "Exclude" } else { "Include" };

    pick_list(vec!["Include", "Exclude"], Some(label), move |s| on_change(s == "Exclude"))
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu)
        .into()
}

fn close_row<'a>(mode: Element<'a, Message>, on_remove: Message) -> Element<'a, Message> {
    row![
        mode,
        Space::new().width(Length::Fill),
        button(theme::button_label("X").size(CONTROL_TEXT_SIZE))
            .width(Length::Fixed(REMOVE_BTN_WIDTH))
            .on_press(on_remove)
            .style(theme::danger_button),
    ].align_y(Vertical::Center).into()
}

fn name_or_id_row<'a>(value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    row![
        text("Name or ID:").width(Length::Fixed(LABEL_WIDTH)),
        text_input("Any", value).on_input(on_input).width(Length::Fixed(VALUE_INPUT_WIDTH)).style(theme::rounded_input),
    ].spacing(FIELD_SPACING).align_y(Vertical::Center).into()
}

fn enemy_card<'a>(idx: usize, enemy: &'a EnemyFilter) -> Element<'a, Message> {
    let mode = mode_pick_list(enemy.is_exclude, move |is_exclude| Message::EnemyModeChanged(idx, is_exclude));

    let mut ranges_col = column![].spacing(4);
    let mut current = row![].spacing(PAIR_SPACING);
    for (i, &range) in ENEMY_RANGES.iter().enumerate() {
        let value = enemy_range_ref(enemy, range);
        current = current.push(range_row(
            enemy_range_label(range),
            &value.min,
            &value.max,
            move |v| Message::EnemyRangeMinChanged(idx, range, v),
            move |v| Message::EnemyRangeMaxChanged(idx, range, v),
        ));
        if (i + 1) % 2 == 0 {
            ranges_col = ranges_col.push(current);
            current = row![].spacing(PAIR_SPACING);
        }
    }
    if !ENEMY_RANGES.len().is_multiple_of(2) {
        ranges_col = ranges_col.push(current);
    }

    let boss_type_label = match enemy.boss_type {
        None => "Any",
        Some(0) => "None",
        Some(1) => "Boss",
        Some(2) => "Screen Shake",
        Some(_) => "Unknown",
    };

    let body = column![
        close_row(mode, Message::RemoveEnemy(idx)),
        row![
            text("Boss Type:"),
            pick_list(vec!["Any", "None", "Boss", "Screen Shake"], Some(boss_type_label), move |s| {
                Message::EnemyBossTypeChanged(idx, match s { "None" => Some(0), "Boss" => Some(1), "Screen Shake" => Some(2), _ => None })
            })
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
            text("Is Base:"),
            tristate_button(enemy.is_base, Message::EnemyIsBaseToggled(idx)),
        ].spacing(10).align_y(Vertical::Center),
        name_or_id_row(&enemy.name_or_id, move |v| Message::EnemyNameChanged(idx, v)),
        ranges_col,
    ].spacing(CARD_SPACING);

    card_frame(body)
}

fn enemy_range_ref(enemy: &EnemyFilter, range: EnemyRange) -> &StatRange {
    match range {
        EnemyRange::Amount => &enemy.amount,
        EnemyRange::StartFrame => &enemy.start_frame,
        EnemyRange::RespawnMin => &enemy.respawn_min,
        EnemyRange::RespawnMax => &enemy.respawn_max,
        EnemyRange::BaseHpPerc => &enemy.base_hp_perc,
        EnemyRange::LayerMin => &enemy.layer_min,
        EnemyRange::LayerMax => &enemy.layer_max,
        EnemyRange::Magnification => &enemy.magnification,
        EnemyRange::AtkMagnification => &enemy.atk_magnification,
        EnemyRange::Score => &enemy.score,
        EnemyRange::TimeFlag => &enemy.time_flag,
        EnemyRange::KillCount => &enemy.kill_count,
    }
}

fn lineup_card<'a>(idx: usize, cat: &'a LineupFilter) -> Element<'a, Message> {
    let mode = mode_pick_list(cat.is_exclude, move |is_exclude| Message::LineupModeChanged(idx, is_exclude));

    let body = column![
        close_row(mode, Message::RemoveLineup(idx)),
        name_or_id_row(&cat.name_or_id, move |v| Message::LineupNameChanged(idx, v)),
        range_row(
            "Total Level",
            &cat.level.min,
            &cat.level.max,
            move |v| Message::LineupLevelMinChanged(idx, v),
            move |v| Message::LineupLevelMaxChanged(idx, v),
        ),
    ].spacing(CARD_SPACING);

    card_frame(body)
}

fn treasure_card<'a>(idx: usize, treasure: &'a TreasureFilter) -> Element<'a, Message> {
    let mode = mode_pick_list(treasure.is_exclude, move |is_exclude| Message::TreasureModeChanged(idx, is_exclude));

    let body = column![
        close_row(mode, Message::RemoveTreasure(idx)),
        name_or_id_row(&treasure.name_or_id, move |v| Message::TreasureNameChanged(idx, v)),
        range_row(
            "Amount",
            &treasure.amount.min,
            &treasure.amount.max,
            move |v| Message::TreasureAmountMinChanged(idx, v),
            move |v| Message::TreasureAmountMaxChanged(idx, v),
        ),
        range_row(
            "Chance (%)",
            &treasure.chance.min,
            &treasure.chance.max,
            move |v| Message::TreasureChanceMinChanged(idx, v),
            move |v| Message::TreasureChanceMaxChanged(idx, v),
        ),
    ].spacing(CARD_SPACING);

    card_frame(body)
}

fn material_card<'a>(idx: usize, material: &'a MaterialFilter) -> Element<'a, Message> {
    let mode = mode_pick_list(material.is_exclude, move |is_exclude| Message::MaterialModeChanged(idx, is_exclude));

    let body = column![
        close_row(mode, Message::RemoveMaterial(idx)),
        name_or_id_row(&material.name_or_id, move |v| Message::MaterialNameChanged(idx, v)),
        range_row(
            "Amount",
            &material.amount.min,
            &material.amount.max,
            move |v| Message::MaterialAmountMinChanged(idx, v),
            move |v| Message::MaterialAmountMaxChanged(idx, v),
        ),
    ].spacing(CARD_SPACING);

    card_frame(body)
}
