use super::{FileDelta, Status};

const MAP_STAGE_DATA: &str = "MapStageData";

const STAGE: &str = "stage";

const CSV: &str = ".csv";

const MAP_HEADER_LINES: usize = 2;

pub const MAP_OPTION: &str = "Map_option.csv";

const MAX_CROWNS_COLUMN: usize = 1;

pub struct Crowned {
    pub global_map: u32,
    pub before: u8,
    pub after: u8,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Located {
    pub prefix: String,
    pub map: u32,
    pub stage: Option<u32>,
}

#[derive(Default)]
pub struct Report {
    pub fresh_maps: Vec<Located>,
    pub fresh_stages: Vec<Located>,
    pub changed_stages: Vec<Located>,
    pub crowned: Vec<Crowned>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.fresh_maps.is_empty()
            && self.fresh_stages.is_empty()
            && self.changed_stages.is_empty()
            && self.crowned.is_empty()
    }
}

pub(crate) fn mineable(filename: &str) -> bool {
    split_map_data(filename).is_some() || split_stage(filename).is_some()
}

pub fn crowns(delta: &FileDelta) -> Vec<Crowned> {
    if delta.file != MAP_OPTION {
        return Vec::new();
    }

    let mut found: Vec<Crowned> = delta
        .rows
        .iter()
        .filter_map(|row| {
            let global_map = row.key.trim().parse::<u32>().ok()?;
            let before = column(row.before.as_deref()?)?;
            let after = column(row.after.as_deref()?)?;

            (after > before).then_some(Crowned { global_map, before, after })
        })
        .collect();

    found.sort_by_key(|crowned| crowned.global_map);

    found
}

fn column(line: &str) -> Option<u8> {
    line.split([',', '\t']).nth(MAX_CROWNS_COLUMN)?.trim().parse().ok()
}

pub fn read(delta: &FileDelta) -> Report {
    let mut report = Report::default();

    if delta.file == MAP_OPTION {
        report.crowned = crowns(delta);

        return report;
    }

    if let Some((prefix, map)) = split_map_data(&delta.file) {
        if delta.status == Status::Baseline {
            report.fresh_maps.push(Located { prefix, map, stage: None });

            return report;
        }

        for row in &delta.rows {
            let Some(stage) = stage_index(&row.key) else {
                continue;
            };

            if row.after.as_deref().is_none_or(terminates) {
                continue;
            }

            let found = Located { prefix: prefix.clone(), map, stage: Some(stage) };

            match (&row.before, &row.after) {
                (None, Some(_)) => report.fresh_stages.push(found),
                (Some(_), Some(_)) => report.changed_stages.push(found),
                _ => {}
            }
        }

        return report;
    }

    if let Some(found) = split_stage(&delta.file)
        && delta.status == Status::Changed
        && !delta.rows.is_empty()
    {
        report.changed_stages.push(found);
    }

    report
}

fn stage_index(key: &str) -> Option<u32> {
    let line = key.trim().parse::<usize>().ok()?;

    u32::try_from(line.checked_sub(MAP_HEADER_LINES)?).ok()
}

fn terminates(line: &str) -> bool {
    let mut written = line.split([',', '\t']).map(str::trim).filter(|part| !part.is_empty());

    written.next() == Some("-1") && written.next().is_none()
}

fn split_map_data(name: &str) -> Option<(String, u32)> {
    let body = name.strip_prefix(MAP_STAGE_DATA)?.strip_suffix(CSV)?;
    let (prefix, map) = body.rsplit_once('_')?;

    if prefix.is_empty() {
        return None;
    }

    Some((prefix.to_string(), map.parse().ok()?))
}

fn split_stage(name: &str) -> Option<Located> {
    let body = name.strip_prefix(STAGE)?.strip_suffix(CSV)?;
    let (head, stage) = body.rsplit_once('_')?;

    let cut = head.len().checked_sub(3)?;
    let (prefix, map) = head.split_at(cut);

    if prefix.is_empty() || !prefix.chars().all(char::is_alphabetic) {
        return None;
    }

    Some(Located { prefix: prefix.to_string(), map: map.parse().ok()?, stage: Some(stage.parse().ok()?) })
}

#[cfg(test)]
mod tests {
    use super::super::RowDelta;
    use super::*;

    fn delta(file: &str, status: Status, rows: Vec<RowDelta>) -> FileDelta {
        FileDelta {
            file: file.to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status,
            rows_before: 1,
            rows_after: 2,
            rows,
        }
    }

    // Map_option.csv keys by global map id and carries the crown count in column one.
    #[test]
    fn only_a_risen_crown_count_is_reported() {
        let held = FileDelta {
            file: MAP_OPTION.to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 3,
            rows_after: 3,
            rows: vec![
                RowDelta { key: "104".to_string(), before: Some("104,1,0".to_string()), after: Some("104,4,0".to_string()) },
                RowDelta { key: "105".to_string(), before: Some("105,4,0".to_string()), after: Some("105,1,0".to_string()) },
                RowDelta { key: "106".to_string(), before: Some("106,4,0".to_string()), after: Some("106,4,9".to_string()) },
            ],
        };

        let found = crowns(&held);

        assert_eq!(found.len(), 1);
        assert_eq!((found[0].global_map, found[0].before, found[0].after), (104, 1, 4));
    }

    #[test]
    fn a_map_table_names_its_category_and_subchapter() {
        assert_eq!(split_map_data("MapStageDataN_003.csv"), Some(("N".to_string(), 3)));
        assert_eq!(split_map_data("MapStageDataRE_012.csv"), Some(("RE".to_string(), 12)));
        assert_eq!(split_map_data("stageRN000_01.csv"), None);
    }

    // A stage file carries its category, subchapter and stage in the name alone.
    #[test]
    fn a_stage_table_names_the_stage_it_holds() {
        let found = split_stage("stageRN000_01.csv").expect("a stage");

        assert_eq!((found.prefix.as_str(), found.map, found.stage), ("RN", 0, Some(1)));
        assert!(split_stage("stageNormal0.csv").is_none(), "the story tables carry no stage number");
    }

    #[test]
    fn a_brand_new_map_table_is_a_new_subchapter() {
        let report = read(&delta("MapStageDataN_042.csv", Status::Baseline, Vec::new()));

        assert_eq!(report.fresh_maps.len(), 1);
        assert_eq!(report.fresh_maps[0].map, 42);
    }

    // Two header lines sit above the stage rows, so the raw line number reads high.
    #[test]
    fn added_and_moved_rows_split_into_new_and_changed_stages() {
        let report = read(&delta(
            "MapStageDataN_002.csv",
            Status::Changed,
            vec![
                RowDelta { key: "6".to_string(), before: None, after: Some("1,2,3".to_string()) },
                RowDelta { key: "3".to_string(), before: Some("a,b".to_string()), after: Some("b,c".to_string()) },
                RowDelta { key: "1".to_string(), before: None, after: Some("header".to_string()) },
            ],
        ));

        assert_eq!(report.fresh_stages.iter().map(|f| f.stage).collect::<Vec<_>>(), vec![Some(4)]);
        assert_eq!(report.changed_stages.iter().map(|f| f.stage).collect::<Vec<_>>(), vec![Some(1)]);
    }

    // The table closes with a lone -1, which is no stage at all.
    #[test]
    fn the_terminator_row_is_never_a_stage() {
        assert!(terminates("-1"));
        assert!(terminates("-1,,,"));
        assert!(!terminates("-1,2,3"));
        assert!(!terminates("100,200"));

        let report = read(&delta(
            "MapStageDataN_002.csv",
            Status::Changed,
            vec![RowDelta { key: "13".to_string(), before: None, after: Some("-1".to_string()) }],
        ));

        assert!(report.fresh_stages.is_empty());
    }

    #[test]
    fn a_moved_stage_table_is_a_changed_stage() {
        let report = read(&delta(
            "stageRN000_01.csv",
            Status::Changed,
            vec![RowDelta { key: "0".to_string(), before: Some("a".to_string()), after: Some("b".to_string()) }],
        ));

        assert_eq!(report.changed_stages.len(), 1);
        assert_eq!(report.changed_stages[0].stage, Some(1));
    }
}
