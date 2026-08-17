use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HardcodedType {
    Oldest,
}

const CATS: [[&str; 5]; 1] = [
    ["155", "f", "c", "", ""],
];

const LANGUAGES: [&str; 9] = ["de", "en", "es", "fr", "it", "ja", "ko", "th", "tw"];

pub(crate) fn generate_rules() -> HashMap<String, HardcodedType> {
    let mut files = HashMap::new();

    for cat in CATS {
        let id_str = cat[0];
        if id_str.is_empty() {
            continue;
        }

        if let Ok(id_num) = id_str.parse::<u32>() {
            let csv_id = id_num + 1;

            files.insert(format!("unit{}.csv", csv_id), HardcodedType::Oldest);

            for lang in LANGUAGES {
                files.insert(format!("Unit_Explanation{}_{}.csv", csv_id, lang), HardcodedType::Oldest);
            }
        }

        for form in &cat[1..] {
            if form.is_empty() {
                continue;
            }

            files.insert(format!("{}_{}.imgcut", id_str, form), HardcodedType::Oldest);
            files.insert(format!("{}_{}.mamodel", id_str, form), HardcodedType::Oldest);
            files.insert(format!("{}_{}.png", id_str, form), HardcodedType::Oldest);

            for index in 0..=3 {
                files.insert(format!("{}_{}{:02}.maanim", id_str, form, index), HardcodedType::Oldest);
            }

            files.insert(format!("udi{}_{}.png", id_str, form), HardcodedType::Oldest);
            files.insert(format!("uni{}_{}00.png", id_str, form), HardcodedType::Oldest);
            files.insert(format!("gatyachara_{}_{}.png", id_str, form), HardcodedType::Oldest);
        }
    }

    tracing::trace!("Generated {} hardcoded file exceptions", files.len());

    files
}