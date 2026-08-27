use std::cell::RefCell;
use std::collections::HashMap;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, row, scrollable, space, tooltip};
use iced::{Alignment, Border, Element, Length, Theme};
use nyanko::chapter::stage::{AbilityType, CannonType, CertificationPreset, EvolutionForm, TreasureType};

use kore::domains::cat::waiter::unitexplanation;
use kore::domains::stage::fixedlineup::{ResolvedFixedLineup, ResolvedSlot};
use kore::Vfs;

use crate::app::theme;
use crate::common::item_icon;
use crate::widget::roster_list;
use crate::widget::{section, smooth_scroll};

const ICON_SIZE: f32 = 128.0 * 0.45;
const ICON_GAP_H: f32 = 4.0;
const ICON_GAP_V: f32 = 0.0;
const CARD_PADDING: f32 = 4.0;
const CARD_HEIGHT: f32 = 115.0;
const SCROLL_AREA_HEIGHT: f32 = CARD_HEIGHT - CARD_PADDING * 2.0;
const SCROLLBAR_SPACING: f32 = 6.0;
const UPGRADES_GAP: f32 = 24.0;
const GRID_SPACING: f32 = 2.0;
const GRID_COLUMN_GAP: f32 = 12.0;
const GRID_BLOCK_GAP: f32 = 6.0;
const CANNON_LABEL_WIDTH: f32 = 100.0;
const UPGRADE_LABEL_WIDTH: f32 = 110.0;
const CHAPTER_LABEL_WIDTH: f32 = 80.0;
const CELL_PADDING: [u16; 2] = [2, 4];
const TABLE_TEXT_SIZE: f32 = 11.0;
const TOOLTIP_PADDING: f32 = 8.0;

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<String, Handle>>,
}

impl State {
    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn icon(&self, path: &std::path::Path) -> Option<Handle> {
        let key = path.to_string_lossy().to_string();
        if let Some(cached) = self.icon_cache.borrow().get(&key) {
            return Some(cached.clone());
        }

        let (handle, _, _) = item_icon::load_cropped(path)?;
        self.icon_cache.borrow_mut().insert(key, handle.clone());
        Some(handle)
    }

    pub fn view<'a>(&'a self, resolved_lineup: &ResolvedFixedLineup, preset: &'a CertificationPreset, vfs: &'a Vfs) -> Element<'a, super::Message> {
        let mut top_row = row![].spacing(ICON_GAP_H);
        for slot in resolved_lineup.slots.iter().take(5) {
            top_row = top_row.push(self.slot_view(slot, preset, vfs));
        }

        let mut bottom_row = row![].spacing(ICON_GAP_H);
        for slot in resolved_lineup.slots.iter().skip(5).take(5) {
            bottom_row = bottom_row.push(self.slot_view(slot, preset, vfs));
        }

        let slots_col = column![top_row, bottom_row].spacing(ICON_GAP_V);

        let upgrades_scroller = scrollable(upgrades_section(preset))
            .height(Length::Fixed(SCROLL_AREA_HEIGHT))
            .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().spacing(SCROLLBAR_SPACING)));

        let upgrades_panel = container(smooth_scroll(upgrades_scroller))
            .padding(CARD_PADDING)
            .height(Length::Fixed(CARD_HEIGHT))
            .style(theme::card_container_outlined);

        let body = row![slots_col, upgrades_panel].spacing(UPGRADES_GAP).align_y(Alignment::Start);

        section("Fixed Lineup", Length::Fixed(super::CONTENT_WIDTH), body)
    }

    fn slot_view<'a>(&'a self, slot: &ResolvedSlot, preset: &'a CertificationPreset, vfs: &'a Vfs) -> Element<'a, super::Message> {
        let Some(image_path) = &slot.image_path else {
            return empty_slot();
        };

        let Some(handle) = self.icon(image_path) else {
            return empty_slot();
        };

        let image_el: Element<'a, super::Message> = iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).into();

        let (Some(unit_id), Some(unit_level)) = (slot.unit_id, slot.level) else {
            return image_el;
        };

        let padded_id = format!("{:03}", unit_id);
        let explanation = unitexplanation(vfs, unit_id);

        let form_index = preset.characters.get(&unit_id).map_or(0, |chara| match chara.evolution_form {
            EvolutionForm::Normal => 0,
            EvolutionForm::Evolved => 1,
            EvolutionForm::True => 2,
            EvolutionForm::Ultra => 3,
            _ => 0,
        });

        let mut display_name = format!("Unit {}", padded_id);
        if let Some(Some(name)) = explanation.names.get(form_index).or_else(|| explanation.names.first()) {
            display_name = name.clone();
        }

        let plus_level = slot.plus_level.unwrap_or(0);
        let plus_string = if plus_level > 0 { format!("+{}", plus_level) } else { String::new() };

        let tooltip_content = roster_list::tooltip_table([
            ("Name", display_name),
            ("ID", padded_id),
            ("Level", format!("{}{}", unit_level, plus_string)),
        ]);

        let clickable = button(image_el)
            .padding(0)
            .on_press(super::Message::JumpToUnit(unit_id, form_index, unit_level, plus_level))
            .style(|_theme: &Theme, _status| button::Style::default());

        tooltip(
            clickable,
            container(tooltip_content).padding(TOOLTIP_PADDING).style(container::bordered_box),
            tooltip::Position::Top,
        ).into()
    }
}

fn cell_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: Border::default().rounded(theme::RADIUS_SM),
        ..container::Style::default()
    }
}

fn empty_slot<'a>() -> Element<'a, super::Message> {
    container(space())
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(cell_style)
        .into()
}

fn header_cell<'a>(label: &'a str, width: f32) -> Element<'a, super::Message> {
    theme::bold_text(label)
        .size(TABLE_TEXT_SIZE)
        .width(Length::Fixed(width))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
}

fn zebra_table<'a>(label_header: &'a str, value_header: &'a str, label_width: f32, rows: Vec<(String, String)>) -> Element<'a, super::Message> {
    let header = row![header_cell(label_header, label_width), header_cell(value_header, label_width)].spacing(GRID_COLUMN_GAP);

    let mut table = column![
        container(header)
            .style(theme::zebra_table_header)
            .padding(CELL_PADDING)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
    ].spacing(GRID_SPACING);

    for (index, (label, value)) in rows.into_iter().enumerate() {
        let row_content = row![
            theme::table_cell_text(label, Length::Fixed(label_width)).size(TABLE_TEXT_SIZE),
            theme::table_cell_text(value, Length::Fixed(label_width)).size(TABLE_TEXT_SIZE),
        ].spacing(GRID_COLUMN_GAP);

        table = table.push(
            container(row_content)
                .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
                .padding(CELL_PADDING)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        );
    }

    table.into()
}

fn upgrades_section<'a>(preset: &'a CertificationPreset) -> Element<'a, super::Message> {
    let cannon_name = match preset.slot_cannon_type {
        CannonType::Basic => "Basic",
        CannonType::SlowBeam => "Slow Beam",
        CannonType::IronWall => "Iron Wall",
        CannonType::Thunderbolt => "Thunderbolt",
        CannonType::Waterblast => "Waterblast",
        CannonType::HolyBlast => "HolyBlast",
        CannonType::Breakerblast => "Breakerblast",
        CannonType::Curseblast => "Curseblast",
        CannonType::Unknown(_) => "Unknown",
    };
    let cannon_level = preset.cannon_levels.get(&preset.slot_cannon_type).copied().unwrap_or(0);

    let cannon_grid = zebra_table("Cannon", "Level", CANNON_LABEL_WIDTH, vec![(cannon_name.to_string(), cannon_level.to_string())]);

    const ABILITIES: [(AbilityType, &str); 10] = [
        (AbilityType::CatCannonAttack, "Cat Cannon Attack"),
        (AbilityType::CatCannonRange, "Cat Cannon Range"),
        (AbilityType::CatCannonCharge, "Cat Cannon Charge"),
        (AbilityType::WorkerCatRate, "Worker Cat Efficiency"),
        (AbilityType::WorkerCatWallet, "Worker Cat Wallet"),
        (AbilityType::BaseDefense, "Cat Base Health"),
        (AbilityType::Research, "Research"),
        (AbilityType::BountyUp, "Accounting"),
        (AbilityType::Study, "Study"),
        (AbilityType::CatEnergy, "Cat Energy"),
    ];

    let abilities_rows: Vec<(String, String)> = ABILITIES
        .iter()
        .map(|(ability_type, name)| {
            let level_string = preset.abilities.get(ability_type).map_or("0".to_string(), |ability| {
                if ability.plus_level > 0 { format!("{} +{}", ability.level, ability.plus_level) } else { ability.level.to_string() }
            });
            (name.to_string(), level_string)
        })
        .collect();
    let abilities_grid = zebra_table("Upgrade", "Level", UPGRADE_LABEL_WIDTH, abilities_rows);

    const TREASURES: [(TreasureType, &str); 9] = [
        (TreasureType::EoC1, "EoC Ch. 1"),
        (TreasureType::EoC2, "EoC Ch. 2"),
        (TreasureType::EoC3, "EoC Ch. 3"),
        (TreasureType::ItF1, "ItF Ch. 1"),
        (TreasureType::ItF2, "ItF Ch. 2"),
        (TreasureType::ItF3, "ItF Ch. 3"),
        (TreasureType::CotC1, "CotC Ch. 1"),
        (TreasureType::CotC2, "CotC Ch. 2"),
        (TreasureType::CotC3, "CotC Ch. 3"),
    ];

    let treasures_rows: Vec<(String, String)> = TREASURES
        .iter()
        .map(|(treasure_type, name)| {
            let grades_string = preset.treasures.get(treasure_type).map_or("0/0/0".to_string(), |treasure| {
                format!("{}/{}/{}", treasure.inferior_count, treasure.normal_count, treasure.superior_count)
            });
            (name.to_string(), grades_string)
        })
        .collect();
    let treasures_grid = zebra_table("Chapter", "Treasure", CHAPTER_LABEL_WIDTH, treasures_rows);

    column![
        cannon_grid,
        row![abilities_grid, treasures_grid].spacing(GRID_COLUMN_GAP),
    ].spacing(GRID_BLOCK_GAP).into()
}
