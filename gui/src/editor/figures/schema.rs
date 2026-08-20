use std::borrow::Cow;
use std::sync::LazyLock;

use nyanko::cat::unit::UnitBuy;
use nyanko::cat::unitid;
use nyanko::combat::Scale;
use nyanko::common::tools::columns::{Column, FromColumn};
use nyanko::enemy::t_unit;

use kore::domains::settings::EditorValues;

use crate::app::Page;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    Cat,
    Enemy,
    Buy,
    Curve,
}

pub(crate) const COUNT: usize = 4;

pub(crate) const SUBJECTS: [Subject; COUNT] =
    [Subject::Cat, Subject::Enemy, Subject::Buy, Subject::Curve];

impl Subject {
    pub(crate) fn slot(self) -> usize {
        match self {
            Subject::Cat => 0,
            Subject::Enemy => 1,
            Subject::Buy => 2,
            Subject::Curve => 3,
        }
    }

    pub(crate) fn page(self) -> Page {
        match self {
            Subject::Enemy => Page::Enemies,
            Subject::Cat | Subject::Buy | Subject::Curve => Page::Cats,
        }
    }
}

pub(super) struct Schema {
    subject: Subject,
    comments: bool,
}

pub(super) static CAT: Schema = Schema { subject: Subject::Cat, comments: true };

pub(super) static ENEMY: Schema = Schema { subject: Subject::Enemy, comments: false };

pub(super) static BUY: Schema = Schema { subject: Subject::Buy, comments: false };

pub(super) static CURVE: Schema = Schema { subject: Subject::Curve, comments: false };

pub(super) fn of(subject: Subject) -> &'static Schema {
    match subject {
        Subject::Cat => &CAT,
        Subject::Enemy => &ENEMY,
        Subject::Buy => &BUY,
        Subject::Curve => &CURVE,
    }
}

pub(super) const BRACKET: usize = 10;

const BRACKETS: usize = 20;

const FIRST_GROWTH_LEVEL: usize = 2;

pub(super) struct Entry {
    pub(super) field: &'static str,
    index: usize,
    scale: Scale,
    pub(super) default: &'static str,
}

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

pub(crate) const FORMS: [&str; 4] = ["Normal", "Evolved", "True", "Ultra"];

const BUY_NAMES: &[(&str, &str)] = &[
    ("stage_unlock_requirement", "Stage Unlock"),
    ("chapter_unlock_requirement", "Chapter Unlock"),
    ("sell_xp_yield", "Sell XP"),
    ("sell_np_yield", "Sell NP"),
    ("level_cap_ch2", "Level Cap Ch2"),
    ("evolve_level_xp", "Evolve Level XP"),
    ("egg_id_normal", "Egg ID Normal"),
    ("egg_id_evolved", "Egg ID Evolved"),
];

static CAT_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| labels(&CAT_ORDER, CAT_NAMES));

static ENEMY_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| labels(&ENEMY_ORDER, ENEMY_NAMES));

static BUY_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| labels(&BUY_ORDER, BUY_NAMES));

static CAT_ORDER: LazyLock<Vec<Entry>> = LazyLock::new(|| order(unitid::COLUMNS));

static ENEMY_ORDER: LazyLock<Vec<Entry>> = LazyLock::new(|| order(t_unit::COLUMNS));

static BUY_ORDER: LazyLock<Vec<Entry>> = LazyLock::new(|| order(UnitBuy::COLUMNS));

fn order<T>(columns: &'static [Column<T>]) -> Vec<Entry> {
    let mut sorted: Vec<Entry> = columns
        .iter()
        .map(|column| Entry {
            field: column.field,
            index: column.index,
            scale: column.scale,
            default: column.default,
        })
        .collect();

    sorted.sort_by_key(|entry| entry.index);

    sorted
}

fn labels(order: &[Entry], names: &[(&str, &str)]) -> Vec<String> {
    order
        .iter()
        .map(|entry| {
            names
                .iter()
                .find(|(field, _)| *field == entry.field)
                .map_or_else(|| prettify(entry.field), |(_, label)| (*label).to_owned())
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
    pub(super) fn order(&self) -> &'static [Entry] {
        match self.subject {
            Subject::Cat => &CAT_ORDER,
            Subject::Enemy => &ENEMY_ORDER,
            Subject::Buy => &BUY_ORDER,
            Subject::Curve => &[],
        }
    }

    fn column(&self, index: usize) -> Option<&'static Entry> {
        self.order().get(index)
    }

    pub(super) fn subject(&self) -> Subject {
        self.subject
    }

    pub(super) fn comments(&self) -> bool {
        self.comments
    }

    pub(super) fn known(&self) -> usize {
        match self.subject {
            Subject::Curve => BRACKETS,
            _ => self.order().len(),
        }
    }

    pub(super) fn label(&self, index: usize) -> Cow<'static, str> {
        if self.subject == Subject::Curve {
            let first = (index * BRACKET + 1).max(FIRST_GROWTH_LEVEL);

            return Cow::Owned(format!("Levels {first}-{}", (index + 1) * BRACKET));
        }

        let table = match self.subject {
            Subject::Cat => &CAT_LABELS,
            Subject::Enemy => &ENEMY_LABELS,
            _ => &BUY_LABELS,
        };

        table
            .get(index)
            .map_or_else(|| Cow::Owned(format!("Column {}", index + 1)), |label| Cow::Borrowed(label.as_str()))
    }

    pub(super) fn to_display(&self, index: usize, raw: i32, values: EditorValues) -> i32 {
        if values == EditorValues::Raw {
            return raw;
        }

        self.column(index).map_or(raw, |column| column.scale.apply(raw))
    }

    pub(super) fn to_raw(&self, index: usize, display: i32, values: EditorValues) -> i32 {
        if values == EditorValues::Raw {
            return display;
        }

        match self.column(index).map(|column| column.scale) {
            Some(Scale::Double) => display / 2,
            Some(Scale::Quarter) => display * 4,
            _ => display,
        }
    }

    pub(super) fn fallback(&self, index: usize) -> i32 {
        self.column(index).and_then(|column| i32::from_column(column.default)).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{FromColumn, CAT, CAT_NAMES, ENEMY, ENEMY_NAMES, Schema};

    fn check(schema: &Schema, names: &[(&str, &str)], subject: &str) {
        for (field, _) in names {
            assert!(
                schema.order().iter().any(|column| column.field == *field),
                "{subject}: override names {field}, which nyanko no longer publishes"
            );
        }
    }

    fn defaults_parse(schema: &Schema, subject: &str) {
        for column in schema.order() {
            assert!(
                i32::from_column(column.default).is_some(),
                "{subject}: {} declares default {:?}, which is not an integer and would silently fall back to 0",
                column.field,
                column.default
            );
        }
    }

    #[test]
    fn cat_defaults_are_integers() {
        defaults_parse(&CAT, "cat");
    }

    #[test]
    fn enemy_defaults_are_integers() {
        defaults_parse(&ENEMY, "enemy");
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
