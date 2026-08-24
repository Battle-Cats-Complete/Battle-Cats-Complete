use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use nyanko::graphics::rig::{Animation, BoundingBox, Rig, Tolerance};
use nyanko::graphics::tools::boundary::scan_bounds;
use tracing::{info, warn};

use crate::common::job::ProgressCounter;

use super::BoundsOutcome;

const YIELD_INTERVAL: usize = 100;

pub fn search(
    unit: &Rig,
    clips: &[(&Animation, Option<i32>)],
    tolerance: f32,
    progress: &ProgressCounter,
    abort_signal: &AtomicBool,
) -> BoundsOutcome {
    let limits: Vec<i32> = clips.iter()
        .map(|(animation, to_frame)| {
            let last = animation.playback_frames().saturating_sub(1);
            to_frame.map_or(last, |to| last.min(to))
        })
        .collect();

    let total = limits.iter().map(|last| (last + 1).max(0) as usize).sum();
    progress.reset(total);

    info!("Starting bounds measurement over {} frames", total);

    let strictness = Tolerance::new(tolerance);
    let mut bounds: Option<BoundingBox> = None;

    for ((animation, _), last) in clips.iter().zip(&limits) {
        for frame in 0..=*last {
            if abort_signal.load(Ordering::Relaxed) {
                info!("Bounds measurement explicitly aborted by user.");
                return BoundsOutcome::Aborted;
            }

            if let Some(measured) = scan_bounds(unit, Some(animation), strictness, Some((frame, frame))) {
                bounds = Some(bounds.map_or(measured, |bounds| bounds.union(&measured)));
            }

            if progress.advance().is_multiple_of(YIELD_INTERVAL) {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    bounds.map_or_else(
        || {
            warn!("No visible geometry found while measuring bounds.");
            BoundsOutcome::Empty
        },
        BoundsOutcome::Found,
    )
}
