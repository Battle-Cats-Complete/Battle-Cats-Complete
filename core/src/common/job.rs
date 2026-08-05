use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
pub enum JobEvent {
    Log(String),
    Progress { current: usize, total: usize },
    Finished(JobOutcome),
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

    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    pub fn fraction(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }

        self.current.load(Ordering::Relaxed) as f32 / total as f32
    }
}

#[derive(Debug, Clone)]
pub enum JobOutcome {
    Completed,
    Aborted,
    Failed(String),
}
