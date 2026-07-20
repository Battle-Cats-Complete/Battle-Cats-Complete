use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, responsive, row, scrollable, stack, tooltip, Space};
use iced::{Alignment, Element, Length, Size};
use image::imageops;

use core::common::game::{AbilityItem, CustomIcon, ABILITY_X, ABILITY_Y, TRAIT_Y};
use core::modules::enemy::game::abilities::collect_ability_data;
use core::modules::enemy::game::registry::{get_fallback_by_icon, AbilityIcon};
use core::modules::enemy::game::EnemyRenderContext;

use crate::common::shared::{fallback_icon, text_with_superscript, ICON_SIZE};
use crate::common::{CustomAssets, SpriteSheet};

use super::Message;

const SCROLLBAR_RESERVE: f32 = 24.0;

pub struct State {
    icon_cache: RefCell<HashMap<usize, Handle>>,
}

impl Default for State {
    fn default() -> Self {
        Self { icon_cache: RefCell::new(HashMap::new()) }
    }
}

impl State {
    pub fn view<'a>(
        &'a self,
        ctx: &EnemyRenderContext,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
    ) -> Element<'a, Message> {
        let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = collect_ability_data(ctx);

        responsive(move |size: Size| {
            let per_row = icons_per_row(size.width);

            let mut col = column![].spacing(0).width(Length::Fill);
            let mut previous_content = false;
            let mut last_was_trait = false;

            if !grp_trait.is_empty() {
                col = col.push(self.icon_row(&grp_trait, sheets, assets, per_row));
                previous_content = true;
                last_was_trait = true;
            }

            if !grp_hl1.is_empty() {
                if previous_content {
                    col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                    last_was_trait = false;
                }
                col = col.push(self.icon_row(&grp_hl1, sheets, assets, per_row));
                previous_content = true;
            }

            if !grp_hl2.is_empty() {
                if previous_content {
                    col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                    last_was_trait = false;
                }
                col = col.push(self.icon_row(&grp_hl2, sheets, assets, per_row));
                previous_content = true;
            }

            let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
            if has_body {
                if previous_content {
                    col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                    last_was_trait = false;
                }

                col = col.push(self.ability_list(&grp_b1, sheets, assets));

                if !grp_b1.is_empty() && !grp_b2.is_empty() {
                    col = col.push(spacer(ABILITY_Y));
                }

                col = col.push(self.ability_list(&grp_b2, sheets, assets));
                previous_content = true;
            }

            if !grp_footer.is_empty() {
                if previous_content {
                    col = col.push(spacer(if last_was_trait { TRAIT_Y } else { ABILITY_Y }));
                }
                col = col.push(self.icon_row(&grp_footer, sheets, assets, per_row));
            }

            scrollable(col).height(Length::Fill).width(Length::Fill).into()
        }).into()
    }

    fn icon_row(&self, items: &[AbilityItem], sheets: &[SpriteSheet], assets: &CustomAssets, per_row: usize) -> Element<'_, Message> {
        let mut col = column![].spacing(ABILITY_Y);
        for chunk in items.chunks(per_row) {
            let mut wrapped_row = row![].spacing(ABILITY_X).align_y(Alignment::Center);
            for item in chunk {
                let icon = self.icon_element(item, sheets, assets);
                wrapped_row = wrapped_row.push(tooltip(
                    icon,
                    container(text_with_superscript(&item.text)).padding(6).style(container::bordered_box),
                    tooltip::Position::Top,
                ));
            }
            col = col.push(wrapped_row);
        }

        col.into()
    }

    fn ability_list(&self, items: &[AbilityItem], sheets: &[SpriteSheet], assets: &CustomAssets) -> Element<'_, Message> {
        let mut col = column![].spacing(0).width(Length::Fill);
        let count = items.len();

        for (i, item) in items.iter().enumerate() {
            let icon = self.icon_element(item, sheets, assets);
            let description = container(text_with_superscript(&item.text)).width(Length::Fill);

            col = col.push(row![icon, description].spacing(8).align_y(Alignment::Center).width(Length::Fill));

            if i < count - 1 {
                col = col.push(spacer(ABILITY_Y));
            }
        }

        col.into()
    }

    fn icon_element(&self, item: &AbilityItem, sheets: &[SpriteSheet], assets: &CustomAssets) -> Element<'_, Message> {
        if item.custom_icon != CustomIcon::None {
            if let Some(handle) = assets.get_icon_texture(item.custom_icon) {
                return iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).into();
            }
        }

        if let Some(icon_id) = item.icon_id {
            if let Some(handle) = self.icon_handle(icon_id, sheets) {
                let icon_widget = iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE));

                if let Some(border_id) = item.border_id {
                    if let Some(border_handle) = self.icon_handle(border_id, sheets) {
                        let border_widget = iced_image(border_handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE));
                        return stack![icon_widget, border_widget].width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).into();
                    }
                }

                return icon_widget.into();
            }
        }

        let icon_enum = if item.custom_icon != core::common::game::CustomIcon::None {
            AbilityIcon::Custom(item.custom_icon)
        } else {
            AbilityIcon::Standard(item.icon_id.unwrap_or(9999))
        };

        fallback_icon(get_fallback_by_icon(icon_enum))
    }

    fn icon_handle(&self, icon_id: usize, sheets: &[SpriteSheet]) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&icon_id) {
            return Some(cached.clone());
        }

        for sheet in sheets {
            let Some(cut) = sheet.core.cuts_map.get(&icon_id) else { continue; };
            let Some(image_data) = &sheet.core.image_data else { continue; };

            let width = image_data.width();
            let height = image_data.height();

            let px = (cut.uv_coordinates.min.x * width as f32).round() as u32;
            let py = (cut.uv_coordinates.min.y * height as f32).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;

            if pw == 0 || ph == 0 || px + pw > width || py + ph > height {
                continue;
            }

            let cropped = imageops::crop_imm(image_data.as_ref(), px, py, pw, ph).to_image();
            let handle = Handle::from_rgba(pw, ph, cropped.into_raw());
            self.icon_cache.borrow_mut().insert(icon_id, handle.clone());
            return Some(handle);
        }

        None
    }
}

fn spacer<'a>(height: f32) -> Element<'a, Message> {
    Space::new().height(Length::Fixed(height)).into()
}

fn icons_per_row(available_width: f32) -> usize {
    let usable = (available_width - SCROLLBAR_RESERVE).max(ICON_SIZE);
    let slot = ICON_SIZE + ABILITY_X;
    (((usable + ABILITY_X) / slot).floor() as usize).max(1)
}
