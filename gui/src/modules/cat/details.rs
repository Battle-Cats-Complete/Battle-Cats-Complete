use std::cell::RefCell;
use std::collections::HashMap;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, scrollable, stack, text, Space};
use iced::{font, Border, Color, Element, Font, Length, Padding, Theme};

use core::common::io;
use core::modules::cat::scanner::CatEntry;
use core::Vfs;

use crate::common::item_icon;
use crate::widget::smooth_scroll;

use super::Message;

const HEADING_SIZE: f32 = 20.0;
const SECTION_SPACING: f32 = 16.0;
const SECTION_GAP: f32 = 11.0;
const HEADING_BODY_GAP: f32 = 8.0;
const EVOLVE_ITEM_SPACING: f32 = 6.0;
const MATERIAL_CANVAS: u32 = 128;
const MATERIAL_SIZE: f32 = 64.0;
const MATERIAL_SPACING: f32 = 5.0;
const AMOUNT_TEXT_SIZE: f32 = 13.0;
const XP_ICON_ID: i32 = 6;
const XP_ICON_HEIGHT: f32 = 32.0;
const XP_TEXT_SIZE: f32 = 18.0;
const XP_SPACING: f32 = 5.0;
const EVOLVE_TEXT_LINES: usize = 3;

#[derive(Clone)]
struct TrimmedIcon {
    handle: Handle,
    width: u32,
    height: u32,
}

#[derive(Default)]
pub(super) struct State {
    boxed_icons: RefCell<HashMap<i32, Option<Handle>>>,
    trimmed_icons: RefCell<HashMap<i32, Option<TrimmedIcon>>>,
}

impl State {
    pub(super) fn clear_icons(&self) {
        self.boxed_icons.borrow_mut().clear();
        self.trimmed_icons.borrow_mut().clear();
    }

    fn boxed_icon(&self, id: i32, vfs: &Vfs) -> Option<Handle> {
        if let Some(cached) = self.boxed_icons.borrow().get(&id) {
            return cached.clone();
        }

        let handle = io::gatya_item_icon(vfs, id).and_then(|path| item_icon::load_boxed(&path, MATERIAL_CANVAS));
        self.boxed_icons.borrow_mut().insert(id, handle.clone());
        handle
    }

    fn trimmed_icon(&self, id: i32, vfs: &Vfs) -> Option<TrimmedIcon> {
        if let Some(cached) = self.trimmed_icons.borrow().get(&id) {
            return cached.clone();
        }

        let loaded = io::gatya_item_icon(vfs, id)
            .and_then(|path| item_icon::load_cropped(&path))
            .map(|(handle, width, height)| TrimmedIcon { handle, width, height });
        self.trimmed_icons.borrow_mut().insert(id, loaded.clone());
        loaded
    }

    pub(super) fn view<'a>(&'a self, cat: &'a CatEntry, form: usize, vfs: &'a Vfs) -> Element<'a, Message> {
        let description = cat.description.get(form)
            .and_then(|d| d.as_ref())
            .map(|lines| lines.join("\n"))
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| "No description available".to_string());

        let description_section = column![
            text("Description").size(HEADING_SIZE),
            text(description),
        ]
            .spacing(HEADING_BODY_GAP)
            .width(Length::Fill);

        let mut content = column![description_section]
            .spacing(SECTION_SPACING)
            .width(Length::Fill);

        if let Some(evolve) = self.view_evolve(cat, form, vfs) {
            content = content.push(evolve);
        }

        smooth_scroll(scrollable(content)).into()
    }

    fn view_evolve<'a>(&'a self, cat: &'a CatEntry, form: usize, vfs: &'a Vfs) -> Option<Element<'a, Message>> {
        let (materials, xp_cost) = match form {
            2 => (&cat.unitbuy.true_form_materials, cat.unitbuy.true_form_xp_cost),
            3 => (&cat.unitbuy.ultra_form_materials, cat.unitbuy.ultra_form_xp_cost),
            _ => return None,
        };

        let evolve_text = cat.evolve_text.texts.get(form)
            .and_then(|t| t.as_ref())
            .filter(|lines| !lines.is_empty())
            .map(|lines| {
                let mut padded = lines.clone();
                padded.resize(EVOLVE_TEXT_LINES, String::new());
                padded.join("\n")
            });

        if materials.is_empty() && evolve_text.is_none() && xp_cost <= 0 {
            return None;
        }

        let mut section = column![
            Space::new().height(Length::Fixed(SECTION_GAP)),
            text("Evolve").size(HEADING_SIZE),
            Space::new().height(Length::Fixed(HEADING_BODY_GAP)),
        ]
            .width(Length::Fill);

        if let Some(evolve_text) = evolve_text {
            section = section.push(text(evolve_text));
            section = section.push(Space::new().height(Length::Fixed(EVOLVE_ITEM_SPACING)));
        }

        if !materials.is_empty() {
            let mut icon_row = row![].spacing(MATERIAL_SPACING);
            for (item_id, amount) in materials {
                icon_row = icon_row.push(self.view_material(*item_id, *amount, vfs));
            }
            section = section.push(icon_row);
            section = section.push(Space::new().height(Length::Fixed(EVOLVE_ITEM_SPACING)));
        }

        if xp_cost > 0 {
            section = section.push(self.view_xp_cost(xp_cost, vfs));
        }

        Some(section.into())
    }

    fn view_material<'a>(&'a self, item_id: i32, amount: i32, vfs: &Vfs) -> Element<'a, Message> {
        let icon: Element<'a, Message> = self.boxed_icon(item_id, vfs).map_or_else(
            || container(text(format!("ID {}", item_id)).size(AMOUNT_TEXT_SIZE + 1.0))
                .width(Length::Fixed(MATERIAL_SIZE))
                .height(Length::Fixed(MATERIAL_SIZE))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(placeholder_style)
                .into(),
            |handle| iced_image(handle)
                .width(Length::Fixed(MATERIAL_SIZE))
                .height(Length::Fixed(MATERIAL_SIZE))
                .into(),
        );

        let badge = container(text(format!("×{}", amount)).size(AMOUNT_TEXT_SIZE))
            .padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 })
            .style(badge_style);

        stack![
            icon,
            container(badge)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Right)
                .align_y(Vertical::Bottom),
        ]
            .width(Length::Fixed(MATERIAL_SIZE))
            .height(Length::Fixed(MATERIAL_SIZE))
            .into()
    }

    fn view_xp_cost<'a>(&'a self, xp_cost: i32, vfs: &Vfs) -> Element<'a, Message> {
        let icon: Element<'a, Message> = self.trimmed_icon(XP_ICON_ID, vfs).map_or_else(
            || container(text("XP").size(AMOUNT_TEXT_SIZE))
                .width(Length::Fixed(XP_ICON_HEIGHT))
                .height(Length::Fixed(XP_ICON_HEIGHT))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(placeholder_style)
                .into(),
            |icon| {
                let display_width = if icon.height > 0 {
                    XP_ICON_HEIGHT * (icon.width as f32 / icon.height as f32)
                } else {
                    XP_ICON_HEIGHT
                };

                iced_image(icon.handle)
                    .width(Length::Fixed(display_width))
                    .height(Length::Fixed(XP_ICON_HEIGHT))
                    .into()
            },
        );

        row![
            icon,
            text(xp_cost.to_string()).size(XP_TEXT_SIZE).font(Font { weight: font::Weight::Bold, ..Font::DEFAULT }),
        ]
            .spacing(XP_SPACING)
            .align_y(Vertical::Center)
            .into()
    }
}

fn placeholder_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

fn badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color { a: 0.63, ..Color::BLACK }.into()),
        text_color: Some(Color::WHITE),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}
