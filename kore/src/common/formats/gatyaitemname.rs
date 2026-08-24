use std::collections::HashMap;
use std::fs;

use nyanko::combat::Separator;

use crate::Vfs;

use super::GatyaItemName;

pub(crate) fn load(vfs: &Vfs, filename: &str) -> HashMap<usize, GatyaItemName> {
    let mut item_name_map = HashMap::new();
    let file_paths = vfs.list(filename);

    let Some(first_path) = file_paths.first() else {
        return item_name_map;
    };

    let Ok(file_content) = fs::read_to_string(first_path) else {
        return item_name_map;
    };

    let csv_separator = Separator::detect(&file_content).unwrap_or(Separator::Comma).char();

    for (current_row_index, line_string) in file_content.lines().enumerate() {
        let clean_line = line_string.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() {
            continue;
        }

        let line_parts: Vec<&str> = clean_line.split(csv_separator).collect();
        if line_parts.is_empty() {
            continue;
        }

        let item_name_string = line_parts[0].trim().to_string();

        let description_lines_array: Vec<String> = line_parts.iter()
            .skip(1)
            .map(|description_part| description_part.trim().to_string())
            .filter(|description_part| !description_part.is_empty() && description_part != "＠")
            .collect();

        let item_name_data = GatyaItemName {
            name: item_name_string,
            description: description_lines_array,
        };

        item_name_map.insert(current_row_index, item_name_data);
    }

    item_name_map
}