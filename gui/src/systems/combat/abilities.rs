use iced::widget::{column, container, image as iced_image, responsive, row, scrollable, stack, tooltip};
use iced::{Alignment, Element, Length, Size};

use kore::systems::combat::abilities::collect_ability_data;
use kore::systems::combat::registry::{get_fallback_by_icon, AbilityIcon};
use kore::systems::combat::{AbilityItem, CustomIcon, RenderContext, ABILITY_X, ABILITY_Y, TRAIT_Y};

use crate::common::ability_icon;
use crate::common::{CustomAssets, SpriteSheet};
use crate::widget::text_with_superscript;
use crate::widget::{ability_spacer, fallback_icon, icons_per_row, smooth_scroll, ICON_SIZE};

pub(crate) const DESCRIPTION_TEXT_SIZE: f32 = 13.0;

const MISSING_ICON_ID: usize = 9999;

#[derive(Clone, Copy)]
pub(crate) struct ListLayout {
    pub(crate) per_row: usize,
    pub(crate) fill: bool,
}

impl ListLayout {
    pub(crate) fn width(&self) -> Length {
        if self.fill { Length::Fill } else { Length::Shrink }
    }
}

#[derive(Default)]
pub(crate) struct State {
    icons: ability_icon::Cache,
}

impl State {
    pub(crate) fn clear_icons(&self) {
        self.icons.clear();
    }

    pub(crate) fn view<'a, Message: 'a>(
        &'a self,
        ctx: &RenderContext<'_>,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
        body: impl Fn(&[AbilityItem], ListLayout) -> Element<'a, Message> + 'a,
    ) -> Element<'a, Message> {
        let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = collect_ability_data(ctx);

        responsive(move |size: Size| {
            let per_row = icons_per_row(size.width, ABILITY_X);

            let mut col = column![].spacing(0).width(Length::Fill);
            let mut previous_content = false;
            let mut last_was_trait = false;

            if !grp_trait.is_empty() {
                col = col.push(self.icon_row(&grp_trait, sheets, assets, per_row));
                previous_content = true;
                last_was_trait = true;
            }

            for headline in [&grp_hl1, &grp_hl2] {
                if headline.is_empty() { continue; }

                if previous_content {
                    col = col.push(ability_spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                    last_was_trait = false;
                }

                col = col.push(self.icon_row(headline, sheets, assets, per_row));
                previous_content = true;
            }

            if !grp_b1.is_empty() || !grp_b2.is_empty() {
                if previous_content {
                    col = col.push(ability_spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                    last_was_trait = false;
                }

                let layout = ListLayout { per_row, fill: true };
                col = col.push(body(&grp_b1, layout));

                if !grp_b1.is_empty() && !grp_b2.is_empty() {
                    col = col.push(ability_spacer(ABILITY_Y));
                }

                col = col.push(body(&grp_b2, layout));
                previous_content = true;
            }

            if !grp_footer.is_empty() {
                if previous_content {
                    col = col.push(ability_spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                }
                col = col.push(self.icon_row(&grp_footer, sheets, assets, per_row));
            }

            smooth_scroll(scrollable(col).height(Length::Fill).width(Length::Fill)).into()
        }).into()
    }

    pub(crate) fn icon_row<'a, Message: 'a>(
        &self,
        items: &[AbilityItem],
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
        per_row: usize,
    ) -> Element<'a, Message> {
        let mut col = column![].spacing(ABILITY_Y);

        for chunk in items.chunks(per_row) {
            let mut wrapped_row = row![].spacing(ABILITY_X).align_y(Alignment::Center);

            for item in chunk {
                wrapped_row = wrapped_row.push(tooltip(
                    self.icon_element(item, sheets, assets),
                    container(text_with_superscript(&item.text, DESCRIPTION_TEXT_SIZE)).padding(6).style(container::bordered_box),
                    tooltip::Position::Top,
                ));
            }

            col = col.push(wrapped_row);
        }

        col.into()
    }

    pub(crate) fn ability_row<'a, Message: 'a>(
        &self,
        item: &AbilityItem,
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
        layout: ListLayout,
    ) -> Element<'a, Message> {
        let icon = self.icon_element(item, sheets, assets);
        let description = container(text_with_superscript(&item.text, DESCRIPTION_TEXT_SIZE)).width(layout.width());

        row![icon, description].spacing(8).align_y(Alignment::Center).width(layout.width()).into()
    }

    pub(crate) fn ability_list<'a, Message: 'a>(
        &self,
        items: &[AbilityItem],
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
        layout: ListLayout,
    ) -> Element<'a, Message> {
        let mut col = column![].spacing(0).width(layout.width());

        for (index, item) in items.iter().enumerate() {
            col = col.push(self.ability_row(item, sheets, assets, layout));

            if index + 1 < items.len() {
                col = col.push(ability_spacer(ABILITY_Y));
            }
        }

        col.into()
    }

    pub(crate) fn icon_element<'a, Message: 'a>(
        &self,
        item: &AbilityItem,
        sheets: &[SpriteSheet],
        assets: &CustomAssets,
    ) -> Element<'a, Message> {
        if item.custom_icon != CustomIcon::None
            && let Some(handle) = assets.get_icon_texture(item.custom_icon) {
            return sized_image(handle);
        }

        if let Some(icon_id) = item.icon_id
            && let Some(handle) = self.icons.handle(icon_id, sheets) {
            let Some(border_id) = item.border_id else {
                return sized_image(handle);
            };

            let Some(border_handle) = self.icons.handle(border_id, sheets) else {
                return sized_image(handle);
            };

            return stack![sized_image::<Message>(handle), sized_image::<Message>(border_handle)]
                .width(Length::Fixed(ICON_SIZE))
                .height(Length::Fixed(ICON_SIZE))
                .into();
        }

        let icon = if item.custom_icon != CustomIcon::None {
            AbilityIcon::Custom(item.custom_icon)
        } else {
            AbilityIcon::Standard(item.icon_id.unwrap_or(MISSING_ICON_ID))
        };

        fallback_icon(get_fallback_by_icon(icon))
    }

    pub(crate) fn raw_icon<'a, Message: 'a>(&self, icon_id: usize, sheets: &[SpriteSheet]) -> Element<'a, Message> {
        self.icons.handle(icon_id, sheets).map_or_else(
            || fallback_icon(get_fallback_by_icon(AbilityIcon::Standard(icon_id))),
            sized_image,
        )
    }
}

fn sized_image<'a, Message: 'a>(handle: iced::widget::image::Handle) -> Element<'a, Message> {
    iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).into()
}
