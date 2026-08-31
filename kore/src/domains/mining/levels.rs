use nyanko::cat::unit::UnitBuy;

use super::FileDelta;

pub struct Cap {
    pub base: i32,
    pub plus: i32,
}

impl Cap {
    pub fn label(&self) -> String {
        if self.plus > 0 {
            return format!("Lv{}+{}", self.base, self.plus);
        }

        format!("Lv{}", self.base)
    }
}

pub struct Raised {
    pub cat_id: u32,
    pub before: Cap,
    pub after: Cap,
}

pub fn read(delta: &FileDelta) -> Vec<Raised> {
    let mut found: Vec<Raised> = delta
        .rows
        .iter()
        .filter_map(|row| {
            let cat_id = row.key.trim().parse::<u32>().ok()?;
            let before = cap(&parse(row.before.as_deref()?)?);
            let after = cap(&parse(row.after.as_deref()?)?);

            if before.base == after.base && before.plus == after.plus {
                return None;
            }

            Some(Raised { cat_id, before, after })
        })
        .collect();

    found.sort_by_key(|raised| raised.cat_id);

    found
}

fn cap(row: &UnitBuy) -> Cap {
    Cap { base: row.level_cap_catseye, plus: row.level_cap_plus }
}

fn parse(line: &str) -> Option<UnitBuy> {
    UnitBuy::parse(line, None).ok()?.into_values().next()
}

#[cfg(test)]
mod tests {
    use super::super::{RowDelta, Status};
    use super::*;

    // unitbuy columns 50 and 51 hold the catseye cap and the plus cap.
    fn row(base: &str, plus: &str) -> String {
        let mut columns = vec!["0"; 63];
        columns[50] = base;
        columns[51] = plus;

        columns.join(",")
    }

    fn delta(rows: Vec<RowDelta>) -> FileDelta {
        FileDelta {
            file: "unitbuy.csv".to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 1,
            rows_after: 1,
            rows,
        }
    }

    #[test]
    fn a_raised_plus_cap_is_reported_with_both_readings() {
        let found = read(&delta(vec![RowDelta {
            key: "42".to_string(),
            before: Some(row("50", "20")),
            after: Some(row("50", "25")),
        }]));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].before.label(), "Lv50+20");
        assert_eq!(found[0].after.label(), "Lv50+25");
    }

    // A unitbuy row moves for many reasons; only the caps belong in this section.
    #[test]
    fn a_row_whose_caps_held_still_is_not_a_level_change() {
        let held = row("50", "20");
        let found = read(&delta(vec![RowDelta {
            key: "42".to_string(),
            before: Some(held.clone()),
            after: Some(held),
        }]));

        assert!(found.is_empty());
    }

    #[test]
    fn a_unit_without_plus_levels_reads_as_a_bare_level() {
        assert_eq!(Cap { base: 30, plus: 0 }.label(), "Lv30");
    }
}
