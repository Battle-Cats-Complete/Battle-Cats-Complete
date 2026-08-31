use nyanko::combat::Entity;
use nyanko::enemy::unit::t_unit;

use crate::domains::enemy::files;

use super::FileDelta;

const HEADER_LINES: usize = 2;

pub struct Changed {
    pub enemy_id: u32,
    pub previous: Entity,
    pub current: Entity,
}

pub fn read(delta: &FileDelta) -> Vec<Changed> {
    if delta.file != files::STATS {
        return Vec::new();
    }

    let mut found: Vec<Changed> = delta
        .rows
        .iter()
        .filter_map(|row| {
            let enemy_id = enemy_id(&row.key)?;
            let previous = parse(row.before.as_deref()?)?;
            let current = parse(row.after.as_deref()?)?;

            (previous != current).then_some(Changed { enemy_id, previous, current })
        })
        .collect();

    found.sort_by_key(|changed| changed.enemy_id);

    found
}

pub fn fresh(delta: &FileDelta) -> Vec<u32> {
    if delta.file != files::STATS {
        return Vec::new();
    }

    let mut found: Vec<u32> = delta
        .rows
        .iter()
        .filter(|row| row.before.is_none() && row.after.is_some())
        .filter_map(|row| enemy_id(&row.key))
        .collect();

    found.sort_unstable();

    found
}

fn enemy_id(key: &str) -> Option<u32> {
    let line = key.trim().parse::<usize>().ok()?;

    u32::try_from(line.checked_sub(HEADER_LINES)?).ok()
}

fn parse(line: &str) -> Option<Entity> {
    let padded = format!("{}{}", "\n".repeat(HEADER_LINES), line);

    t_unit::parse_row(padded.as_bytes(), 0, None)
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
            rows_after: 3,
            rows,
        }
    }

    fn stats(hitpoints: &str) -> String {
        let mut columns = vec!["0"; 120];
        columns[0] = hitpoints;

        columns.join(",")
    }

    // t_unit.csv opens with two header lines, so the enemy id sits that far below the
    // raw line number the differ keys rows by.
    #[test]
    fn a_row_number_reads_past_the_tables_header() {
        let held = delta(
            "t_unit.csv",
            vec![
                RowDelta { key: "7".to_string(), before: None, after: Some(stats("100")) },
                RowDelta { key: "3".to_string(), before: Some(stats("100")), after: Some(stats("200")) },
                RowDelta { key: "0".to_string(), before: None, after: Some(stats("50")) },
            ],
        );

        assert_eq!(fresh(&held), vec![5], "line seven is enemy five, and a header line is no enemy");

        let changed = read(&held);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].enemy_id, 1);
    }

    // The header offset is nyanko's, so a single row must parse the way the whole table does.
    #[test]
    fn a_lone_row_parses_as_nyanko_reads_it() {
        let row = stats("400");
        let table = format!("header\nheader\n{}", row);

        let whole = t_unit::parse_row(table.as_bytes(), 0, None).expect("nyanko reads the row");

        assert_eq!(parse(&row).as_ref(), Some(&whole));
    }

    // Trailing padding moves the row text without moving the enemy.
    #[test]
    fn padding_that_parses_the_same_is_not_a_change() {
        let padded = format!("{},0,0", stats("100"));
        let held = delta(
            "t_unit.csv",
            vec![RowDelta { key: "3".to_string(), before: Some(stats("100")), after: Some(padded) }],
        );

        assert!(read(&held).is_empty());
    }

    #[test]
    fn another_table_yields_nothing() {
        let held = delta(
            "unitbuy.csv",
            vec![RowDelta { key: "0".to_string(), before: Some(stats("1")), after: Some(stats("2")) }],
        );

        assert!(read(&held).is_empty() && fresh(&held).is_empty());
    }
}
