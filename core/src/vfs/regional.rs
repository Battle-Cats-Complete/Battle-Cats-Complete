use std::ffi::OsStr;
use std::path::Path;

const LANG_TERMINATOR: &str = "--";
const UDI_FORM_EXCLUSIONS: &[(u32, u32)] = &[];

pub(super) fn candidates(filename: &str, priority: &[String]) -> Vec<String> {
    interleaved(&[filename], priority)
}

pub(super) fn interleaved(filenames: &[&str], priority: &[String]) -> Vec<String> {
    let mut targets = Vec::new();

    for code in priority {
        if code == LANG_TERMINATOR {
            break;
        }

        for filename in filenames {
            if code.is_empty() {
                targets.push((*filename).to_string());
                continue;
            }

            if form_excluded(filename) {
                continue;
            }

            if let Some(name) = suffixed(filename, code) {
                targets.push(name);
            }
        }
    }

    targets
}

fn suffixed(filename: &str, code: &str) -> Option<String> {
    let path = Path::new(filename);
    let stem = path.file_stem().and_then(OsStr::to_str)?;
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();

    if extension.is_empty() {
        return Some(format!("{}_{}", stem, code));
    }

    Some(format!("{}_{}.{}", stem, code, extension))
}

fn form_excluded(filename: &str) -> bool {
    let Some(stem) = Path::new(filename).file_stem().and_then(OsStr::to_str) else {
        return false;
    };

    let Some(rest) = stem.strip_prefix("udi") else {
        return false;
    };

    let Some((digits, form)) = rest.split_once('_') else {
        return false;
    };

    if form != "f" && form != "c" {
        return false;
    }

    let Ok(id) = digits.parse::<u32>() else {
        return false;
    };

    UDI_FORM_EXCLUSIONS.iter().any(|&(first, last)| id >= first && id <= last)
}
