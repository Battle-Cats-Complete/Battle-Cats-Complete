use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{debug, trace};

pub(crate) fn debounce_loop(rx: Receiver<PathBuf>, final_sender: Sender<PathBuf>, ctx: egui::Context) {
    let mut pending_paths: HashSet<PathBuf> = HashSet::new();
    let mut deadline: Option<Instant> = None;
    let mut max_deadline: Option<Instant> = None;

    let buffer_duration = Duration::from_millis(500);
    let max_duration = Duration::from_secs(2);

    loop {
        let timeout = if let (Some(d), Some(md)) = (deadline, max_deadline) {
            let now = Instant::now();
            let effective_deadline = d.min(md);

            if now >= effective_deadline {
                if !pending_paths.is_empty() {
                    debug!("Debounce loop threshold triggered, pushing {} paths", pending_paths.len());
                    for path in pending_paths.drain() {
                        let _ = final_sender.send(path);
                    }
                    ctx.request_repaint();
                }
                deadline = None;
                max_deadline = None;
                Duration::from_millis(u64::MAX)
            } else {
                effective_deadline.saturating_duration_since(now)
            }
        } else {
            Duration::from_millis(u64::MAX)
        };

        match rx.recv_timeout(timeout) {
            Ok(path) => {
                trace!("Debouncer queued path: {:?}", path);
                pending_paths.insert(path);

                let now = Instant::now();
                deadline = Some(now + buffer_duration);
                if max_deadline.is_none() {
                    max_deadline = Some(now + max_duration);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                debug!("Debounce loop disconnected upstream, shutting down");
                break;
            }
        }
    }
}