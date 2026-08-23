use std::fmt;

use kore::domains::settings::EditorValues;

use super::combat::{cats, enemies};
use super::{schema::Subject, talents, unitbuy, unitlevel};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Rule {
    Plain,
    Opaque,
    Flag,
    Choice(&'static [Choice]),
    Gated(&'static [Choice], Gate),
    Percent,
    Floor(i32),
    Offset(i32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor::figures) struct Gate {
    pub(in crate::editor::figures) field: &'static str,
    pub(in crate::editor::figures) blocked: i32,
    pub(in crate::editor::figures) reason: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor::figures) struct Choice {
    raw: i32,
    label: &'static str,
}

impl Choice {
    pub(in crate::editor::figures) const fn new(raw: i32, label: &'static str) -> Choice {
        Choice { raw, label }
    }

    pub(super) fn raw(self) -> i32 {
        self.raw
    }

    pub(super) fn label(self) -> &'static str {
        self.label
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Toggle {
    No,
    Yes,
}

impl Toggle {
    pub(super) fn flip(self) -> Toggle {
        match self {
            Toggle::No => Toggle::Yes,
            Toggle::Yes => Toggle::No,
        }
    }

    pub(super) fn raw(self) -> i32 {
        match self {
            Toggle::No => 0,
            Toggle::Yes => 1,
        }
    }

    fn of(raw: i32) -> Option<Toggle> {
        match raw {
            0 => Some(Toggle::No),
            1 => Some(Toggle::Yes),
            _ => None,
        }
    }
}

impl fmt::Display for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Toggle::No => "No",
            Toggle::Yes => "Yes",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Face {
    Number,
    Danger,
    Toggle(Toggle),
    Choice(&'static [Choice], Choice),
    Disabled(&'static str),
}

pub(super) const PERCENT_FLOOR: i32 = 0;
pub(super) const PERCENT_CEILING: i32 = 100;

impl Rule {
    pub(super) fn gate(self) -> Option<Gate> {
        match self {
            Rule::Gated(_, gate) => Some(gate),
            _ => None,
        }
    }

    pub(super) fn to_display(self, raw: i32, values: EditorValues) -> i32 {
        match self {
            Rule::Offset(by) if values == EditorValues::Resolved => raw.saturating_add(by),
            _ => raw,
        }
    }

    pub(super) fn to_raw(self, display: i32, values: EditorValues) -> i32 {
        match self {
            Rule::Offset(by) if values == EditorValues::Resolved => display.saturating_sub(by),
            _ => display,
        }
    }

    pub(super) fn signed(self, values: EditorValues) -> bool {
        if values == EditorValues::Raw {
            return true;
        }

        match self {
            Rule::Percent => false,
            Rule::Floor(least) => least < 0,
            _ => true,
        }
    }

    pub(super) fn clamp(self, raw: i32, values: EditorValues) -> i32 {
        if values == EditorValues::Raw {
            return raw;
        }

        match self {
            Rule::Percent => raw.clamp(PERCENT_FLOOR, PERCENT_CEILING),
            Rule::Floor(least) => raw.max(least),
            _ => raw,
        }
    }

    pub(super) fn face(self, raw: i32, values: EditorValues) -> Face {
        if values == EditorValues::Raw {
            return Face::Number;
        }

        match self {
            Rule::Plain | Rule::Percent | Rule::Floor(_) | Rule::Offset(_) => Face::Number,
            Rule::Opaque => Face::Danger,
            Rule::Flag => Toggle::of(raw).map_or(Face::Danger, Face::Toggle),
            Rule::Choice(options) | Rule::Gated(options, _) => options
                .iter()
                .find(|choice| choice.raw == raw)
                .map_or(Face::Danger, |choice| Face::Choice(options, *choice)),
        }
    }
}

pub(super) fn note(subject: Subject, field: Option<&str>) -> Option<&'static str> {
    let field = field?;

    match subject {
        Subject::Cat => cats::note(field),
        Subject::Enemy => enemies::note(field),
        Subject::Buy | Subject::Curve | Subject::Talents => None,
    }
}

pub(super) fn rule(subject: Subject, index: usize, field: Option<&str>) -> Rule {
    match subject {
        Subject::Cat => lookup(field, cats::rule),
        Subject::Enemy => lookup(field, enemies::rule),
        Subject::Buy => lookup(field, unitbuy::rule),
        Subject::Curve => unitlevel::rule(),
        Subject::Talents => talents::rule(index),
    }
}

fn lookup(field: Option<&str>, find: fn(&str) -> Option<Rule>) -> Rule {
    field.and_then(find).unwrap_or(Rule::Opaque)
}

#[cfg(test)]
mod tests {
    use super::{rule, Rule};
    use crate::editor::figures::combat::{cats, enemies};
    use crate::editor::figures::schema::{self, SUBJECTS};

    #[test]
    fn cats_and_enemies_agree_on_the_columns_they_share() {
        for field in ["area_attack", "attack_count_state"] {
            assert_eq!(
                cats::rule(field),
                enemies::rule(field),
                "{field} is declared in both combat tables and the two copies have drifted",
            );
        }
    }

    #[test]
    fn no_scaled_column_is_a_flag() {
        for subject in SUBJECTS {
            for (index, entry) in schema::of(subject).order().iter().enumerate() {
                if !entry.scaled() {
                    continue;
                }

                assert_ne!(
                    rule(subject, index, Some(entry.field)),
                    Rule::Flag,
                    "{subject:?}: {} carries a nyanko Scale, so it is a magnitude and never a flag",
                    entry.field,
                );
            }
        }
    }
}
