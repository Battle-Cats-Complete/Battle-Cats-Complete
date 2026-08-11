const JUNK_NAMES: [&str; 3] = ["thumbs.db", "ehthumbs.db", "desktop.ini"];
const JUNK_SUFFIXES: [&str; 5] = [".swp", ".swo", ".tmp", ".bak", "~"];

pub fn ignored(name: &str) -> bool {
    if name.starts_with('.') || name.starts_with('#') {
        return true;
    }

    if JUNK_NAMES.iter().any(|junk| name.eq_ignore_ascii_case(junk)) {
        return true;
    }

    let lowered = name.to_ascii_lowercase();

    JUNK_SUFFIXES.iter().any(|suffix| lowered.ends_with(suffix))
}
