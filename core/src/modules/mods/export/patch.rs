use std::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use std::thread;

use tracing::{error, info, trace};

use crate::modules::mods::logic::state::ModDataState;

pub enum ExportEvent {
    Log(String),
    Success(String),
    Error(String),
}

pub static EVENT_RECEIVER: Mutex<Option<Receiver<ExportEvent>>> = Mutex::new(None);

pub fn spawn_log_adapter(event_transmitter: Sender<ExportEvent>) -> Sender<String> {
    let (string_transmitter, string_receiver) = mpsc::channel();

    thread::spawn(move || {
        trace!("Spawned log adapter thread.");
        for message in string_receiver {
            let _ = event_transmitter.send(ExportEvent::Log(message));
        }
    });

    string_transmitter
}

pub fn process_events(state: &mut ModDataState) -> bool {
    let mut is_busy = state.export.is_busy;

    let Ok(guard) = EVENT_RECEIVER.try_lock() else { return is_busy; };
    let Some(receiver) = guard.as_ref() else { return is_busy; };

    while let Ok(event) = receiver.try_recv() {
        match event {
            ExportEvent::Log(message) => {
                trace!("Export Log: {}", message);
                state.export.log_content.push_str(&format!("{}\n", message));
            },
            ExportEvent::Success(message) => {
                info!("Export Success: {}", message);
                state.export.log_content.push_str(&format!("{}\n", message));
                state.export.status_message = "Complete!".to_string();
                state.export.is_busy = false;
                is_busy = false;
            },
            ExportEvent::Error(error_message) => {
                error!("Export Error: {}", error_message);
                state.export.log_content.push_str(&format!("!! ERROR: {}\n", error_message));
                state.export.status_message = "Failed".to_string();
                state.export.is_busy = false;
                is_busy = false;
            }
        }
    }

    is_busy
}