use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use nyanko::graphics::rig::{Animation, Unit};
use tracing::{error, info, warn};

use super::LoopStatus;

const TIMEOUT_SECONDS: u64 = 180;

pub fn start_search(
    unit: Arc<Unit>,
    animation: Arc<Animation>,
    tolerance: f32,
    minimum_loop_length: i32,
    maximum_loop_length: Option<i32>,
    status_sender: mpsc::Sender<LoopStatus>,
    abort_signal: Arc<AtomicBool>
) {
    info!("Starting loop detection routine");

    thread::spawn(move || {
        let start_time = Instant::now();
        let cycle_result = unit.calculate_cycle(
            &animation,
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
                    if let Err(e) = status_sender.send(LoopStatus::Error("Timed out (3 mins)".to_string())) {
                        error!("Failed to send timeout error: {}", e);
                    }
                    return false;
                }

                if current_frame % 5 == 0 {
                    if let Err(e) = status_sender.send(LoopStatus::Searching(current_frame)) {
                        error!("Failed to dispatch Searching status: {}", e);
                    }
                }

                if current_frame % 100 == 0 {
                    thread::sleep(Duration::from_millis(1));
                }

                true
            }
        );

        match cycle_result {
            Some((start_frame, end_frame)) => {
                info!("Loop boundary found: {} to {}", start_frame, end_frame);
                if let Err(e) = status_sender.send(LoopStatus::Found(start_frame, end_frame)) {
                    error!("Failed to send loop found status: {}", e);
                }
            }
            None => {
                if !abort_signal.load(Ordering::Relaxed) && start_time.elapsed().as_secs() <= TIMEOUT_SECONDS {
                    warn!("No loops found within limits.");
                    if let Err(e) = status_sender.send(LoopStatus::Error("No loop found within limits".to_string())) {
                        error!("Failed to send loop not found error: {}", e);
                    }
                }
            }
        }
    });
}