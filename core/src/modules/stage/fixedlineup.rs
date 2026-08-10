use tracing::warn;

use nyanko::chapter::stage::{CertificationPreset, EvolutionForm, PresetChara};

use crate::Vfs;

use super::paths;

pub struct ResolvedSlot {
    pub unit_id: Option<u32>,
    pub level: Option<u16>,
    pub plus_level: Option<u16>,
    pub image_path: Option<std::path::PathBuf>,
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

fn resolve_empty_slot(vfs: &Vfs) -> ResolvedSlot {
    let resolved_fallback_path = vfs.list(&paths::empty_cat_icon()).into_iter().next();

    ResolvedSlot {
        unit_id: None,
        level: None,
        plus_level: None,
        image_path: resolved_fallback_path,
    }
}

fn resolve_populated_slot(
    vfs: &Vfs,
    unit_id: u32,
    character_data: &PresetChara,
) -> ResolvedSlot {
    let form_letter = get_file_letter(&character_data.evolution_form);

    let resolved_image_path = vfs.list(&paths::cat_form_img(unit_id, form_letter)).into_iter().next();

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
    vfs: &Vfs,
    preset_lineup_data: &CertificationPreset,
) -> ResolvedFixedLineup {
    let mut resolved_slots_array = Vec::with_capacity(10);

    for slot_index in 0..10 {
        let Some(&target_unit_id) = preset_lineup_data.slot_units.get(slot_index) else {
            resolved_slots_array.push(resolve_empty_slot(vfs));
            continue;
        };

        let Some(target_character_data) = preset_lineup_data.characters.get(&target_unit_id) else {
            resolved_slots_array.push(resolve_empty_slot(vfs));
            continue;
        };

        let populated_slot = resolve_populated_slot(
            vfs,
            target_unit_id,
            target_character_data
        );

        resolved_slots_array.push(populated_slot);
    }

    ResolvedFixedLineup {
        slots: resolved_slots_array,
    }
}