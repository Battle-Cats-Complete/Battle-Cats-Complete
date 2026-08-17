use std::collections::HashMap;

use crate::common::context::GlobalContext;
use crate::systems::combat::abilities::collect_ability_data;
use crate::systems::combat::registry::{format_stat, Magnification, StatContext, STAT_ATK_CYCLE, STAT_ATTACK, STAT_COOLDOWN, STAT_COST, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_RARITY, STAT_SPEED};
use crate::systems::combat::RenderContext;
use crate::systems::statblock::{SpiritData, StatCell, StatblockData};

use super::game::stats::get_final_stats;
use super::scanner::CatEntry;
use super::waiter::unitid;

pub struct Subject<'a> {
    pub cat: &'a CatEntry,
    pub form: usize,
    pub current_level: i32,
    pub level_input: &'a str,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub is_conjure_expanded: bool,
    pub global: GlobalContext<'a>,
}

pub fn build(subject: Subject<'_>) -> Option<StatblockData> {
    let cat = subject.cat;
    let dynamic_stats = unitid(&subject.global.vault.vfs, cat.id as i32);
    let base_stats = dynamic_stats.as_ref().and_then(|forms| forms.get(subject.form))?;

    let form_allows_talents = subject.form >= 2;
    let talent_data = if form_allows_talents { cat.talent_data.as_ref() } else { None };
    let talent_levels = if form_allows_talents { subject.talent_levels } else { None };
    let final_stats = get_final_stats(base_stats, cat.curve.as_ref(), subject.current_level, talent_data, talent_levels);

    let ctx = RenderContext {
        global: subject.global,
        base_stats,
        final_stats: &final_stats,
        magnification: Magnification::default(),
        current_level: subject.current_level,
        level_curve: cat.curve.as_ref(),
        talent_data,
        talent_levels,
        is_conjure_unit: false,
    };

    Some(assemble(&ctx, cat, subject.form, subject.level_input.to_string(), subject.is_conjure_expanded))
}

fn assemble(
    ctx: &RenderContext<'_>,
    cat_entry: &CatEntry,
    current_form: usize,
    level_input: String,
    is_conjure_expanded: bool,
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let spirit_data = if is_conjure_expanded {
        build_spirit_data(ctx)
    } else {
        None
    };

    let anim_frames = cat_entry.atk_anim_frames[current_form];
    let stat_ctx = StatContext::cat(ctx.final_stats, anim_frames, Some(&cat_entry.unitbuy));
    let cycle = (STAT_ATK_CYCLE.get_value)(&stat_ctx);

    let headers_1 = vec![
        STAT_ATTACK.display_name.to_string(),
        STAT_DPS.display_name.to_string(),
        STAT_RANGE.display_name.to_string(),
        STAT_ATK_CYCLE.display_name.to_string(),
        STAT_RARITY.display_name.to_string(),
    ];

    let data_1 = vec![
        StatCell::Text(format_stat(&STAT_ATTACK, &stat_ctx)),
        StatCell::Text(format_stat(&STAT_DPS, &stat_ctx)),
        StatCell::Text(ctx.final_stats.standing_range.to_string()),
        StatCell::Frames(cycle),
        StatCell::Text(format_stat(&STAT_RARITY, &stat_ctx)),
    ];

    let headers_2 = vec![
        STAT_HITPOINTS.display_name.to_string(),
        STAT_KNOCKBACKS.display_name.to_string(),
        STAT_SPEED.display_name.to_string(),
        STAT_COOLDOWN.display_name.to_string(),
        STAT_COST.display_name.to_string(),
    ];

    let cd_frames = (STAT_COOLDOWN.get_value)(&stat_ctx);

    let data_2 = vec![
        StatCell::Text(ctx.final_stats.hitpoints.to_string()),
        StatCell::Text(ctx.final_stats.knockbacks.to_string()),
        StatCell::Text(ctx.final_stats.speed.to_string()),
        StatCell::Frames(cd_frames),
        StatCell::Text(format_stat(&STAT_COST, &stat_ctx)),
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

fn build_spirit_data(ctx: &RenderContext<'_>) -> Option<SpiritData> {
    if ctx.base_stats.conjure_unit_id <= 0 {
        return None;
    }

    let conjure_stats_vec = unitid(&ctx.global.vault.vfs, ctx.base_stats.conjure_unit_id)?;
    let conjure_stats = conjure_stats_vec.first()?;

    let conjure_final = get_final_stats(conjure_stats, ctx.level_curve, ctx.current_level, None, None);

    let spirit_ctx = RenderContext {
        base_stats: conjure_stats,
        final_stats: &conjure_final,
        talent_data: None,
        talent_levels: None,
        is_conjure_unit: true,
        ..*ctx
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
