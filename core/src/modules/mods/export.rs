pub mod apk;
pub mod bcm;
pub mod modify;
pub mod pack;
pub mod sign;

use std::path::PathBuf;
use std::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tracing::{error, info, trace};

use crate::common::region::Region;

use super::ModDataState;

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum ExportType {
    #[default]
    Apk,
    Bcm,
    Pack,
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub enum PatchMode {
    #[default]
    Update,
    Create,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExportState {
    #[serde(skip)] pub is_open: bool,
    #[serde(skip)] pub is_busy: bool,
    pub tab: ExportType,
    pub patch_mode: PatchMode,
    pub target_region: Region,
    pub app_title: String,
    pub package_suffix: String,
    pub pack_name: String,
    #[serde(skip)] pub selected_apk: Option<PathBuf>,
    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_busy: false,
            tab: ExportType::Apk,
            patch_mode: PatchMode::Update,
            target_region: Region::En,
            app_title: String::new(),
            package_suffix: String::new(),
            pack_name: String::new(),
            selected_apk: None,
            status_message: "Ready to export.".to_string(),
            log_content: String::new(),
        }
    }
}

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