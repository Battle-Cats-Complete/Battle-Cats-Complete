pub fn sanitize(value: &str) -> String {
    value.chars().filter(char::is_ascii_alphanumeric).collect()
}
