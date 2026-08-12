use std::fs;
use std::path::Path;

use tracing::warn;

pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR_WIDTH: usize = 16;
const IHDR_HEIGHT: usize = 20;
const IHDR_END: usize = 24;

pub enum Preview {
    Image { bytes: Vec<u8>, width: u32, height: u32 },
    Text(String),
    Oversized,
    Binary,
    Unavailable,
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

    if bytes.starts_with(&PNG_SIGNATURE) {
        return dimensions(&bytes).map_or(Preview::Unavailable, |(width, height)| Preview::Image { bytes, width, height });
    }

    if meta.len() > MAX_TEXT_BYTES {
        return Preview::Oversized;
    }

    String::from_utf8(bytes).map_or(Preview::Binary, Preview::Text)
}

fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let width = u32::from_be_bytes(bytes.get(IHDR_WIDTH..IHDR_HEIGHT)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(IHDR_HEIGHT..IHDR_END)?.try_into().ok()?);

    (width > 0 && height > 0).then_some((width, height))
}
