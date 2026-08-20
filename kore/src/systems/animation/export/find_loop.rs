use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nyanko::graphics::rig::{Animation, Rig};
use tracing::{info, warn};

use super::LoopStatus;

const TIMEOUT_SECONDS: u64 = 180;

pub fn search(
    unit: &Rig,
    animation: &Animation,
    tolerance: f32,
    minimum_loop_length: i32,
    maximum_loop_length: Option<i32>,
    emit: impl Fn(LoopStatus),
    abort_signal: &AtomicBool,
) {
    info!("Starting loop detection routine");

    let start_time = Instant::now();
    let cycle_result = unit.calculate_cycle(
        animation,
        tolerance,
        Some(minimum_loop_length),
        maximum_loop_length,
        |current_frame| {
            if abort_signal.load(Ordering::Relaxed) {
                info!("Loop search explicitly aborted by user.");
                return false;
            }

            if start_time.elapsed().as_secs() > TIMEOUT_SECONDS {
                warn!("Loop search timed out after {} seconds.", TIMEOUT_SECONDS);
                emit(LoopStatus::Error("Timed out (3 mins)".to_string()));
                return false;
            }

            if current_frame % 5 == 0 {
                emit(LoopStatus::Searching(current_frame));
            }

            if current_frame % 100 == 0 {
                thread::sleep(Duration::from_millis(1));
            }

            true
        }
    );

    match cycle_result {
        Some(cycle) => {
            info!("Loop boundary found: {} to {}", cycle.start, cycle.end);
            emit(LoopStatus::Found(cycle.start, cycle.end));
        }
        None => {
            if !abort_signal.load(Ordering::Relaxed) && start_time.elapsed().as_secs() <= TIMEOUT_SECONDS {
                warn!("No loops found within limits.");
                emit(LoopStatus::Error("No loop found within limits".to_string()));
            }
        }
    }
}
