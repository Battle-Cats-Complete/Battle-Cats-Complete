use std::path::PathBuf;

use kore::systems::animation::authoring::{Imgcut, Maanim, Mamodel};

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run -p kore --example rig_corpus -- <game or mods root>");

        return;
    };

    let mut total = 0usize;
    let mut exact = 0usize;
    let mut drifted: Vec<PathBuf> = Vec::new();
    let mut unread: Vec<PathBuf> = Vec::new();
    let mut pending = vec![PathBuf::from(root)];

    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                pending.push(path);

                continue;
            }

            let kind = path.extension().and_then(|kind| kind.to_str()).unwrap_or_default();

            if !matches!(kind, "maanim" | "mamodel" | "imgcut") {
                continue;
            }

            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };

            total += 1;

            let written = match kind {
                "maanim" => Maanim::parse(&bytes).map(|doc| doc.write()).ok(),
                "imgcut" => Imgcut::parse(&bytes).map(|doc| doc.write()).ok(),
                _ => Mamodel::parse(&bytes).map(|doc| doc.write()).ok(),
            };

            match written {
                Some(body) if body == bytes => exact += 1,
                Some(_) => drifted.push(path),
                None => unread.push(path),
            }
        }
    }

    println!("total {} exact {} drifted {} unreadable {}", total, exact, drifted.len(), unread.len());

    for path in drifted.iter().take(6).chain(unread.iter().take(6)) {
        println!("  {}", path.display());
    }
}
