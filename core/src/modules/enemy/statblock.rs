use crate::common::context::GlobalContext;
use crate::statblock::{StatCell, StatblockData};
use crate::Vault;

use super::game::abilities::collect_ability_data;
use super::game::registry::{format_enemy_stat, Magnification, STAT_ATK_CYCLE, STAT_ATTACK, STAT_CASH_DROP, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_SPEED};
use super::game::EnemyRenderContext;
use super::scanner::{self, EnemyEntry};

pub struct Subject<'a> {
    pub enemy: &'a EnemyEntry,
    pub magnification: Magnification,
    pub show_invalid: bool,
    pub global: GlobalContext<'a>,
    pub vault: &'a Vault,
}

pub fn build(subject: Subject<'_>) -> StatblockData {
    let enemy = subject.enemy;
    let dynamic_entry = scanner::scan_single(enemy.id, subject.vault, subject.show_invalid);
    let stats = dynamic_entry.as_ref().map_or(&enemy.stats, |entry| &entry.stats);

    let ctx = EnemyRenderContext {
        global: subject.global,
        stats,
        magnification: subject.magnification,
    };

    assemble(&ctx, enemy)
}

fn assemble(ctx: &EnemyRenderContext, enemy_entry: &EnemyEntry) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let frames = enemy_entry.atk_anim_frames;
    let cycle = (STAT_ATK_CYCLE.get_value)(ctx.stats, frames, ctx.magnification);

    let top_val_str = if ctx.magnification.hitpoints == ctx.magnification.attack {
        format!("{}%", ctx.magnification.hitpoints)
    } else {
        format!("{}%/{}%", ctx.magnification.hitpoints, ctx.magnification.attack)
    };

    let headers_1 = vec![
        STAT_ATTACK.display_name.to_string(),
        STAT_DPS.display_name.to_string(),
        STAT_RANGE.display_name.to_string(),
        STAT_ATK_CYCLE.display_name.to_string(),
    ];

    let data_1 = vec![
        StatCell::Text(format_enemy_stat(&STAT_ATTACK, ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat(&STAT_DPS, ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat(&STAT_RANGE, ctx.stats, frames, ctx.magnification)),
        StatCell::Frames(cycle),
    ];

    let headers_2 = vec![
        STAT_HITPOINTS.display_name.to_string(),
        STAT_KNOCKBACKS.display_name.to_string(),
        STAT_SPEED.display_name.to_string(),
        STAT_CASH_DROP.display_name.to_string(),
    ];

    let data_2 = vec![
        StatCell::Text(format_enemy_stat(&STAT_HITPOINTS, ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat(&STAT_KNOCKBACKS, ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat(&STAT_SPEED, ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat(&STAT_CASH_DROP, ctx.stats, frames, ctx.magnification)),
    ];

    StatblockData {
        is_cat: false,
        id_str: enemy_entry.id_str(),
        name: enemy_entry.display_name(),
        icon_path: enemy_entry.icon_path.clone(),
        top_label: "Magnify:".to_string(),
        top_value: top_val_str,
        headers_1,
        data_1,
        headers_2,
        data_2,
        traits, h1, h2, b1, b2, footer, spirit_data: None,
    }
}
