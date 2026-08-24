#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    Percent,
    Multiplier,
    Frames,
    Count,
    Range,
    Money,
}

impl Unit {
    fn name(self) -> &'static str {
        match self {
            Unit::Percent => "percent, capped at 100",
            Unit::Multiplier => "percent of the base, uncapped",
            Unit::Frames => "frames",
            Unit::Count => "count",
            Unit::Range => "range units, stored at four times the shown distance",
            Unit::Money => "cost units, worth 1.5 money each",
        }
    }

    pub(super) fn percent(self) -> bool {
        self == Unit::Percent
    }
}

#[derive(Clone, Copy)]
pub(super) struct Value {
    label: &'static str,
    unit: Unit,
    fixed: bool,
    note: Option<&'static str>,
}

impl Value {
    const fn new(label: &'static str, unit: Unit) -> Value {
        Value { label, unit, fixed: false, note: None }
    }

    const fn fixed(label: &'static str, unit: Unit) -> Value {
        Value { label, unit, fixed: true, note: None }
    }

    const fn noted(label: &'static str, unit: Unit, note: &'static str) -> Value {
        Value { label, unit, fixed: false, note: Some(note) }
    }

    pub(super) fn unit(self) -> Unit {
        self.unit
    }

    pub(super) fn hint(self) -> String {
        let mut hint = format!("{} ({})", self.label, self.unit.name());

        if self.fixed {
            hint.push_str("\nOnly the Minimum is read; the Maximum is ignored");
        }

        if let Some(note) = self.note {
            hint.push('\n');
            hint.push_str(note);
        }

        hint
    }
}

const CHANCE: Value = Value::new("Chance", Unit::Percent);
const DURATION: Value = Value::new("Duration", Unit::Frames);
const LEVEL: Value = Value::new("Level", Unit::Count);
const RESIST: Value = Value::new("Resistance", Unit::Percent);

const SURGE: &[Value] = &const {
    [
        CHANCE,
        LEVEL,
        Value::fixed("Spawn Anchor", Unit::Range),
        Value::fixed("Spawn Span", Unit::Range),
    ]
};

const WAVE: &[Value] = &const { [CHANCE, LEVEL] };

const CHANCE_ONLY: &[Value] = &const { [CHANCE] };

const RESIST_ONLY: &[Value] = &const { [RESIST] };

const AILMENT: &[Value] = &const { [CHANCE, DURATION] };

pub(super) fn values(ability: i32) -> Option<&'static [Value]> {
    let listed: &'static [Value] = match ability {
        1 => &const {
            [
                CHANCE,
                DURATION,
                Value::noted("Reduced From", Unit::Percent, "Stored inverted: the game reads 100 minus this"),
            ]
        },
        2 | 3 | 60 => AILMENT,
        8 | 11 | 13 | 15 | 58 => CHANCE_ONLY,
        10 => &const {
            [
                Value::noted("Health Threshold", Unit::Percent, "Stored inverted: the game reads 100 minus this"),
                Value::new("Boost", Unit::Multiplier),
            ]
        },
        17 | 62 => WAVE,
        18..=22 | 30 | 52 | 54 => RESIST_ONLY,
        25 => &const { [Value::new("Cost Reduction", Unit::Money)] },
        26 => &const { [Value::new("Cooldown Reduction", Unit::Frames)] },
        27 => &const { [Value::new("Speed Increase", Unit::Count)] },
        28 => &const { [Value::new("Knockback Increase", Unit::Count)] },
        31 | 32 => &const { [Value::new("Increase", Unit::Percent)] },
        50 => &const { [CHANCE, Value::new("Boost", Unit::Multiplier)] },
        51 => AILMENT,
        56 | 65 => SURGE,
        64 => &const { [Value::new("Dodge Chance", Unit::Percent), Value::new("Dodge Duration", Unit::Frames)] },
        67 => &const {
            [CHANCE, Value::fixed("Spawn Anchor", Unit::Range), Value::fixed("Spawn Span", Unit::Range)]
        },
        4..=7 | 9 | 12 | 14 | 16 | 23 | 24 | 29 | 33..=49 | 53 | 55 | 57 | 59 | 61 | 63 | 66 | 68 | 69 => &[],
        _ => return None,
    };

    Some(listed)
}

pub(super) fn value(ability: i32, pair: usize) -> Option<Value> {
    values(ability)?.get(pair).copied()
}

#[cfg(test)]
mod tests {
    use nyanko::combat::get_talent;

    use super::values;

    #[test]
    fn every_published_talent_has_a_value_table() {
        for id in 1..=u8::MAX {
            let Some(ability) = get_talent(id) else {
                continue;
            };

            assert!(
                values(i32::from(id)).is_some(),
                "nyanko publishes talent {id} ({:?}), which no value-table arm names",
                ability.identity,
            );
        }
    }

    #[test]
    fn no_table_declares_more_than_the_four_stored_pairs() {
        for id in 1..=u8::MAX {
            if get_talent(id).is_none() {
                continue;
            }

            let listed = values(i32::from(id)).unwrap_or_default();

            assert!(
                listed.len() <= super::super::VALUES,
                "talent {id} declares {} value pairs, but a row only stores {}",
                listed.len(),
                super::super::VALUES,
            );
        }
    }
}
