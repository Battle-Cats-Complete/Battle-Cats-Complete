fn is_auditable_text(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".csv")
        || lower.ends_with(".tsv")
        || lower.ends_with(".mamodel")
        || lower.ends_with(".maanim")
        || lower.ends_with(".imgcut")
        || lower.ends_with(".json")
        || lower.ends_with(".list")
}

pub(crate) fn strip_carriage_returns(data: &[u8], filename: &str) -> Vec<u8> {
    if !is_auditable_text(filename) {
        return data.to_vec();
    }

    data.iter().copied().filter(|&byte| byte != b'\r').collect()
}