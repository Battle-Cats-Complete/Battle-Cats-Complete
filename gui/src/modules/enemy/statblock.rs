use core::modules::enemy::logic::abilities::collect_ability_data;
use core::modules::enemy::logic::context::EnemyRenderContext;
use core::modules::enemy::logic::scanner::EnemyEntry;
use core::modules::enemy::registry::{format_enemy_stat, get_enemy_stat};

use crate::modules::statblock::builder::{StatCell, StatblockData};

pub(crate) fn build_enemy_statblock(
    ctx: &EnemyRenderContext,
    enemy_entry: &EnemyEntry,
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let frames = enemy_entry.atk_anim_frames;
    let cycle = (get_enemy_stat("Atk Cycle").get_value)(ctx.stats, frames, ctx.magnification);

    let top_val_str = if ctx.magnification.hitpoints == ctx.magnification.attack {
        format!("{}%", ctx.magnification.hitpoints)
    } else {
        format!("{}%/{}%", ctx.magnification.hitpoints, ctx.magnification.attack)
    };

    let headers_1 = vec![
        get_enemy_stat("Attack").display_name.to_string(),
        get_enemy_stat("Dps").display_name.to_string(),
        get_enemy_stat("Range").display_name.to_string(),
        get_enemy_stat("Atk Cycle").display_name.to_string(),
    ];

    let data_1 = vec![
        StatCell::Text(format_enemy_stat("Attack", ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat("Dps", ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat("Range", ctx.stats, frames, ctx.magnification)),
        StatCell::Frames(cycle),
    ];

    let headers_2 = vec![
        get_enemy_stat("Hitpoints").display_name.to_string(),
        get_enemy_stat("Knockbacks").display_name.to_string(),
        get_enemy_stat("Speed").display_name.to_string(),
        get_enemy_stat("Cash Drop").display_name.to_string(),
    ];

    let data_2 = vec![
        StatCell::Text(format_enemy_stat("Hitpoints", ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat("Knockbacks", ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat("Speed", ctx.stats, frames, ctx.magnification)),
        StatCell::Text(format_enemy_stat("Cash Drop", ctx.stats, frames, ctx.magnification)),
    ];

    StatblockData {
        is_cat: false,
        id_str: enemy_entry.id_str(),
        name: enemy_entry.display_name(),
        icon_path: enemy_entry.icon_path.clone(),
        top_label: "Magnification:".to_string(),
        top_value: top_val_str,
        headers_1,
        data_1,
        headers_2,
        data_2,
        traits, h1, h2, b1, b2, footer, spirit_data: None,
    }
}