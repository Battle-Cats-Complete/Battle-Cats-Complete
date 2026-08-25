use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc as async_mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::stream;
use notify::event::ModifyKind;
use notify::{recommended_watcher, ErrorKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, trace, warn};

use kore::common::architecture;
use kore::common::junk;

const BATCH_BUFFER: usize = 8;
const COALESCE: Duration = Duration::from_millis(20);
const SETTLE_POLL: Duration = Duration::from_millis(15);

#[cfg(windows)]
const SETTLE_STREAK: u8 = 1;
#[cfg(not(windows))]
const SETTLE_STREAK: u8 = 2;
const SETTLE_CEILING: Duration = Duration::from_secs(2);
const BULK_QUIET: Duration = Duration::from_millis(500);
const IDLE_TICK: Duration = Duration::from_secs(2);
const FLOOR: Duration = Duration::from_millis(1);
const BULK_THRESHOLD: usize = 512;

static SUSPENDED: AtomicBool = AtomicBool::new(false);

pub(crate) fn suspend() {
    debug!("File watcher suspended for a bulk job");
    SUSPENDED.store(true, Ordering::Relaxed);
}

pub(crate) fn resume() {
    debug!("File watcher resumed");
    SUSPENDED.store(false, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asset {
    Unit(u32),
    Enemy(u32),
    Item(u32),
}

#[derive(Debug, Clone)]
pub enum Change {
    Batch(Vec<PathBuf>),
    Bulk,
    Unavailable(Lapse),
}

#[derive(Debug, Clone, Copy)]
pub enum Lapse {
    Crowded,
    Broken,
}

pub(crate) fn changes() -> impl Stream<Item = Change> {
    stream::channel(BATCH_BUFFER, |mut output: async_mpsc::Sender<Change>| async move {
        let (raw_tx, raw_rx) = channel();

        let _watcher = match spawn_watcher(raw_tx) {
            Ok(watcher) => watcher,
            Err(lapse) => {
                warn!(?lapse, "File watcher unavailable; live reload is disabled for this session");
                let _ = output.send(Change::Unavailable(lapse)).await;
                return;
            }
        };

        let (batch_tx, mut batch_rx) = async_mpsc::unbounded();
        thread::spawn(move || debounce(&raw_rx, &batch_tx));

        while let Some(change) = batch_rx.next().await {
            if output.send(change).await.is_err() {
                break;
            }
        }
    })
}

fn spawn_watcher(sender: Sender<Hit>) -> Result<RecommendedWatcher, Lapse> {
    let mut watcher = recommended_watcher(move |result: notify::Result<Event>| match result {
        Ok(event) => forward(event, &sender),
        Err(err) => warn!("File watcher reported an error: {}", err),
    })
    .map_err(|err| {
        warn!("Failed to create the file watcher: {}", err);

        Lapse::Broken
    })?;

    for root in [architecture::MODS, architecture::GAME] {
        let path = Path::new(root);

        if !path.exists() && let Err(err) = fs::create_dir_all(path) {
            warn!(path = %path.display(), "Could not create a watch root, live reload will miss it: {}", err);
            continue;
        }

        watch(&mut watcher, path, RecursiveMode::Recursive)?;
    }

    Ok(watcher)
}

fn watch(watcher: &mut RecommendedWatcher, path: &Path, mode: RecursiveMode) -> Result<(), Lapse> {
    let Err(err) = watcher.watch(path, mode) else {
        return Ok(());
    };

    if matches!(err.kind, ErrorKind::MaxFilesWatch) {
        warn!(
            path = %path.display(),
            "This system has no free watch slots left. Another copy of the app, an IDE or a search \
             indexer is likely holding them; on Linux, raise fs.inotify.max_user_watches"
        );

        return Err(Lapse::Crowded);
    }

    warn!(path = %path.display(), "Failed to watch directory: {}", err);

    Err(Lapse::Broken)
}

fn forward(event: Event, sender: &Sender<Hit>) {
    if SUSPENDED.load(Ordering::Relaxed) || matches!(event.kind, EventKind::Access(_)) {
        return;
    }

    let complete = settled(&event.kind);

    for path in event.paths {
        if !is_relevant(&path) {
            continue;
        }

        trace!(complete, "File watcher detected a change: {:?}", path);
        let _ = sender.send(Hit { path, complete });
    }
}

fn settled(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_)))
}

fn is_relevant(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()).is_none_or(junk::ignored) {
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

    if parts[root + 1..].iter().any(|part| junk::ignored(part)) {
        return false;
    }

    if parts[root] == architecture::GAME {
        return true;
    }

    parts.len() > root + 1
}

struct Hit {
    path: PathBuf,
    complete: bool,
}

struct Probe {
    size: Option<u64>,
    streak: u8,
    due: Instant,
    expires: Instant,
}

impl Probe {
    fn new(path: &Path, now: Instant) -> Self {
        Self { size: sized(path), streak: 0, due: now + SETTLE_POLL, expires: now + SETTLE_CEILING }
    }
}

fn sized(path: &Path) -> Option<u64> {
    path.metadata().ok().map(|meta| meta.len())
}

#[cfg(windows)]
fn unlocked(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;

    fs::OpenOptions::new().read(true).share_mode(0).open(path).is_ok()
}

#[cfg(not(windows))]
fn unlocked(path: &Path) -> bool {
    fs::File::open(path).is_ok()
}

fn harvest(settling: &mut HashMap<PathBuf, Probe>, ready: &mut HashSet<PathBuf>, now: Instant) {
    settling.retain(|path, probe| {
        if probe.due > now {
            return true;
        }

        let size = sized(path);

        if size == probe.size && (!path.is_file() || unlocked(path)) {
            probe.streak += 1;
        } else {
            probe.size = size;
            probe.streak = 0;
        }

        if probe.streak < SETTLE_STREAK && now < probe.expires {
            probe.due = now + SETTLE_POLL;

            return true;
        }

        ready.insert(path.clone());

        false
    });
}

fn admit(hit: Hit, ready: &mut HashSet<PathBuf>, settling: &mut HashMap<PathBuf, Probe>, bulk: &mut Option<Instant>) {
    let now = Instant::now();

    if bulk.is_some() || ready.len() + settling.len() >= BULK_THRESHOLD {
        ready.clear();
        settling.clear();
        *bulk = Some(now + BULK_QUIET);

        return;
    }

    if hit.complete {
        settling.remove(&hit.path);
        ready.insert(hit.path);

        return;
    }

    ready.remove(&hit.path);

    settling
        .entry(hit.path)
        .and_modify(|probe| {
            probe.streak = 0;
            probe.due = now + SETTLE_POLL;
        })
        .or_insert_with_key(|path| Probe::new(path, now));
}

fn wait(now: Instant, flush_at: Option<Instant>, bulk: Option<Instant>, settling: &HashMap<PathBuf, Probe>) -> Duration {
    let next = settling.values().map(|probe| probe.due).min();

    [flush_at, bulk, next]
        .into_iter()
        .flatten()
        .min()
        .map_or(IDLE_TICK, |at| at.saturating_duration_since(now).max(FLOOR))
}

fn debounce(events: &Receiver<Hit>, batches: &async_mpsc::UnboundedSender<Change>) {
    let mut ready: HashSet<PathBuf> = HashSet::new();
    let mut settling: HashMap<PathBuf, Probe> = HashMap::new();
    let mut bulk: Option<Instant> = None;
    let mut flush_at: Option<Instant> = None;

    loop {
        let now = Instant::now();

        harvest(&mut settling, &mut ready, now);

        if !ready.is_empty() {
            flush_at = flush_at.or(Some(now + COALESCE));
        }

        if bulk.is_some_and(|at| at <= now) {
            debug!("File watcher escalating a bulk change to a full re-index");

            if batches.unbounded_send(Change::Bulk).is_err() {
                return;
            }

            bulk = None;
            ready.clear();
            settling.clear();
        } else if flush_at.is_some_and(|at| at <= now) && !ready.is_empty() {
            debug!("File watcher flushing {} settled paths", ready.len());

            if batches.unbounded_send(Change::Batch(ready.drain().collect())).is_err() {
                return;
            }
        }

        if ready.is_empty() {
            flush_at = None;
        }

        match events.recv_timeout(wait(now, flush_at, bulk, &settling)) {
            Ok(hit) => admit(hit, &mut ready, &mut settling, &mut bulk),
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

#[cfg(test)]
mod tests {
    use std::env;
    use std::io::Write;

    use notify::event::{CreateKind, RemoveKind, RenameMode};

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("bcc-watch-{name}-{}", std::process::id()));

            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("scratch root");

            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn drain(settling: &mut HashMap<PathBuf, Probe>, ready: &mut HashSet<PathBuf>, rounds: u8) {
        for _ in 0..rounds {
            let due = settling.values().map(|probe| probe.due).min();
            let Some(at) = due else { return };

            harvest(settling, ready, at);
        }
    }

    #[test]
    fn a_rename_into_place_skips_the_settle_wait() {
        // Our own saves stage to a hidden .tmp and rename, so the file is whole the
        // instant the watcher hears about it. That path must not pay the poll.
        assert!(settled(&EventKind::Modify(ModifyKind::Name(RenameMode::To))));
        assert!(settled(&EventKind::Remove(RemoveKind::File)));
        assert!(!settled(&EventKind::Create(CreateKind::File)));
        assert!(!settled(&EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content))));
    }

    #[test]
    fn a_growing_file_is_held_back_until_it_stops_growing() {
        let scratch = Scratch::new("growing");
        let path = scratch.0.join("unit044.csv");
        let mut file = fs::File::create(&path).expect("seed file");

        file.write_all(b"0,1,2\n").expect("first write");
        file.flush().expect("flush");

        let mut settling = HashMap::new();
        let mut ready = HashSet::new();

        settling.insert(path.clone(), Probe::new(&path, Instant::now()));

        for _ in 0..SETTLE_STREAK + 2 {
            file.write_all(b"3,4,5\n").expect("write");
            file.flush().expect("flush");

            drain(&mut settling, &mut ready, 1);

            assert!(ready.is_empty(), "a file that grew between probes must not be reported");
        }

        drop(file);

        drain(&mut settling, &mut ready, SETTLE_STREAK + 1);

        assert!(ready.contains(&path), "a closed, stable file must settle");
        assert!(settling.is_empty());
    }

    #[test]
    fn the_ceiling_releases_a_file_that_never_settles() {
        let scratch = Scratch::new("ceiling");
        let path = scratch.0.join("stuck.csv");

        fs::write(&path, "0,1,2\n").expect("seed file");

        let mut settling = HashMap::new();
        let mut ready = HashSet::new();
        let stale = Instant::now() - SETTLE_CEILING - Duration::from_millis(1);

        settling.insert(path.clone(), Probe { size: Some(0), streak: 0, due: stale, expires: stale });

        harvest(&mut settling, &mut ready, Instant::now());

        assert!(ready.contains(&path), "the ceiling must bound the wait");
    }
}
