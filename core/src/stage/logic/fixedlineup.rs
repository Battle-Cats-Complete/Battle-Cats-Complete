use std::path::PathBuf;
use tracing::warn;

use crate::global::resolver;
use nyanko::chapter::stage::{CertificationPreset, PresetChara, EvolutionForm};

pub struct ResolvedSlot {
    pub unit_id: Option<u32>,
    pub level: Option<u16>,
    pub plus_level: Option<u16>,
    pub image_path: Option<PathBuf>,
}

pub struct ResolvedFixedLineup {
    pub slots: Vec<ResolvedSlot>,
}

fn get_file_letter(evolution_form: &EvolutionForm) -> &'static str {
    match evolution_form {
        EvolutionForm::Normal => "f",
        EvolutionForm::Evolved => "c",
        EvolutionForm::True => "s",
        EvolutionForm::Ultra => "u",
        EvolutionForm::Unknown(_) => "f",
    }
}

fn resolve_empty_slot(active_language_priority_array: &[String]) -> ResolvedSlot {
    let fallback_directory = PathBuf::from("game/cats");
    let fallback_file_name = "uni.png".to_string();

    let resolved_fallback_path = resolver::get(
        &fallback_directory,
        [&fallback_file_name],
        active_language_priority_array
    ).into_iter().next();

    ResolvedSlot {
        unit_id: None,
        level: None,
        plus_level: None,
        image_path: resolved_fallback_path,
    }
}

fn resolve_populated_slot(
    unit_id: u32,
    character_data: &PresetChara,
    active_language_priority_array: &[String],
) -> ResolvedSlot {
    let form_letter = get_file_letter(&character_data.evolution_form);

    let image_directory_path = PathBuf::from(format!("game/cats/{:03}/{}", unit_id, form_letter));
    let image_file_name = format!("uni{:03}_{}00.png", unit_id, form_letter);

    let resolved_image_path = resolver::get(
        &image_directory_path,
        [&image_file_name],
        active_language_priority_array
    ).into_iter().next();

    if resolved_image_path.is_none() {
        warn!("Missing unit icon for unit {} form {}", unit_id, form_letter);
    }

    ResolvedSlot {
        unit_id: Some(unit_id),
        level: Some(character_data.level),
        plus_level: Some(character_data.plus_level),
        image_path: resolved_image_path,
    }
}

pub fn resolve_lineup(
    preset_lineup_data: &CertificationPreset,
    active_language_priority_array: &[String],
) -> ResolvedFixedLineup {
    let mut resolved_slots_array = Vec::with_capacity(10);

    for slot_index in 0..10 {
        let Some(&target_unit_id) = preset_lineup_data.slot_units.get(slot_index) else {
            resolved_slots_array.push(resolve_empty_slot(active_language_priority_array));
            continue;
        };

        let Some(target_character_data) = preset_lineup_data.characters.get(&target_unit_id) else {
            resolved_slots_array.push(resolve_empty_slot(active_language_priority_array));
            continue;
        };

        let populated_slot = resolve_populated_slot(
            target_unit_id,
            target_character_data,
            active_language_priority_array
        );

        resolved_slots_array.push(populated_slot);
    }

    ResolvedFixedLineup {
        slots: resolved_slots_array,
    }
}