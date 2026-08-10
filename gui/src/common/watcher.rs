use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc as async_mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::stream;
use notify::{recommended_watcher, ErrorKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, trace, warn};

use core::modules::data::architecture;

const BATCH_BUFFER: usize = 8;
const QUIET_WINDOW: Duration = Duration::from_millis(500);
const MAX_WINDOW: Duration = Duration::from_secs(2);

const MOD_CONTENT_DIRS: [&str; 3] = ["patch", "icons", "loose"];

const SCRATCH_SUFFIXES: [&str; 6] = [".kate-swp", ".swp", ".swo", ".tmp", ".bak", "~"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asset {
    Unit(u32),
    Enemy(u32),
    Item(u32),
}

#[derive(Debug, Clone)]
pub enum Change {
    Batch(Vec<PathBuf>),
    Unavailable,
}

pub(crate) fn changes() -> impl Stream<Item = Change> {
    stream::channel(BATCH_BUFFER, |mut output: async_mpsc::Sender<Change>| async move {
        let (raw_tx, raw_rx) = channel();

        let Some(_watcher) = spawn_watcher(raw_tx) else {
            warn!("File watcher unavailable; live reload is disabled for this session");
            let _ = output.send(Change::Unavailable).await;
            return;
        };

        let (batch_tx, mut batch_rx) = async_mpsc::unbounded();
        thread::spawn(move || debounce(&raw_rx, &batch_tx));

        while let Some(batch) = batch_rx.next().await {
            if output.send(Change::Batch(batch)).await.is_err() {
                break;
            }
        }
    })
}

fn spawn_watcher(sender: Sender<PathBuf>) -> Option<RecommendedWatcher> {
    let mut watcher = recommended_watcher(move |result: notify::Result<Event>| match result {
        Ok(event) => forward(event, &sender),
        Err(err) => warn!("File watcher reported an error: {}", err),
    })
    .inspect_err(|err| warn!("Failed to create the file watcher: {}", err))
    .ok()?;

    for root in [architecture::MODS, architecture::GAME] {
        let path = Path::new(root);

        if path.exists() && !watch(&mut watcher, path, RecursiveMode::Recursive) {
            return None;
        }
    }

    Some(watcher)
}

fn watch(watcher: &mut RecommendedWatcher, path: &Path, mode: RecursiveMode) -> bool {
    match watcher.watch(path, mode) {
        Ok(()) => true,
        Err(err) if matches!(err.kind, ErrorKind::MaxFilesWatch) => {
            warn!(
                path = %path.display(),
                "OS file-watch limit reached. Simplify the directory structure or raise fs.inotify.max_user_watches"
            );
            false
        }
        Err(err) => {
            warn!(path = %path.display(), "Failed to watch directory: {}", err);
            false
        }
    }
}

fn forward(event: Event, sender: &Sender<PathBuf>) {
    if matches!(event.kind, EventKind::Access(_)) {
        return;
    }

    for path in event.paths {
        if !is_relevant(&path) {
            continue;
        }

        trace!("File watcher detected a change: {:?}", path);
        let _ = sender.send(path);
    }
}

fn is_scratch(name: &str) -> bool {
    name.starts_with('.')
        || name.starts_with('#')
        || SCRATCH_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}

fn is_relevant(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()).is_none_or(is_scratch) {
        return false;
    }

    let parts: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect();

    let Some(root) = parts.iter().position(|part| part == architecture::GAME || part == architecture::MODS) else {
        return false;
    };

    if parts[root] == architecture::GAME {
        return parts.get(root + 1).is_none_or(|name| !architecture::TRANSIENT.contains(&name.as_str()));
    }

    parts
        .get(root + 2)
        .is_none_or(|folder| MOD_CONTENT_DIRS.contains(&folder.as_str()))
}

fn debounce(events: &Receiver<PathBuf>, batches: &async_mpsc::UnboundedSender<Vec<PathBuf>>) {
    let mut pending: HashSet<PathBuf> = HashSet::new();
    let mut quiet: Option<Instant> = None;
    let mut ceiling: Option<Instant> = None;

    loop {
        let mut timeout = MAX_WINDOW;

        if let (Some(quiet_at), Some(ceiling_at)) = (quiet, ceiling) {
            let deadline = quiet_at.min(ceiling_at);
            let now = Instant::now();

            if now < deadline {
                timeout = deadline.saturating_duration_since(now);
            } else {
                if !pending.is_empty() {
                    debug!("File watcher flushing {} changed paths", pending.len());

                    if batches.unbounded_send(pending.drain().collect()).is_err() {
                        return;
                    }
                }

                quiet = None;
                ceiling = None;
            }
        }

        match events.recv_timeout(timeout) {
            Ok(path) => {
                pending.insert(path);

                let now = Instant::now();
                quiet = Some(now + QUIET_WINDOW);
                ceiling = ceiling.or(Some(now + MAX_WINDOW));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                debug!("File watcher shut down");
                return;
            }
        }
    }
}

pub(crate) fn mount_of(path: &Path) -> Option<String> {
    let parts: Vec<&str> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();

    let root = parts.iter().position(|part| *part == architecture::GAME || *part == architecture::MODS)?;

    if parts[root] == architecture::GAME {
        return Some(architecture::GAME.to_string());
    }

    parts.get(root + 1).map(|name| (*name).to_string())
}

pub(crate) fn asset(name: &str) -> Option<Asset> {
    if let Some(rest) = name.strip_prefix("enemy_icon_") {
        return leading_id(rest).map(Asset::Enemy);
    }

    if let Some(rest) = name.strip_prefix("gatyaitemD_") {
        return leading_id(rest).map(Asset::Item);
    }

    let unit = name.strip_prefix("udi").or_else(|| name.strip_prefix("uni"))?;

    leading_id(unit).map(Asset::Unit)
}

fn leading_id(text: &str) -> Option<u32> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}
