use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum JobEvent {
    Log(String),
    Progress { current: usize, total: usize },
    Finished(JobOutcome),
}

const TICK_INTERVAL_US: u64 = 16_000;

pub struct Ticker {
    start: Instant,
    next: AtomicU64,
}

impl Default for Ticker {
    fn default() -> Self {
        Self { start: Instant::now(), next: AtomicU64::new(0) }
    }
}

impl Ticker {
    pub fn ready(&self, done: usize, total: usize) -> bool {
        if done >= total {
            return true;
        }

        let elapsed = self.start.elapsed().as_micros() as u64;
        let next = self.next.load(Ordering::Relaxed);

        if elapsed < next {
            return false;
        }

        self.next
            .compare_exchange(next, elapsed + TICK_INTERVAL_US, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }
}

#[derive(Default)]
pub struct ProgressCounter {
    current: AtomicUsize,
    total: AtomicUsize,
}

impl ProgressCounter {
    pub fn reset(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.current.store(0, Ordering::Relaxed);
    }

    pub fn advance(&self) -> usize {
        self.current.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn current(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    pub fn fraction(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }

        self.current() as f32 / total as f32
    }
}

#[derive(Debug, Clone)]
pub enum JobOutcome {
    Completed,
    Aborted,
    Failed(String),
}
