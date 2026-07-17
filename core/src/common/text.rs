use regex::Regex;

pub fn strip_markdown(text: &str) -> String {
    let mut text = text.to_string();

    if let Ok(re_link) = Regex::new(r"\[([^]]+)]\([^)]+\)") {
        text = re_link.replace_all(&text, "$1").to_string();
    }

    if let Ok(re_list) = Regex::new(r"(?m)^(\s*)[*\-]\s+") {
        text = re_list.replace_all(&text, "${1}• ").to_string();
    }

    text = text.replace("**", "");
    text = text.replace("__", "");
    text = text.replace("*", "");
    text = text.replace("_", "");
    text = text.replace("`", "");

    text
}