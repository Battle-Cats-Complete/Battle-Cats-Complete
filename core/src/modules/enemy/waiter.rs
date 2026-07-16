use std::fs;
use std::path::Path;

use nyanko::enemy::unit::{EnemyName, EnemyPictureBook, Battle};

use crate::common::resolver;

pub fn enemyname(lang_dir: &Path, priority: &[String]) -> Vec<String> {
    let mut final_names: Vec<String> = Vec::new();
    let base_dir = lang_dir.join("Enemyname");

    for file_path in resolver::get(&base_dir, ["Enemyname.tsv"], priority) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed_names) = EnemyName::parse_all(bytes) else {
            continue;
        };

        for (index, enemy) in parsed_names.into_iter().enumerate() {
            if index >= final_names.len() {
                final_names.push(enemy.name.unwrap_or_default());
                continue;
            }

            if final_names[index].is_empty() {
                let Some(valid_name) = enemy.name else {
                    continue;
                };

                final_names[index] = valid_name;
            }
        }
    }

    final_names
}

pub fn enemypicturebook(lang_dir: &Path, priority: &[String]) -> Vec<Vec<String>> {
    let mut final_descriptions: Vec<Vec<String>> = Vec::new();
    let base_dir = lang_dir.join("EnemyPictureBook");

    for file_path in resolver::get(&base_dir, ["EnemyPictureBook.csv"], priority) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed_books) = EnemyPictureBook::parse_all(bytes) else {
            continue;
        };

        for (index, enemy) in parsed_books.into_iter().enumerate() {
            if index >= final_descriptions.len() {
                final_descriptions.push(enemy.description.unwrap_or_default());
                continue;
            }

            if final_descriptions[index].is_empty() {
                let Some(valid_desc) = enemy.description else {
                    continue;
                };

                final_descriptions[index] = valid_desc;
            }
        }
    }

    final_descriptions
}

pub fn t_unit(dir: &Path, filename: &str, priority: &[String]) -> Option<Vec<Battle>> {
    let path = resolver::get(dir, [filename], priority).into_iter().next()?;
    let bytes = fs::read(path).ok()?;

    Battle::parse_all(bytes).ok()
}