use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};
use nyanko::common::data::img015;

use core::common::game::{ABILITY_Y, TRAIT_Y};
use core::common::game::AbilityItem;
use core::modules::cat::game::abilities::collect_ability_data;
use core::modules::cat::game::stats::get_final_stats;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::waiter::unitid;
use core::modules::settings::Settings;

use crate::app::theme;
use crate::common::superscript::text_with_superscript;
use crate::common::{CustomAssets, SpriteSheet};

use super::abilities::{spacer, DESCRIPTION_TEXT_SIZE};
use super::abilities::{ListLayout, Message, SpiritContext, State};

const ICON_TEXT_GAP: f32 = 8.0;
const DETAILS_BUTTON_GAP: f32 = 8.0;

impl State {
    pub(super) fn conjure_row(
        &self,
        item: &AbilityItem,
        spirit: SpiritContext,
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
        settings: &Settings,
    ) -> Element<'_, Message> {
        let icon = self.icon_element(item, sheets, assets);
        let description = text_with_superscript(&item.text, DESCRIPTION_TEXT_SIZE);
        let expanded = self.conjure_expanded(spirit.cat_id, settings);
        let details_btn = button(text("Details").size(11))
            .style(move |t: &Theme, status| theme::toggle_button(t, status, expanded))
            .on_press(Message::ToggleConjureExpand(spirit.cat_id));

        row![
            icon,
            Space::new().width(Length::Fixed(ICON_TEXT_GAP)),
            description,
            Space::new().width(Length::Fixed(DETAILS_BUTTON_GAP)),
            details_btn,
        ]
        .align_y(Alignment::Center)
        .into()
    }

    pub(super) fn conjure_details(
        &self,
        spirit: SpiritContext,
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
        settings: &Settings,
        per_row: usize,
    ) -> Element<'_, Message> {
        let spirit = SpiritContext { cat_id: 0, ..spirit };
        let Some(conjure_stats_vec) = unitid(spirit.conjure_unit_id, &settings.general.language_priority) else {
            return container(text("Spirit data not found")).padding(8).into();
        };

        let Some(conjure_stats) = conjure_stats_vec.first() else {
            return container(text("Spirit data not found")).padding(8).into();
        };

        let conjure_final = get_final_stats(conjure_stats, spirit.level_curve, spirit.current_level, None, None);

        let spirit_ctx = CatRenderContext {
            global: spirit.global,
            base_stats: conjure_stats,
            final_stats: &conjure_final,
            current_level: spirit.current_level,
            level_curve: spirit.level_curve,
            talent_data: None,
            talent_levels: None,
            is_conjure_unit: true,
        };

        let dmg_row = row![
            self.raw_icon(img015::ICON_AREA_ATTACK, sheets),
            text(format!("Damage {}\nRange {}", conjure_final.attack_1, conjure_final.standing_range)).size(DESCRIPTION_TEXT_SIZE)
        ].spacing(ICON_TEXT_GAP).align_y(Alignment::Center);

        let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = collect_ability_data(&spirit_ctx);

        let mut col = column![dmg_row].spacing(0).width(Length::Shrink);
        let mut prev = false;
        let mut last_was_trait = false;

        if !s_traits.is_empty() {
            col = col.push(spacer(ABILITY_Y));
            col = col.push(self.icon_row(&s_traits, sheets, assets, per_row));
            prev = true;
            last_was_trait = true;
        }

        if !s_h1.is_empty() {
            col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
            col = col.push(self.icon_row(&s_h1, sheets, assets, per_row));
            prev = true;
            last_was_trait = false;
        }

        if !s_h2.is_empty() {
            col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
            col = col.push(self.icon_row(&s_h2, sheets, assets, per_row));
            prev = true;
            last_was_trait = false;
        }

        let has_body = !s_b1.is_empty() || !s_b2.is_empty();
        if has_body {
            let layout = ListLayout { per_row, fill: false };
            col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
            col = col.push(self.ability_list(&s_b1, spirit, sheets, assets, settings, layout));
            if !s_b1.is_empty() && !s_b2.is_empty() {
                col = col.push(spacer(ABILITY_Y));
            }
            col = col.push(self.ability_list(&s_b2, spirit, sheets, assets, settings, layout));
            prev = true;
        }

        if !s_footer.is_empty() {
            if prev {
                col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
            }
            col = col.push(self.icon_row(&s_footer, sheets, assets, per_row));
        }

        container(col).width(Length::Shrink).padding(8).style(theme::spirit_card_container).into()
    }
}
