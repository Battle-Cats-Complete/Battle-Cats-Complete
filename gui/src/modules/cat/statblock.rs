use core::modules::cat::game::abilities::collect_ability_data;
use core::modules::cat::game::registry::{format_cat_stat, get_cat_stat};
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::waiter::unitid;
use core::modules::settings::Settings;

use crate::modules::statblock::builder::{SpiritData, StatCell, StatblockData};

pub(crate) fn build_cat_statblock(
    ctx: &CatRenderContext,
    cat_entry: &CatEntry,
    current_form: usize,
    level_input: String,
    is_conjure_expanded: bool,
    settings: &Settings,
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let spirit_data = if is_conjure_expanded {
        build_spirit_data(ctx, settings)
    } else {
        None
    };

    let anim_frames = cat_entry.atk_anim_frames[current_form];
    let unitbuy_opt = Some(&cat_entry.unitbuy);
    let cycle = (get_cat_stat("Atk Cycle").get_value)(ctx.final_stats, anim_frames, unitbuy_opt);

    let headers_1 = vec![
        get_cat_stat("Attack").display_name.to_string(),
        get_cat_stat("Dps").display_name.to_string(),
        get_cat_stat("Range").display_name.to_string(),
        get_cat_stat("Atk Cycle").display_name.to_string(),
        get_cat_stat("Rarity").display_name.to_string(),
    ];

    let data_1 = vec![
        StatCell::Text(format_cat_stat("Attack", ctx.final_stats, anim_frames, unitbuy_opt)),
        StatCell::Text(format_cat_stat("Dps", ctx.final_stats, anim_frames, unitbuy_opt)),
        StatCell::Text(ctx.final_stats.standing_range.to_string()),
        StatCell::Frames(cycle),
        StatCell::Text(format_cat_stat("Rarity", ctx.final_stats, anim_frames, unitbuy_opt)),
    ];

    let headers_2 = vec![
        get_cat_stat("Hitpoints").display_name.to_string(),
        get_cat_stat("Knockbacks").display_name.to_string(),
        get_cat_stat("Speed").display_name.to_string(),
        get_cat_stat("Cooldown").display_name.to_string(),
        get_cat_stat("Cost").display_name.to_string(),
    ];

    let cd_frames = (get_cat_stat("Cooldown").get_value)(ctx.final_stats, anim_frames, unitbuy_opt);

    let data_2 = vec![
        StatCell::Text(ctx.final_stats.hitpoints.to_string()),
        StatCell::Text(ctx.final_stats.knockbacks.to_string()),
        StatCell::Text(ctx.final_stats.speed.to_string()),
        StatCell::Frames(cd_frames),
        StatCell::Text(format_cat_stat("Cost", ctx.final_stats, anim_frames, unitbuy_opt)),
    ];

    StatblockData {
        is_cat: true,
        id_str: cat_entry.id_str(current_form),
        name: cat_entry.display_name(current_form),
        icon_path: cat_entry.deploy_icon_paths[current_form].clone(),
        top_label: "Level:".to_string(),
        top_value: level_input,
        headers_1,
        data_1,
        headers_2,
        data_2,
        traits, h1, h2, b1, b2, footer, spirit_data,
    }
}

fn build_spirit_data(ctx: &CatRenderContext, settings: &Settings) -> Option<SpiritData> {
    if ctx.base_stats.conjure_unit_id <= 0 {
        return None;
    }

    let conjure_stats_vec = unitid(ctx.base_stats.conjure_unit_id, &settings.general.language_priority)?;
    let conjure_stats = conjure_stats_vec.first()?;

    let conjure_final = get_final_stats(conjure_stats, ctx.level_curve, ctx.current_level, None, None);

    let spirit_ctx = CatRenderContext {
        global: ctx.global,
        base_stats: conjure_stats,
        final_stats: &conjure_final,
        current_level: ctx.current_level,
        level_curve: ctx.level_curve,
        talent_data: None,
        talent_levels: None,
        is_conjure_unit: true,
    };

    let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = collect_ability_data(&spirit_ctx);

    Some(SpiritData {
        dmg_text: format!("Damage {}\nRange {}", conjure_final.attack_1, conjure_final.standing_range),
        traits: s_traits,
        h1: s_h1,
        h2: s_h2,
        b1: s_b1,
        b2: s_b2,
        footer: s_footer,
    })
}
