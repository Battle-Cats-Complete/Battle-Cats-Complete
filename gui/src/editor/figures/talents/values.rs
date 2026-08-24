#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    Percent,
    Multiplier,
    Frames,
    Count,
    Range,
    Money,
    Inverted,
}

#[derive(Clone, Copy)]
pub(super) struct Value {
    label: &'static str,
    unit: Unit,
    fixed: bool,
}

impl Value {
    const fn new(label: &'static str, unit: Unit) -> Value {
        Value { label, unit, fixed: false }
    }

    const fn fixed(mut self) -> Value {
        self.fixed = true;

        self
    }

    pub(super) fn label(self) -> &'static str {
        self.label
    }

    pub(super) fn unit(self) -> Unit {
        self.unit
    }

    pub(super) fn is_fixed(self) -> bool {
        self.fixed
    }
}

const CHANCE: Value = Value::new("Chance", Unit::Percent);
const DURATION: Value = Value::new("Duration", Unit::Frames);
const LEVEL: Value = Value::new("Level", Unit::Count).fixed();
const RESIST: Value = Value::new("Resistance", Unit::Percent);
const BOOST: Value = Value::new("Boost", Unit::Multiplier);
const ANCHOR: Value = Value::new("Spawn Anchor", Unit::Range).fixed();
const SPAN: Value = Value::new("Spawn Span", Unit::Range).fixed();

const SURGE: &[Value] = &const { [CHANCE, LEVEL, ANCHOR, SPAN] };

const WAVE: &[Value] = &const { [CHANCE, LEVEL] };

const CHANCE_ONLY: &[Value] = &const { [CHANCE] };

const RESIST_ONLY: &[Value] = &const { [RESIST] };

const AILMENT: &[Value] = &const { [CHANCE, DURATION] };

pub(super) fn values(ability: i32) -> Option<&'static [Value]> {
    let listed: &'static [Value] = match ability {
        1 => &const { [CHANCE, DURATION, Value::new("Reduced To", Unit::Inverted).fixed()] },
        2 | 3 | 60 => AILMENT,
        8 | 11 | 13 | 15 | 58 => CHANCE_ONLY,
        10 => &const { [Value::new("Health Threshold", Unit::Inverted), BOOST] },
        17 | 62 => WAVE,
        18..=22 | 30 | 52 | 54 => RESIST_ONLY,
        25 => &const { [Value::new("Reduction", Unit::Money)] },
        26 => &const { [Value::new("Reduction", Unit::Frames)] },
        27 | 28 => &const { [Value::new("Increase", Unit::Count)] },
        31 | 32 => &const { [Value::new("Increase", Unit::Percent)] },
        50 => &const { [CHANCE, BOOST] },
        51 => AILMENT,
        56 | 65 => SURGE,
        61 => &const { [Value::new("Reduction", Unit::Percent)] },
        64 => &const {
            [Value::new("Dodge Chance", Unit::Percent), Value::new("Dodge Duration", Unit::Frames)]
        },
        67 => &const { [CHANCE, ANCHOR, SPAN] },
        4..=7 | 9 | 12 | 14 | 16 | 23 | 24 | 29 | 33..=49 | 53 | 55 | 57 | 59 | 63 | 66 | 68 | 69 => &[],
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

    #[test]
    fn every_pair_nyanko_publishes_is_named_here() {
        for id in 1..=u8::MAX {
            let Some(ability) = get_talent(id) else {
                continue;
            };

            let listed = values(i32::from(id)).unwrap_or_default();

            assert!(
                listed.len() >= ability.talent_values.len(),
                "nyanko gives talent {id} ({:?}) {} value pairs, but only {} are named here",
                ability.identity,
                ability.talent_values.len(),
                listed.len(),
            );
        }
    }
}
