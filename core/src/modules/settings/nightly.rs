use std::sync::atomic::{AtomicBool, Ordering};

use tracing::info;

pub static NIGHTLY_FEATURES_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn register_nightly_usage() {
    info!("Nightly development features activated.");
    NIGHTLY_FEATURES_ACTIVE.store(true, Ordering::Relaxed);
}