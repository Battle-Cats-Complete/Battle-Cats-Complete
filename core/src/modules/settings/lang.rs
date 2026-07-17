use tracing::debug;

pub const LANGUAGE_LIST: &[(&str, &str)] = &[
    ("", "Base"),
    ("en", "English"),
    ("ja", "Japanese"),
    ("tw", "Taiwanese"),
    ("ko", "Korean"),
    ("--", "None"),
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];

pub fn get_label_for_code(target_code: &str) -> String {
    for (language_code, language_label) in LANGUAGE_LIST {
        if *language_code == target_code {
            return language_label.to_string();
        }
    }
    format!("Unknown ({})", target_code)
}

pub fn default_priority() -> Vec<String> {
    vec![
        "".to_string(), "en".to_string(), "ja".to_string(), "tw".to_string(),
        "ko".to_string(), "--".to_string(), "es".to_string(), "de".to_string(),
        "fr".to_string(), "it".to_string(), "th".to_string()
    ]
}

pub fn ensure_complete_list(priority_list: &mut Vec<String>) {
    if priority_list.is_empty() {
        debug!("Priority list empty, resetting to default sequence.");
        *priority_list = default_priority();
        return;
    }

    let none_label = "--".to_string();
    if !priority_list.contains(&none_label) {
        priority_list.push(none_label);
    }

    for &(language_code, _) in LANGUAGE_LIST {
        let code_string = language_code.to_string();
        if !priority_list.contains(&code_string) {
            priority_list.push(code_string);
        }
    }

    priority_list.retain(|code_string| {
        LANGUAGE_LIST.iter().any(|(list_code, _)| *list_code == code_string)
    });
}