use std::collections::HashMap;
use std::path::Path;

use crate::common::formats::gatyaitembuy::GatyaItemBuy;
use crate::common::formats::gatyaitemname::GatyaItemName;
use crate::modules::stage::paths;
use super::treasure::ResolvedDrop;

pub const MAT_IDS: [u32; 16] = [
    85, 86, 87, 88, 89, 90, 91, 140,
    187, 188, 189, 190, 191, 192, 193, 194,
];

pub fn resolve(
    idx: usize,
    amt: u32,
    buy: &HashMap<u32, GatyaItemBuy>,
    names: &HashMap<usize, GatyaItemName>,
    langs: &[String]
) -> ResolvedDrop {
    let Some(&item_id) = MAT_IDS.get(idx) else {
        return ResolvedDrop {
            name: format!("ID {}", idx),
            image_path: None,
            amount_display: amt.to_string(),
        };
    };

    let Some(buy_data) = buy.get(&item_id) else {
        return ResolvedDrop {
            name: format!("ID {}", item_id),
            image_path: None,
            amount_display: amt.to_string(),
        };
    };

    let target_row = buy_data.row_index;
    let name = names.get(&target_row)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| format!("ID {}", item_id));

    let img_id = if buy_data.img_id != -1 {
        buy_data.img_id as u32
    } else {
        buy_data.row_index as u32
    };

    let dir = Path::new(paths::DIR_GATYA_ITEM);
    let file = paths::gatya_item_img(img_id);
    let image_path = crate::common::resolver::get(dir, [&file], langs).into_iter().next();

    ResolvedDrop {
        name,
        image_path,
        amount_display: amt.to_string(),
    }
}