use std::collections::HashMap;

use core::common::context::GlobalContext;
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::waiter::unitid;
use core::modules::settings::Settings;

use crate::common::SpriteSheet;
use crate::widget::statblock_export::Request;

use super::statblock::build_cat_statblock;

pub(super) struct Ctx<'a> {
    pub(super) cat: &'a CatEntry,
    pub(super) form: usize,
    pub(super) current_level: i32,
    pub(super) level_input: &'a str,
    pub(super) talent_levels: Option<&'a HashMap<u8, u8>>,
    pub(super) is_conjure_expanded: bool,
    pub(super) sheets: &'a [SpriteSheet],
    pub(super) global: GlobalContext<'a>,
    pub(super) settings: &'a Settings,
}

pub(super) fn request(ctx: Ctx<'_>) -> Option<Request<'_>> {
    let cat = ctx.cat;
    let dynamic_stats = unitid(cat.id as i32, &ctx.settings.general.language_priority);
    let base_stats = dynamic_stats.as_ref().and_then(|forms| forms.get(ctx.form))?;

    let form_allows_talents = ctx.form >= 2;
    let talent_data = if form_allows_talents { cat.talent_data.as_ref() } else { None };
    let talent_levels = if form_allows_talents { ctx.talent_levels } else { None };
    let final_stats = get_final_stats(base_stats, cat.curve.as_ref(), ctx.current_level, talent_data, talent_levels);

    let cat_ctx = CatRenderContext {
        global: ctx.global,
        base_stats,
        final_stats: &final_stats,
        current_level: ctx.current_level,
        level_curve: cat.curve.as_ref(),
        talent_data,
        talent_levels,
        is_conjure_unit: false,
    };

    let data = build_cat_statblock(&cat_ctx, cat, ctx.form, ctx.level_input.to_string(), ctx.is_conjure_expanded, ctx.settings);

    Some(Request { data, sheets: ctx.sheets, settings: ctx.settings })
}
