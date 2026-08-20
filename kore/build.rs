use std::fs;
use std::path::{Path, PathBuf};

const INPUTS: [&str; 3] = ["src", "Cargo.toml", "../Cargo.lock"];

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn main() {
    let mut fingerprint = FNV_OFFSET;

    for input in INPUTS {
        println!("cargo::rerun-if-changed={}", input);
        absorb_path(Path::new(input), &mut fingerprint);
    }

    println!("cargo::rustc-env=CORE_FINGERPRINT={:016x}", fingerprint);
}

fn absorb_path(path: &Path, hash: &mut u64) {
    absorb(path.to_string_lossy().as_bytes(), hash);

    if !path.is_dir() {
        if let Ok(bytes) = fs::read(path) {
            absorb(&bytes, hash);
        }
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();

    for child in children {
        absorb_path(&child, hash);
    }
}

fn absorb(bytes: &[u8], hash: &mut u64) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
