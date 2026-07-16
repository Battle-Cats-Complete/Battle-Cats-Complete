use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::common::region::Region;

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbImportType {
    All,
    Update,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbTarget {
    Specific(Region),
    All,
}

impl AdbTarget {
    pub fn suffix(&self) -> &'static str {
        match self {
            AdbTarget::Specific(region) => region.metadata().package_suffix,
            AdbTarget::All => "all",
        }
    }

    pub fn as_name(&self) -> &'static str {
        match self {
            AdbTarget::Specific(region) => region.metadata().display_name,
            AdbTarget::All => "All Regions",
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DataTab {
    Import,
    Export
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportSubTab {
    Emulator,
    Sort,
    Decrypt,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode { None, Folder, Zip }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct DataConfigState {
    pub active_tab: DataTab,

    #[serde(skip)] pub selected_job: Option<ImportSubTab>,
    pub import_path: String,
    pub import_mode: ImportMode,
    pub adb_import_type: AdbImportType,
    pub adb_target: AdbTarget,
    pub decrypt_path: String,

    pub export_filename: String,
    pub compression_level: i32,
    pub include_raw: bool,

    #[serde(skip)] pub import_log_content: String,
    #[serde(skip)] pub import_rx: Option<Receiver<String>>,
    #[serde(skip)] pub import_job_status: Arc<AtomicU8>,
    #[serde(skip)] pub import_abort_flag: Arc<AtomicBool>,
    #[serde(skip)] pub import_job_completed_time: Option<Instant>,
    #[serde(skip)] pub import_job_aborted_time: Option<Instant>,
    #[serde(skip)] pub import_progress_current: Arc<AtomicUsize>,
    #[serde(skip)] pub import_progress_maximum: Arc<AtomicUsize>,

    #[serde(skip)] pub export_log_content: String,
    #[serde(skip)] pub export_rx: Option<Receiver<String>>,
    #[serde(skip)] pub export_job_status: Arc<AtomicU8>,
    #[serde(skip)] pub export_abort_flag: Arc<AtomicBool>,
    #[serde(skip)] pub export_job_completed_time: Option<Instant>,
    #[serde(skip)] pub export_job_aborted_time: Option<Instant>,
    #[serde(skip)] pub export_progress_current: Arc<AtomicUsize>,
    #[serde(skip)] pub export_progress_maximum: Arc<AtomicUsize>,
}

impl Default for DataConfigState {
    fn default() -> Self {
        Self {
            active_tab: DataTab::Import,
            selected_job: None,
            import_path: String::new(),
            import_mode: ImportMode::Folder,
            adb_import_type: AdbImportType::All,
            adb_target: AdbTarget::Specific(Region::En),
            decrypt_path: String::new(),
            export_filename: String::new(),
            compression_level: 9,
            include_raw: false,

            import_log_content: String::new(),
            import_rx: None,
            import_job_status: Arc::new(AtomicU8::new(0)),
            import_abort_flag: Arc::new(AtomicBool::new(false)),
            import_job_completed_time: None,
            import_job_aborted_time: None,
            import_progress_current: Arc::new(AtomicUsize::new(0)),
            import_progress_maximum: Arc::new(AtomicUsize::new(0)),

            export_log_content: String::new(),
            export_rx: None,
            export_job_status: Arc::new(AtomicU8::new(0)),
            export_abort_flag: Arc::new(AtomicBool::new(false)),
            export_job_completed_time: None,
            export_job_aborted_time: None,
            export_progress_current: Arc::new(AtomicUsize::new(0)),
            export_progress_maximum: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub struct UpdateFlags {
    pub import_finished_just_now: bool,
    pub needs_repaint: bool,
}

impl DataConfigState {
    pub fn tick_threads(&mut self) -> UpdateFlags {
        let mut flags = UpdateFlags { import_finished_just_now: false, needs_repaint: false };

        if let Some(receiver) = &self.import_rx {
            while let Ok(message) = receiver.try_recv() {
                self.import_log_content.push_str(&format!("{}\n", message));
            }
        }

        if let Some(receiver) = &self.export_rx {
            while let Ok(message) = receiver.try_recv() {
                self.export_log_content.push_str(&format!("{}\n", message));
            }
        }

        let import_status_value = self.import_job_status.load(Ordering::Relaxed);

        if import_status_value == 1 {
            flags.needs_repaint = true;
        } else if import_status_value == 2 || import_status_value == 3 {
            if self.import_abort_flag.load(Ordering::Relaxed) {
                self.import_job_aborted_time = Some(Instant::now());
            } else if import_status_value == 2 {
                flags.import_finished_just_now = true;
                self.import_job_completed_time = Some(Instant::now());
            }
            self.import_job_status.store(0, Ordering::Relaxed);
            self.import_abort_flag.store(false, Ordering::Relaxed);
            self.import_rx = None;
            flags.needs_repaint = true;
        }

        if let Some(time) = self.import_job_completed_time {
            if time.elapsed().as_secs() < 2 { flags.needs_repaint = true; }
            else { self.import_job_completed_time = None; }
        }

        if let Some(time) = self.import_job_aborted_time {
            if time.elapsed().as_secs() < 2 { flags.needs_repaint = true; }
            else { self.import_job_aborted_time = None; }
        }

        let export_status_value = self.export_job_status.load(Ordering::Relaxed);

        if export_status_value == 1 {
            flags.needs_repaint = true;
        } else if export_status_value == 2 || export_status_value == 3 {
            if self.export_abort_flag.load(Ordering::Relaxed) {
                self.export_job_aborted_time = Some(Instant::now());
            } else if export_status_value == 2 {
                self.export_job_completed_time = Some(Instant::now());
            }
            self.export_job_status.store(0, Ordering::Relaxed);
            self.export_abort_flag.store(false, Ordering::Relaxed);
            self.export_rx = None;
            flags.needs_repaint = true;
        }

        if let Some(time) = self.export_job_completed_time {
            if time.elapsed().as_secs() < 2 { flags.needs_repaint = true; }
            else { self.export_job_completed_time = None; }
        }

        if let Some(time) = self.export_job_aborted_time {
            if time.elapsed().as_secs() < 2 { flags.needs_repaint = true; }
            else { self.export_job_aborted_time = None; }
        }

        flags
    }
}