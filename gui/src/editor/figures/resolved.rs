use super::combat::{cats, enemies};
use super::{schema::Subject, unitbuy, unitlevel};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Rule {
    Plain,
    Opaque,
}

pub(super) fn rule(subject: Subject, field: Option<&str>) -> Rule {
    match subject {
        Subject::Cat => lookup(field, cats::rule),
        Subject::Enemy => lookup(field, enemies::rule),
        Subject::Buy => lookup(field, unitbuy::rule),
        Subject::Curve => unitlevel::rule(),
    }
}

fn lookup(field: Option<&str>, find: fn(&str) -> Option<Rule>) -> Rule {
    field.and_then(find).unwrap_or(Rule::Opaque)
}
