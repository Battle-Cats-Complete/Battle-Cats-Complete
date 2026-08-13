use std::collections::HashMap;

use nyanko::chapter::Category;
use tracing::debug;

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;

const CURRENCY_SCALE: u32 = 1000;

pub struct ResolvedCost {
    pub header: String,
    pub value: String,
}

fn plain(energy: u32) -> ResolvedCost {
    ResolvedCost { header: "Energy".to_string(), value: energy.to_string() }
}

fn catamin(energy: u32) -> ResolvedCost {
    let grade = match energy / CURRENCY_SCALE {
        0 => "A",
        1 => "B",
        _ => "C",
    };

    ResolvedCost {
        header: "Catamin".to_string(),
        value: format!("{}{}", energy % CURRENCY_SCALE, grade),
    }
}

fn short_label(name: &str) -> String {
    let Some(last_word) = name.split_whitespace().next_back() else {
        return name.to_string();
    };

    if last_word.ends_with('s') {
        return last_word.to_string();
    }

    format!("{}s", last_word)
}

pub fn resolve_cost(
    category: &Category,
    energy: u32,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
) -> ResolvedCost {
    if *category == Category::CataminStages {
        return catamin(energy);
    }

    let item_id = energy / CURRENCY_SCALE;
    let amount = energy % CURRENCY_SCALE;

    if item_id == 0 || amount == 0 {
        return plain(energy);
    }

    let Some(item_buy) = item_buy_registry.get(&item_id) else {
        return plain(energy);
    };

    let Some(item_name) = item_name_registry.get(&item_buy.row_index) else {
        return plain(energy);
    };

    debug!(item_id, amount, name = %item_name.name, "Resolving stage cost as item currency");

    ResolvedCost {
        header: short_label(&item_name.name),
        value: amount.to_string(),
    }
}
