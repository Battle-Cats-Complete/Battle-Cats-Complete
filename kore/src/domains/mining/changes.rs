use nyanko::cat::unit::unitid;
use nyanko::combat::Entity;

use crate::domains::cat::files;

use super::FileDelta;

pub struct Changed {
    pub cat_id: u32,
    pub form: usize,
    pub previous: Entity,
    pub current: Entity,
}

pub fn read(delta: &FileDelta) -> Vec<Changed> {
    let Some(cat_id) = files::stats_id(&delta.file) else {
        return Vec::new();
    };

    let mut found: Vec<Changed> = delta
        .rows
        .iter()
        .filter_map(|row| {
            let form = row.key.trim().parse::<usize>().ok()?;
            let previous = parse(row.before.as_deref()?)?;
            let current = parse(row.after.as_deref()?)?;

            if previous == current {
                return None;
            }

            Some(Changed { cat_id, form, previous, current })
        })
        .collect();

    found.sort_by_key(|changed| changed.form);

    found
}

fn parse(line: &str) -> Option<Entity> {
    unitid::parse(line.as_bytes(), None).ok()?.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::super::{RowDelta, Status};
    use super::*;

    fn delta(file: &str, rows: Vec<RowDelta>) -> FileDelta {
        FileDelta {
            file: file.to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 2,
            rows_after: 2,
            rows,
        }
    }

    fn stats(hitpoints: &str) -> String {
        let mut columns = vec!["0"; 120];
        columns[0] = hitpoints;

        columns.join(",")
    }

    // Each line of a unit stats file is one form, so a changed line is a changed form.
    #[test]
    fn a_changed_line_reads_as_that_units_form() {
        let found = read(&delta(
            "unit440.csv",
            vec![RowDelta { key: "2".to_string(), before: Some(stats("100")), after: Some(stats("200")) }],
        ));

        assert_eq!(found.len(), 1);
        assert_eq!((found[0].cat_id, found[0].form), (439, 2));
        assert_ne!(found[0].previous.hitpoints, found[0].current.hitpoints);
    }

    // An added line is a form the unit did not have, which the Forms section reports.
    #[test]
    fn an_added_line_is_not_a_change() {
        let found = read(&delta(
            "unit440.csv",
            vec![RowDelta { key: "3".to_string(), before: None, after: Some(stats("200")) }],
        ));

        assert!(found.is_empty());
    }

    // Granting a later form an ability makes the developers pad every earlier line with
    // the column's default, so the row text moves while the unit itself does not.
    #[test]
    fn trailing_padding_that_changes_nothing_is_not_a_change() {
        let padded = format!("{},0,0,0,0", stats("100"));

        let found = read(&delta(
            "unit440.csv",
            vec![RowDelta { key: "0".to_string(), before: Some(stats("100")), after: Some(padded) }],
        ));

        assert!(found.is_empty());
    }

    #[test]
    fn a_table_that_is_not_a_unit_yields_nothing() {
        let found = read(&delta(
            "unitbuy.csv",
            vec![RowDelta { key: "0".to_string(), before: Some(stats("1")), after: Some(stats("2")) }],
        ));

        assert!(found.is_empty());
    }
}
