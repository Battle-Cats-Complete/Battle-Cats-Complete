use std::sync::atomic::{AtomicBool, Ordering};

pub static NIGHTLY_FEATURES_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn register_nightly_usage() {
    NIGHTLY_FEATURES_ACTIVE.store(true, Ordering::Relaxed);
}