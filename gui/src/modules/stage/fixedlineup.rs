use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, scrollable, space, text, tooltip};
use iced::{Alignment, Border, Element, Length, Theme};
use nyanko::chapter::stage::{AbilityType, CannonType, CertificationPreset, EvolutionForm, TreasureType};

use core::modules::cat::waiter::unitexplanation;
use core::modules::stage::fixedlineup::{ResolvedFixedLineup, ResolvedSlot};
use core::Vfs;

use crate::app::theme;
use crate::common::item_icon;
use crate::widget::{section, smooth_scroll};

const ICON_SIZE: f32 = 128.0 * 0.45;
const ICON_SPACING: f32 = 8.0;
const SCROLL_AREA_HEIGHT: f32 = 90.0;
const UPGRADES_GAP: f32 = 24.0;
const GRID_SPACING: f32 = 4.0;
const GRID_COLUMN_GAP: f32 = 24.0;
const GRID_BLOCK_GAP: f32 = 12.0;
const CANNON_LABEL_WIDTH: f32 = 140.0;
const UPGRADE_LABEL_WIDTH: f32 = 160.0;
const CHAPTER_LABEL_WIDTH: f32 = 110.0;

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
        let mut top_row = row![].spacing(ICON_SPACING);
        for slot in resolved_lineup.slots.iter().take(5) {
            top_row = top_row.push(self.slot_view(slot, preset, vfs));
        }

        let mut bottom_row = row![].spacing(ICON_SPACING);
        for slot in resolved_lineup.slots.iter().skip(5).take(5) {
            bottom_row = bottom_row.push(self.slot_view(slot, preset, vfs));
        }

        let slots_col = column![top_row, bottom_row].spacing(ICON_SPACING);

        let upgrades_panel = container(
            smooth_scroll(
                scrollable(upgrades_section(preset))
                    .height(Length::Fixed(SCROLL_AREA_HEIGHT))
            )
        )
            .padding(4)
            .style(|theme: &Theme| container::Style {
                border: Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 2.0,
                    radius: theme::RADIUS_SM.into(),
                },
                ..container::Style::default()
            });

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

        let image_el = iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE));

        let (Some(unit_id), Some(unit_level)) = (slot.unit_id, slot.level) else {
            return image_el.into();
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

        let tooltip_content = column![
            row![text("[Name] ").size(12), text(display_name)].align_y(Alignment::Center),
            row![text("[ID] ").size(12), text(padded_id)].align_y(Alignment::Center),
            row![text("[Level] ").size(12), text(format!("{}{}", unit_level, plus_string))].align_y(Alignment::Center),
        ].spacing(2);

        tooltip(
            image_el,
            container(tooltip_content).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        ).into()
    }
}

fn empty_slot<'a>() -> Element<'a, super::Message> {
    container(space())
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            border: Border::default().rounded(theme::RADIUS_SM),
            ..container::Style::default()
        })
        .into()
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

    let cannon_grid = column![
        row![theme::bold_text("Cannon").width(Length::Fixed(CANNON_LABEL_WIDTH)), theme::bold_text("Level")].spacing(GRID_COLUMN_GAP),
        row![text(cannon_name).width(Length::Fixed(CANNON_LABEL_WIDTH)), text(cannon_level.to_string())].spacing(GRID_COLUMN_GAP),
    ].spacing(GRID_SPACING);

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

    let mut abilities_grid = column![
        row![theme::bold_text("Upgrade").width(Length::Fixed(UPGRADE_LABEL_WIDTH)), theme::bold_text("Level")].spacing(GRID_COLUMN_GAP)
    ].spacing(GRID_SPACING);
    for (ability_type, name) in ABILITIES {
        let level_string = preset.abilities.get(&ability_type).map_or("0".to_string(), |ability| {
            if ability.plus_level > 0 { format!("{} +{}", ability.level, ability.plus_level) } else { ability.level.to_string() }
        });
        abilities_grid = abilities_grid.push(row![text(name).width(Length::Fixed(UPGRADE_LABEL_WIDTH)), text(level_string)].spacing(GRID_COLUMN_GAP));
    }

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

    let mut treasures_grid = column![
        row![theme::bold_text("Chapter").width(Length::Fixed(CHAPTER_LABEL_WIDTH)), theme::bold_text("Grades")].spacing(GRID_COLUMN_GAP)
    ].spacing(GRID_SPACING);
    for (treasure_type, name) in TREASURES {
        let grades_string = preset.treasures.get(&treasure_type).map_or("0/0/0".to_string(), |treasure| {
            format!("{}/{}/{}", treasure.superior_count, treasure.normal_count, treasure.inferior_count)
        });
        treasures_grid = treasures_grid.push(row![text(name).width(Length::Fixed(CHAPTER_LABEL_WIDTH)), text(grades_string)].spacing(GRID_COLUMN_GAP));
    }

    column![
        cannon_grid,
        row![abilities_grid, treasures_grid].spacing(GRID_COLUMN_GAP),
    ].spacing(GRID_BLOCK_GAP).into()
}
