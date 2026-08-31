use super::{FileDelta, Status};

pub struct Report {
    pub status: Status,
    pub region: String,
    pub rows_before: usize,
    pub rows_after: usize,
    pub fresh: Vec<u32>,
}

pub fn read(delta: &FileDelta) -> Report {
    let mut fresh: Vec<u32> = delta
        .rows
        .iter()
        .filter(|row| row.before.is_none() && row.after.is_some())
        .filter_map(|row| row.key.trim().parse::<u32>().ok())
        .collect();

    fresh.sort_unstable();

    Report {
        status: delta.status,
        region: delta.region.clone(),
        rows_before: delta.rows_before,
        rows_after: delta.rows_after,
        fresh,
    }
}

#[cfg(test)]
mod tests {
    use super::super::RowDelta;
    use super::*;

    fn delta(rows: Vec<RowDelta>) -> FileDelta {
        FileDelta {
            file: "unitbuy.csv".to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 2,
            rows_after: 4,
            rows,
        }
    }

    #[test]
    fn only_appended_rows_count_as_new_units() {
        let report = read(&delta(vec![
            RowDelta { key: "3".to_string(), before: None, after: Some("row".to_string()) },
            RowDelta { key: "1".to_string(), before: Some("old".to_string()), after: Some("new".to_string()) },
            RowDelta { key: "2".to_string(), before: None, after: Some("row".to_string()) },
            RowDelta { key: "0".to_string(), before: Some("gone".to_string()), after: None },
        ]));

        assert_eq!(report.fresh, vec![2, 3], "a retuned row is not a new unit, and a dropped one is not either");
    }
}
