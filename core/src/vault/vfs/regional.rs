use std::ffi::OsStr;
use std::ops::RangeInclusive;
use std::path::Path;

const LANG_TERMINATOR: &str = "--";

const UDI_BASE_ONLY_FORM: &str = "s";
const UDI_BASE_ONLY_IDS: RangeInclusive<u32> = 0..=9;

pub(super) fn interleaved<'a>(
    filenames: &'a [&'a str],
    priority: &'a [String],
) -> impl Iterator<Item = String> + 'a {
    priority
        .iter()
        .take_while(|code| code.as_str() != LANG_TERMINATOR)
        .flat_map(move |code| filenames.iter().filter_map(move |filename| variant(filename, code)))
}

fn variant(filename: &str, code: &str) -> Option<String> {
    if code.is_empty() {
        return Some(filename.to_string());
    }

    if form_excluded(filename) {
        return None;
    }

    suffixed(filename, code)
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

    if form != UDI_BASE_ONLY_FORM {
        return false;
    }

    digits.parse::<u32>().is_ok_and(|id| UDI_BASE_ONLY_IDS.contains(&id))
}
