use std::borrow::Cow;
use std::sync::LazyLock;

use nyanko::cat::unitid;
use nyanko::combat::{Column, Scale};
use nyanko::enemy::t_unit;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    Cat,
    Enemy,
}

pub(super) struct Schema {
    subject: Subject,
    comments: bool,
}

pub(super) static CAT: Schema = Schema { subject: Subject::Cat, comments: true };

pub(super) static ENEMY: Schema = Schema { subject: Subject::Enemy, comments: false };

const CAT_NAMES: &[(&str, &str)] = &[
    ("hitpoints", "Base Hitpoints"),
    ("attack_1", "Attack 1 Base Damage"),
    ("eoc1_cost", "EoC1 Cost"),
    ("trait_red", "Target Red"),
    ("trait_floating", "Target Floating"),
    ("trait_dark", "Target Dark"),
    ("trait_metal", "Target Metal"),
    ("trait_traitless", "Target Traitless"),
    ("trait_angel", "Target Angel"),
    ("trait_alien", "Target Alien"),
    ("trait_zombie", "Target Zombie"),
    ("is_metal", "Metal"),
    ("trait_witch", "Target Witch"),
    ("attack_2", "Attack 2 Base Damage"),
    ("attack_3", "Attack 3 Base Damage"),
    ("trait_eva", "Target Eva"),
    ("trait_relic", "Target Relic"),
    ("trait_aku", "Target Aku"),
];

const ENEMY_NAMES: &[(&str, &str)] = &[
    ("hitpoints", "Base Hitpoints"),
    ("attack_1", "Attack 1 Base Damage"),
    ("attack_2", "Attack 2 Base Damage"),
    ("attack_3", "Attack 3 Base Damage"),
];

static CAT_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| labels(unitid::COLUMNS, CAT_NAMES));

static ENEMY_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| labels(t_unit::COLUMNS, ENEMY_NAMES));

static CAT_ORDER: LazyLock<Vec<&'static Column>> = LazyLock::new(|| order(unitid::COLUMNS));

static ENEMY_ORDER: LazyLock<Vec<&'static Column>> = LazyLock::new(|| order(t_unit::COLUMNS));

fn order(columns: &'static [Column]) -> Vec<&'static Column> {
    let mut sorted: Vec<&Column> = columns.iter().collect();
    sorted.sort_by_key(|column| column.index);

    sorted
}

fn labels(columns: &'static [Column], names: &[(&str, &str)]) -> Vec<String> {
    order(columns)
        .into_iter()
        .map(|column| {
            names
                .iter()
                .find(|(field, _)| *field == column.field)
                .map_or_else(|| prettify(column.field), |(_, label)| (*label).to_owned())
        })
        .collect()
}

fn prettify(field: &str) -> String {
    field
        .split('_')
        .map(|word| {
            let mut chars = word.chars();

            chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
        })
        .collect::<Vec<String>>()
        .join(" ")
}

impl Schema {
    fn order(&self) -> &'static [&'static Column] {
        match self.subject {
            Subject::Cat => &CAT_ORDER,
            Subject::Enemy => &ENEMY_ORDER,
        }
    }

    fn column(&self, index: usize) -> Option<&'static Column> {
        self.order().get(index).copied()
    }

    pub(super) fn subject(&self) -> Subject {
        self.subject
    }

    pub(super) fn comments(&self) -> bool {
        self.comments
    }

    pub(super) fn known(&self) -> usize {
        self.order().len()
    }

    pub(super) fn label(&self, index: usize) -> Cow<'static, str> {
        let table = match self.subject {
            Subject::Cat => &CAT_LABELS,
            Subject::Enemy => &ENEMY_LABELS,
        };

        table
            .get(index)
            .map_or_else(|| Cow::Owned(format!("Column {}", index + 1)), |label| Cow::Borrowed(label.as_str()))
    }

    pub(super) fn to_display(&self, index: usize, raw: i32) -> i32 {
        self.column(index).map_or(raw, |column| column.scale.apply(raw))
    }

    pub(super) fn to_raw(&self, index: usize, display: i32) -> i32 {
        match self.column(index).map(|column| column.scale) {
            Some(Scale::Double) => display / 2,
            Some(Scale::Quarter) => display * 4,
            _ => display,
        }
    }

    pub(super) fn fallback(&self, index: usize) -> i32 {
        self.column(index).map_or(0, |column| column.default)
    }
}

#[cfg(test)]
mod tests {
    use super::{CAT, CAT_NAMES, ENEMY, ENEMY_NAMES, Schema};

    fn check(schema: &Schema, names: &[(&str, &str)], subject: &str) {
        for (field, _) in names {
            assert!(
                schema.order().iter().any(|column| column.field == *field),
                "{subject}: override names {field}, which nyanko no longer publishes"
            );
        }
    }

    #[test]
    fn cat_overrides_match_nyanko_fields() {
        check(&CAT, CAT_NAMES, "cat");
    }

    #[test]
    fn enemy_overrides_match_nyanko_fields() {
        check(&ENEMY, ENEMY_NAMES, "enemy");
    }
}
