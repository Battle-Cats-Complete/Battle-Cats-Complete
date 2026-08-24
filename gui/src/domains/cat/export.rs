use std::collections::HashMap;

use kore::common::context::GlobalContext;
use kore::domains::cat::scanner::CatEntry;
use kore::domains::cat::statblock::{self, Subject};
use kore::domains::settings::Settings;

use crate::common::SpriteSheet;
use crate::widget::statblock_export::Request;

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
    let data = statblock::build(Subject {
        cat: ctx.cat,
        form: ctx.form,
        current_level: ctx.current_level,
        level_input: ctx.level_input,
        talent_levels: ctx.talent_levels,
        is_conjure_expanded: ctx.is_conjure_expanded,
        global: ctx.global,
    })?;

    Some(Request { data, sheets: ctx.sheets, settings: ctx.settings })
}
