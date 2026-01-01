pub const GLOBAL_CODES: &[&str] = &["de", "en", "es", "fr", "it", "th"];

pub fn has_language_marker(filename: &str, code: &str) -> bool {
    let marker_dot = format!("_{}.", code);
    let marker_und = format!("_{}_", code);
    filename.contains(&marker_dot) || filename.contains(&marker_und)
}