use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use tracing::{error, warn};

pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR_WIDTH: usize = 16;
const IHDR_HEIGHT: usize = 20;
const IHDR_END: usize = 24;

const DRAFT_EXTENSION: &str = "bcc";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stamp {
    pub mtime: u128,
    pub len: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("{0} changed on disk since it was opened")]
    Conflict(PathBuf),
    #[error("failed to stage {path}: {source}")]
    Staging { path: PathBuf, source: std::io::Error },
    #[error("staged copy of {0} did not match the edited text")]
    Corrupt(PathBuf),
    #[error("failed to replace {path}: {source}")]
    Replace { path: PathBuf, source: std::io::Error },
}

pub enum Preview {
    Image { bytes: Vec<u8>, width: u32, height: u32, stamp: Stamp },
    Text { body: String, stamp: Stamp },
    Oversized,
    Binary,
    Unavailable,
}

pub fn stamp(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok()?;

    let mtime = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());

    Some(Stamp { mtime, len: meta.len() })
}

pub fn load(path: &Path) -> Preview {
    let Ok(meta) = fs::metadata(path) else {
        return Preview::Unavailable;
    };

    if !meta.is_file() {
        return Preview::Unavailable;
    }

    if meta.len() > MAX_IMAGE_BYTES {
        return Preview::Oversized;
    }

    let Ok(bytes) = fs::read(path).inspect_err(|err| warn!(path = %path.display(), "preview read failed: {}", err)) else {
        return Preview::Unavailable;
    };

    let Some(current) = stamp(path) else {
        return Preview::Unavailable;
    };

    if is_png(&bytes) {
        return dimensions(&bytes)
            .map_or(Preview::Unavailable, |(width, height)| Preview::Image { bytes, width, height, stamp: current });
    }

    if meta.len() > MAX_TEXT_BYTES {
        return Preview::Oversized;
    }

    String::from_utf8(bytes).map_or(Preview::Binary, |body| Preview::Text { body, stamp: current })
}

pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&PNG_SIGNATURE)
}

pub fn save(path: &Path, body: &[u8], expected: Stamp) -> Result<Stamp, SaveError> {
    if stamp(path) != Some(expected) {
        return Err(SaveError::Conflict(path.to_path_buf()));
    }

    let draft = draft_path(path);

    if let Err(source) = stage(&draft, body) {
        discard(&draft);

        return Err(SaveError::Staging { path: draft, source });
    }

    if fs::read(&draft).is_ok_and(|written| written == body) {
        return fs::rename(&draft, path)
            .map_err(|source| SaveError::Replace { path: path.to_path_buf(), source })
            .and_then(|()| stamp(path).ok_or_else(|| SaveError::Conflict(path.to_path_buf())));
    }

    discard(&draft);

    Err(SaveError::Corrupt(path.to_path_buf()))
}

fn stage(draft: &Path, body: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(draft)?;

    file.write_all(body)?;
    file.sync_all()
}

fn discard(draft: &Path) {
    if draft.exists()
        && let Err(err) = fs::remove_file(draft)
    {
        error!(path = %draft.display(), "failed to clean up a staged draft: {}", err);
    }
}

fn draft_path(path: &Path) -> PathBuf {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("draft");

    path.with_file_name(format!(".{}.{}", name, DRAFT_EXTENSION))
}

fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let width = u32::from_be_bytes(bytes.get(IHDR_WIDTH..IHDR_HEIGHT)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(IHDR_HEIGHT..IHDR_END)?.try_into().ok()?);

    (width > 0 && height > 0).then_some((width, height))
}
