use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nyanko::cat::unit::UnitBuy;

use crate::cat::waiter::unitexplanation;
use crate::global::formats::gatyaitembuy::GatyaItemBuy;
use crate::global::formats::gatyaitemname::GatyaItemName;
use crate::stage::paths;

pub struct ResolvedDrop {
    pub name: String,
    pub image_path: Option<PathBuf>,
    pub amount_display: String,
}

fn resolve_cat_icon(
    unit_id: u32,
    form_index: usize,
    unit_buy_registry: &HashMap<u32, UnitBuy>,
    langs: &[String]
) -> Option<PathBuf> {
    let default_egg_ids = (-1, -1);
    let egg_id_tuple = unit_buy_registry.get(&unit_id)
        .map(|unit_buy_data| (unit_buy_data.egg_id_normal, unit_buy_data.egg_id_evolved))
        .unwrap_or(default_egg_ids);

    let form_str = match form_index {
        0 => "f",
        1 => "c",
        2 => "s",
        3 => "u",
        _ => "f",
    };

    let img_dir = paths::cat_form_folder(unit_id, form_str);
    let img_file = paths::cat_form_img(unit_id, form_str);

    let resolved_primary_icon = crate::global::resolver::get(
        &img_dir,
        [&img_file],
        langs
    ).into_iter().next();

    if resolved_primary_icon.is_some() {
        return resolved_primary_icon;
    }

    let target_egg_id = if form_index == 0 { egg_id_tuple.0 } else { egg_id_tuple.1 };

    if target_egg_id != -1 {
        let fallback_name = format!("uni{:03}_m00.png", target_egg_id);
        return crate::global::resolver::get(
            &img_dir,
            [&fallback_name],
            langs
        ).into_iter().next();
    }

    None
}

pub fn resolve_drop(
    target_item_id: u32,
    raw_amount: u32,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuy>,
    langs: &[String]
) -> ResolvedDrop {

    if let Some(located_item_unitbuy) = item_buy_registry.get(&target_item_id) {
        let target_name_row_index = located_item_unitbuy.row_index;
        let name = item_name_registry.get(&target_name_row_index)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| target_item_id.to_string());

        let resolved_image_identifier = if located_item_unitbuy.img_id != -1 {
            located_item_unitbuy.img_id as u32
        } else {
            located_item_unitbuy.row_index as u32
        };

        let gatya_dir = Path::new(paths::DIR_GATYA_ITEM);
        let gatya_img = paths::gatya_item_img(resolved_image_identifier);

        let image_path = crate::global::resolver::get(gatya_dir, [&gatya_img], langs).into_iter().next();

        return ResolvedDrop {
            name,
            image_path,
            amount_display: raw_amount.to_string(),
        };
    }

    if let Some(&located_chara_id) = drop_chara_registry.get(&target_item_id) {
        let cat_folder = paths::cat_folder(located_chara_id);
        let explanation = unitexplanation(located_chara_id, &cat_folder, langs);

        let mut name = format!("{}-1", located_chara_id);
        if let Some(first_form_name) = &explanation.names[0] {
            name = first_form_name.clone();
        }

        let image_path = resolve_cat_icon(located_chara_id, 0, unit_buy_registry, langs);

        return ResolvedDrop {
            name,
            image_path,
            amount_display: "-".to_string(),
        };
    }

    if let Some((&unit_id, _)) = unit_buy_registry.iter().find(|(_, row_data)| row_data.true_form_id == target_item_id as i32) {
        let cat_folder = paths::cat_folder(unit_id);
        let explanation = unitexplanation(unit_id, &cat_folder, langs);

        let mut name = format!("{}-3", unit_id);
        if let Some(true_form_name) = &explanation.names[2] {
            name = true_form_name.clone();
        }

        let image_path = resolve_cat_icon(unit_id, 1, unit_buy_registry, langs);

        return ResolvedDrop {
            name,
            image_path,
            amount_display: "-".to_string(),
        };
    }

    ResolvedDrop {
        name: target_item_id.to_string(),
        image_path: None,
        amount_display: raw_amount.to_string(),
    }
}