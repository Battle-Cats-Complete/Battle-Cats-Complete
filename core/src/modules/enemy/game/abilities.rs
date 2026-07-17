use nyanko::enemy::abilities::{AttrUnit, REGISTRY};

use crate::common::game::{AbilityItem, CustomIcon};
use crate::modules::enemy::game::registry::{self, AbilityIcon, DisplayGroup};
use crate::modules::enemy::game::EnemyRenderContext;

pub fn collect_ability_data(
    ctx: &EnemyRenderContext,
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    let mut group_trait = Vec::new();
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    for def in REGISTRY {
        let display_def = registry::get_display_def(def.identity);

        if display_def.group == DisplayGroup::Hidden { continue; }

        let attrs = (def.attributes)(ctx.stats);

        if !attrs.is_empty() {
            let val = attrs.first().map(|(_, v, _)| *v).unwrap_or(0);
            let dur = attrs.iter().find(|(_, _, u)| *u == AttrUnit::Frames).map(|(_, v, _)| *v).unwrap_or(0);

            let text = (display_def.formatter)(val, ctx.stats, dur, ctx.magnification, ctx.global.param);

            let (final_icon, custom_icon) = match display_def.icon {
                AbilityIcon::Standard(id) => (Some(id), CustomIcon::None),
                AbilityIcon::Custom(icon) => (None, icon),
                AbilityIcon::None => (None, CustomIcon::None),
            };

            let item = AbilityItem { icon_id: final_icon, text, custom_icon, border_id: None };

            match display_def.group {
                DisplayGroup::Type => group_trait.push(item),
                DisplayGroup::Headline1 => group_headline_1.push(item),
                DisplayGroup::Headline2 => group_headline_2.push(item),
                DisplayGroup::Body1 => group_body_1.push(item),
                DisplayGroup::Body2 => group_body_2.push(item),
                DisplayGroup::Footer => group_footer.push(item),
                DisplayGroup::Hidden => {}
            }
        }
    }

    (group_trait, group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}