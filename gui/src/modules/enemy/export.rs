use core::common::context::GlobalContext;
use core::modules::enemy::game::registry::Magnification;
use core::modules::enemy::game::EnemyRenderContext;
use core::modules::enemy::scanner::{self, EnemyEntry};
use core::modules::settings::Settings;

use crate::common::SpriteSheet;
use crate::widget::statblock_export::Request;

use super::statblock::build_enemy_statblock;

pub(super) struct Ctx<'a> {
    pub(super) enemy: &'a EnemyEntry,
    pub(super) magnification: Magnification,
    pub(super) sheets: &'a [SpriteSheet],
    pub(super) global: GlobalContext<'a>,
    pub(super) settings: &'a Settings,
}

pub(super) fn request(ctx: Ctx<'_>) -> Option<Request<'_>> {
    let enemy = ctx.enemy;
    let dynamic_entry = scanner::scan_single(enemy.id, &ctx.settings.scanner_config());
    let stats = dynamic_entry.as_ref().map_or(&enemy.stats, |entry| &entry.stats);

    let enemy_ctx = EnemyRenderContext {
        global: ctx.global,
        stats,
        magnification: ctx.magnification,
    };

    let data = build_enemy_statblock(&enemy_ctx, enemy);

    Some(Request { data, sheets: ctx.sheets, settings: ctx.settings })
}
