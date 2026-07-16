use std::env;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use directories::BaseDirs;
use tracing_subscriber::{fmt, EnvFilter};

fn find_override_file(cwd: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|name| cwd.join(name)).find(|path| path.exists())
}

pub(crate) fn init(enable_logging: bool) {
    let cwd = env::current_dir().unwrap_or_default();

    let trace_file = find_override_file(&cwd, &["trace.txt", "trace"]);
    let debug_file = find_override_file(&cwd, &["debug.txt", "debug"]);

    let app_dir = BaseDirs::new().map(|base| base.data_local_dir().join("battle_cats_complete"));
    
    let (log_level, filter_directive, file_path) = if let Some(path) = trace_file {
        (tracing::Level::TRACE, "info,gui=trace,core=trace,nyanko=trace,zbus=error", path)
    } else if let Some(path) = debug_file {
        (tracing::Level::DEBUG, "info,gui=debug,core=debug,nyanko=debug,zbus=error", path)
    } else if enable_logging {
        let Some(dir) = app_dir else { return };

        if fs::create_dir_all(&dir).is_err() {
            return;
        }

        let log_file = dir.join("logs.txt");
        let prev_log = dir.join("logs.prev.txt");

        if log_file.exists() {
            let _ = fs::rename(&log_file, &prev_log);
        }

        (tracing::Level::INFO, "info,zbus=error", log_file)
    } else {
        if let Some(dir) = app_dir {
            let _ = fs::remove_file(dir.join("logs.txt"));
            let _ = fs::remove_file(dir.join("logs.prev.txt"));
        }
        return;
    };

    if let Ok(file) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_path)
    {
        let filter = EnvFilter::new(filter_directive);

        let subscriber = fmt::Subscriber::builder()
            .with_file(true)
            .with_line_number(true)
            .with_env_filter(filter)
            .with_writer(Arc::new(file))
            .with_ansi(false)
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);

        tracing::info!("Tracing initialized at {} level", log_level);
        tracing::debug!("Active filter directive: {}", filter_directive);
    }
}