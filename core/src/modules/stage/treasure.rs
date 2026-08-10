use std::collections::HashMap;
use std::path::PathBuf;

use nyanko::cat::unit::UnitBuy;
use tracing::{debug, trace};

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;
use crate::modules::cat::waiter::unitexplanation;
use crate::Vfs;

use super::files;

pub struct ResolvedDrop {
    pub name: String,
    pub image_path: Option<PathBuf>,
    pub amount_display: String,
}

fn resolve_cat_icon(
    vfs: &Vfs,
    unit_id: u32,
    form_index: usize,
    unit_buy_registry: &HashMap<u32, UnitBuy>
) -> Option<PathBuf> {
    let default_egg = (-1, -1);
    let egg_ids = unit_buy_registry.get(&unit_id)
        .map(|buy_data| (buy_data.egg_id_normal, buy_data.egg_id_evolved))
        .unwrap_or(default_egg);

    let form_str = match form_index {
        0 => "f",
        1 => "c",
        2 => "s",
        3 => "u",
        _ => "f",
    };

    trace!(unit_id, form_index, "Attempting to resolve primary cat icon");

    let primary_icon = vfs.find(&files::cat_form_img(unit_id, form_str));

    if primary_icon.is_some() {
        return primary_icon;
    }

    let target_egg = if form_index == 0 { egg_ids.0 } else { egg_ids.1 };

    if target_egg != -1 {
        trace!(unit_id, target_egg, "Falling back to egg icon");
        let fallback_name = format!("uni{:03}_m00.png", target_egg);
        return vfs.find(&fallback_name);
    }

    None
}

pub fn resolve_drop(
    vfs: &Vfs,
    target_item_id: u32,
    raw_amount: u32,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuy>
) -> ResolvedDrop {

    if let Some(item_buy) = item_buy_registry.get(&target_item_id) {
        debug!(target_item_id, "Resolving regular item drop");
        let name_idx = item_buy.row_index;
        let name = item_name_registry.get(&name_idx)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| target_item_id.to_string());

        let img_id = if item_buy.img_id != -1 {
            item_buy.img_id as u32
        } else {
            item_buy.row_index as u32
        };

        let image_path = vfs.find(&files::gatya_item_img(img_id));

        return ResolvedDrop {
            name,
            image_path,
            amount_display: raw_amount.to_string(),
        };
    }

    if let Some(&chara_id) = drop_chara_registry.get(&target_item_id) {
        debug!(chara_id, target_item_id, "Resolving base cat drop");
        let explanation = unitexplanation(vfs, chara_id);

        let mut name = format!("{}-1", chara_id);
        if let Some(first_form) = &explanation.names[0] {
            name = first_form.clone();
        }

        let image_path = resolve_cat_icon(vfs, chara_id, 0, unit_buy_registry);

        return ResolvedDrop {
            name,
            image_path,
            amount_display: "-".to_string(),
        };
    }

    if let Some((&unit_id, _)) = unit_buy_registry.iter().find(|(_, row)| row.true_form_id == target_item_id as i32) {
        debug!(unit_id, target_item_id, "Resolving true form cat drop");
        let explanation = unitexplanation(vfs, unit_id);

        let mut name = format!("{}-3", unit_id);
        if let Some(true_form) = &explanation.names[2] {
            name = true_form.clone();
        }

        let image_path = resolve_cat_icon(vfs, unit_id, 2, unit_buy_registry);

        return ResolvedDrop {
            name,
            image_path,
            amount_display: "-".to_string(),
        };
    }

    debug!(target_item_id, "Fallback for unresolved drop");
    ResolvedDrop {
        name: target_item_id.to_string(),
        image_path: None,
        amount_display: raw_amount.to_string(),
    }
}