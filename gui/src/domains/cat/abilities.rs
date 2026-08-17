use std::collections::HashMap;

use iced::widget::column;
use iced::Element;
use nyanko::cat::unit::LevelCurve;
use nyanko::common::data::img015;

use core::common::context::GlobalContext;
use core::domains::cat::scanner::CatEntry;
use core::domains::settings::Settings;
use core::systems::combat::{AbilityItem, CustomIcon, RenderContext, ABILITY_Y};

use crate::common::{CustomAssets, SpriteSheet};
use crate::systems::combat::abilities::{self as shared, ListLayout};
use crate::widget::ability_spacer;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleConjureExpand(u32),
}

#[derive(Clone, Copy)]
pub(super) struct SpiritContext<'a> {
    pub(super) cat_id: u32,
    pub(super) global: GlobalContext<'a>,
    pub(super) level_curve: Option<&'a LevelCurve>,
    pub(super) current_level: i32,
    pub(super) conjure_unit_id: i32,
}

#[derive(Default)]
pub struct State {
    pub(super) shared: shared::State,
    conjure_overrides: HashMap<u32, bool>,
}


impl State {
    pub(super) fn clear_icons(&self) {
        self.shared.clear_icons();
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleConjureExpand(cat_id) => {
                let current = self.conjure_overrides.get(&cat_id).copied();
                self.conjure_overrides.insert(cat_id, !current.unwrap_or(false));
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        ctx: &RenderContext<'_>,
        cat: &'a CatEntry,
        global_ctx: GlobalContext<'a>,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
        settings: &'a Settings,
    ) -> Element<'a, Message> {
        let spirit = SpiritContext {
            cat_id: cat.id,
            global: global_ctx,
            level_curve: cat.curve.as_ref(),
            current_level: ctx.current_level,
            conjure_unit_id: ctx.base_stats.conjure_unit_id,
        };

        self.shared.view(ctx, sheets, assets, move |items, layout| {
            self.ability_list(items, spirit, sheets, assets, settings, layout)
        })
    }

    pub(super) fn ability_list<'a>(
        &'a self,
        items: &[AbilityItem],
        spirit: SpiritContext<'a>,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
        settings: &'a Settings,
        layout: ListLayout,
    ) -> Element<'a, Message> {
        let mut col = column![].spacing(0).width(layout.width());

        for (index, item) in items.iter().enumerate() {
            let is_conjure = item.icon_id == Some(img015::ICON_CONJURE) && item.custom_icon == CustomIcon::None;

            let item_row = if is_conjure {
                self.conjure_row(item, spirit, sheets, assets, settings)
            } else {
                self.shared.ability_row(item, sheets, assets, layout)
            };

            col = col.push(item_row);

            if is_conjure && self.conjure_expanded(spirit.cat_id, settings) {
                col = col.push(ability_spacer(ABILITY_Y));
                col = col.push(self.conjure_details(spirit, sheets, assets, settings, layout.per_row));
            }

            if index + 1 < items.len() {
                col = col.push(ability_spacer(ABILITY_Y));
            }
        }

        col.into()
    }

    pub(super) fn conjure_expanded(&self, cat_id: u32, settings: &Settings) -> bool {
        self.conjure_overrides.get(&cat_id).copied().unwrap_or(settings.cat_data.expand_spirit_details)
    }

    pub(crate) fn is_conjure_expanded(&self, cat_id: u32, settings: &Settings) -> bool {
        self.conjure_expanded(cat_id, settings)
    }

}
