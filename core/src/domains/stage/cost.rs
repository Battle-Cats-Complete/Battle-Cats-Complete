use std::collections::HashMap;

use nyanko::chapter::Stage;
use nyanko::chapter::stage::{CataminGrade, CostType};
use tracing::debug;

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;

pub struct CostDisplay {
    pub header: String,
    pub value: String,
}

pub fn grade_label(grade: CataminGrade) -> &'static str {
    match grade {
        CataminGrade::B => "B",
        CataminGrade::C => "C",
        _ => "A",
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

fn item_label(
    item_id: u32,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
) -> Option<String> {
    let item_buy = item_buy_registry.get(&item_id)?;
    let item_name = item_name_registry.get(&item_buy.row_index)?;

    Some(short_label(&item_name.name))
}

pub fn resolve_cost(
    stage: &Stage,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    item_name_registry: &HashMap<usize, GatyaItemName>,
) -> CostDisplay {
    let resolved = stage.resolved_cost();

    match stage.cost_type() {
        CostType::Catamin => {
            let grade = resolved.id.map_or(CataminGrade::A, CataminGrade::from_key);

            CostDisplay {
                header: "Catamin".to_string(),
                value: format!("{}{}", resolved.value, grade_label(grade)),
            }
        }
        CostType::Item => {
            let label = resolved.id
                .and_then(|item_id| item_label(item_id, item_buy_registry, item_name_registry));

            debug!(item_id = ?resolved.id, amount = resolved.value, ?label, "Resolving stage cost as item currency");

            CostDisplay {
                header: label.unwrap_or_else(|| "Items".to_string()),
                value: resolved.value.to_string(),
            }
        }
        _ => CostDisplay {
            header: "Energy".to_string(),
            value: resolved.value.to_string(),
        },
    }
}
