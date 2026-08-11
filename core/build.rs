use std::fs;

const LOCKFILE: &str = "../Cargo.lock";

fn main() {
    println!("cargo::rerun-if-changed={}", LOCKFILE);

    let rev = fs::read_to_string(LOCKFILE)
        .ok()
        .and_then(|lock| nyanko_rev(&lock))
        .unwrap_or_else(|| "unpinned".to_string());

    println!("cargo::rustc-env=NYANKO_REV={}", rev);
}

fn nyanko_rev(lock: &str) -> Option<String> {
    let mut inside = false;

    for line in lock.lines() {
        if let Some(name) = line.strip_prefix("name = ") {
            inside = name.trim_matches('"') == "nyanko";
            continue;
        }

        if inside && let Some(source) = line.strip_prefix("source = ") {
            return source.trim_matches('"').rsplit_once('#').map(|(_, rev)| rev.to_string());
        }
    }

    None
}
