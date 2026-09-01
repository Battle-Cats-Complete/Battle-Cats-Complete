use std::collections::BTreeMap;

use nyanko::cat::unit::UnitExplanation;
use nyanko::enemy::{EnemyName, EnemyPictureBook};

use crate::common::region;
use crate::domains::settings::lang;
use crate::Vfs;

use super::{Ore, Status};

const EXPLANATION: &str = "Unit_Explanation";
const ENEMY_NAMES: &str = "Enemyname";
const ENEMY_BOOK: &str = "EnemyPictureBook";
const CSV: &str = ".csv";
const TSV: &str = ".tsv";
const CODE_LENGTH: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Table {
    Names,
    Book,
}

pub struct Localized {
    pub id: u32,
    pub languages: Vec<String>,
}

pub(crate) fn mineable(filename: &str) -> bool {
    explanation(filename).is_some() || roster(filename).is_some()
}

pub fn cats(ore: &Ore, vfs: &Vfs) -> Vec<Localized> {
    let mut found: BTreeMap<u32, Vec<String>> = BTreeMap::new();

    for delta in &ore.files {
        let Some((id, code)) = explanation(&delta.file) else {
            continue;
        };

        let gained = match delta.status {
            Status::Baseline => vfs.load(&delta.file).is_some_and(|bytes| explained(&bytes)),
            Status::Changed => delta.rows.iter().any(|row| {
                row.after.as_deref().is_some_and(|line| explained(line.as_bytes()))
                    && !row.before.as_deref().is_some_and(|line| explained(line.as_bytes()))
            }),
        };

        if gained {
            record(&mut found, id, &code);
        }
    }

    gather(found)
}

pub fn enemies(ore: &Ore, vfs: &Vfs) -> Vec<Localized> {
    let mut found: BTreeMap<u32, Vec<String>> = BTreeMap::new();

    for delta in &ore.files {
        let Some((table, code)) = roster(&delta.file) else {
            continue;
        };

        let speaks = |line: &str| reads(table, line, &delta.file);

        match delta.status {
            Status::Baseline => {
                let Some(bytes) = vfs.load(&delta.file) else {
                    continue;
                };

                for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
                    if speaks(line) {
                        record(&mut found, index as u32, &code);
                    }
                }
            }
            Status::Changed => {
                for row in &delta.rows {
                    let Ok(id) = row.key.parse::<u32>() else {
                        continue;
                    };

                    if row.after.as_deref().is_some_and(&speaks)
                        && !row.before.as_deref().is_some_and(&speaks)
                    {
                        record(&mut found, id, &code);
                    }
                }
            }
        }
    }

    gather(found)
}

fn record(found: &mut BTreeMap<u32, Vec<String>>, id: u32, code: &str) {
    let spoken = found.entry(id).or_default();

    if !spoken.iter().any(|held| held == code) {
        spoken.push(code.to_string());
    }
}

fn gather(found: BTreeMap<u32, Vec<String>>) -> Vec<Localized> {
    let order = lang::default_priority();
    let rank = |code: &String| order.iter().position(|entry| entry == code).unwrap_or(usize::MAX);

    found
        .into_iter()
        .map(|(id, mut languages)| {
            languages.sort_by(|left, right| rank(left).cmp(&rank(right)).then_with(|| left.cmp(right)));

            Localized { id, languages }
        })
        .collect()
}

fn code(rest: &str) -> Option<String> {
    match rest {
        "" => Some(String::new()),
        other => other
            .strip_prefix('_')
            .filter(|code| code.len() == CODE_LENGTH && code.chars().all(|glyph| glyph.is_ascii_lowercase()))
            .map(str::to_string),
    }
}

fn explanation(filename: &str) -> Option<(u32, String)> {
    let body = filename.strip_prefix(EXPLANATION)?.strip_suffix(CSV)?;
    let cut = body.find(|glyph: char| !glyph.is_ascii_digit()).unwrap_or(body.len());
    let (digits, rest) = body.split_at(cut);

    Some((digits.parse::<u32>().ok()?.checked_sub(1)?, code(rest)?))
}

fn roster(filename: &str) -> Option<(Table, String)> {
    let named = filename.strip_prefix(ENEMY_NAMES).and_then(|body| body.strip_suffix(TSV));

    if let Some(rest) = named {
        return Some((Table::Names, code(rest)?));
    }

    let described = filename.strip_prefix(ENEMY_BOOK).and_then(|body| body.strip_suffix(CSV))?;

    Some((Table::Book, code(described)?))
}

fn reads(table: Table, line: &str, filename: &str) -> bool {
    match table {
        Table::Names => EnemyName::parse(line).is_ok_and(|rows| rows.first().is_some_and(|row| row.name.is_some())),
        Table::Book => EnemyPictureBook::parse(line, Some(region::text_separator(filename)))
            .is_ok_and(|rows| rows.first().is_some_and(|row| row.description.is_some())),
    }
}

fn explained(bytes: &[u8]) -> bool {
    UnitExplanation::parse(bytes, None).is_ok_and(|held| held.names.iter().any(Option::is_some))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_localized_table_is_recognised_only_in_its_own_family() {
        assert_eq!(explanation("Unit_Explanation301_ko.csv"), Some((300, "ko".to_string())));
        assert_eq!(explanation("Unit_Explanation301.csv"), Some((300, String::new())));
        assert!(explanation("Unit_Explanation0.csv").is_none(), "there is no unit before the first");
        assert!(explanation("SkillDescriptions.csv").is_none());

        assert!(matches!(roster("Enemyname_en.tsv"), Some((Table::Names, code)) if code == "en"));
        assert!(matches!(roster("EnemyPictureBook.csv"), Some((Table::Book, code)) if code.is_empty()));

        // The sibling tables share the picture book's prefix but are not the picture book.
        assert!(roster("EnemyPictureBook2_en.csv").is_none());
        assert!(roster("EnemyPictureBookQuestion.csv").is_none());
    }

    // nyanko owns what counts as a placeholder; the section only asks it the question.
    #[test]
    fn a_placeholder_row_is_not_a_localization() {
        assert!(reads(Table::Names, "Ragnarok", "Enemyname_en.tsv"));
        assert!(!reads(Table::Names, "ダミー", "Enemyname_en.tsv"));
        assert!(!reads(Table::Names, "   ", "Enemyname_en.tsv"));

        assert!(explained(b"Ragnarok|A fallen angel."));
        assert!(!explained(b"301|"), "a bare identifier is not a name");
    }
}
