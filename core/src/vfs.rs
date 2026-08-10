mod disk;
mod regional;
mod walk;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::modules::settings::{Settings, lang};

const MOUNT_GAME: &str = "game";

type MountKey = Box<str>;
type Index = FxHashMap<MountKey, MountedDir>;

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
    mtime: u64,
    len: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct MountedDir {
    root: PathBuf,
    files: FxHashMap<MountKey, Entry>,
    dirs: FxHashMap<Box<str>, Vec<Box<str>>>,
    folders: FxHashMap<Box<str>, Vec<Box<str>>>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub key: Box<str>,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum VfsError {
    #[error("mount target is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("path has no usable file name: {0}")]
    InvalidPath(PathBuf),
    #[error("no mount registered under key '{0}'")]
    UnknownMount(Box<str>),
    #[error("directory claims the reserved '{MOUNT_GAME}' mount key: {0}")]
    ReservedMount(PathBuf),
    #[error("failed to read {path}: {source}")]
    Walk { path: PathBuf, source: std::io::Error },
    #[error("virtual file system state is unavailable")]
    Unavailable,
}

pub trait Mount {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError>;
    fn unmount(self, vfs: &Vfs);
}

pub struct Vfs {
    mounts: RwLock<Index>,
    cache: RwLock<FxHashMap<MountKey, Arc<[u8]>>>,
    priority: RwLock<Vec<String>>,
}

impl Vfs {
    pub fn new(settings: &Settings) -> Self {
        let mut order = settings.general.language_priority.clone();
        lang::ensure_complete_list(&mut order);

        Self {
            mounts: RwLock::new(Index::default()),
            cache: RwLock::new(FxHashMap::default()),
            priority: RwLock::new(order),
        }
    }

    pub fn priority(&self, order: &[String]) {
        let mut complete = order.to_vec();
        lang::ensure_complete_list(&mut complete);

        if let Ok(mut current) = self.priority.write() {
            *current = complete;
        }

        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    pub fn create<M: Mount>(&self, target: M) -> Result<Vec<Conflict>, VfsError> {
        target.mount(self)
    }

    pub fn destroy<M: Mount>(&self, target: M) {
        target.unmount(self);
    }

    fn find(&self, filename: &str) -> Option<PathBuf> {
        self.list(filename).into_iter().next()
    }

    pub fn list(&self, filename: &str) -> Vec<PathBuf> {
        self.resolve_all(&regional::candidates(filename, &self.order()))
    }

    pub fn list_any(&self, filenames: &[&str]) -> Vec<PathBuf> {
        self.resolve_all(&regional::interleaved(filenames, &self.order()))
    }

    fn order(&self) -> Vec<String> {
        self.priority.read().map_or_else(|_| Vec::new(), |order| order.clone())
    }

    fn resolve_all(&self, candidates: &[String]) -> Vec<PathBuf> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for candidate in candidates {
            if let Some(path) = resolve(&mounts, candidate) {
                paths.push(path);
            }
        }

        paths.dedup();
        paths
    }

    pub fn load(&self, filename: &str) -> Option<Arc<[u8]>> {
        if let Ok(cache) = self.cache.read()
            && let Some(bytes) = cache.get(filename)
        {
            return Some(Arc::clone(bytes));
        }

        let path = self.find(filename)?;
        let raw = fs::read(&path)
            .inspect_err(|err| warn!(file = filename, path = %path.display(), "vfs read failed: {}", err))
            .ok()?;

        let bytes = Arc::<[u8]>::from(raw);

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(filename.into(), Arc::clone(&bytes));
        }

        Some(bytes)
    }

    pub fn refresh(&self, filename: &str) -> Option<Arc<[u8]>> {
        self.evict(filename);
        self.load(filename)
    }

    pub fn evict(&self, filename: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(filename);
        }
    }

    pub fn keys(&self, mount: &str) -> Vec<Box<str>> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        mounts
            .get(mount)
            .map_or_else(Vec::new, |indexed| indexed.files.keys().cloned().collect())
    }

    pub fn children(&self, dir: &Path) -> Vec<PathBuf> {
        self.matching(dir, "")
    }

    pub fn folders(&self, dir: &Path) -> Vec<PathBuf> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for mount in mounts.values() {
            let Ok(relative) = dir.strip_prefix(&mount.root) else {
                continue;
            };

            let key = relative.to_string_lossy();
            let Some(names) = mount.folders.get(key.as_ref()) else {
                continue;
            };

            for name in names {
                paths.push(dir.join(name.as_ref()));
            }
        }

        paths
    }

    pub fn matching(&self, dir: &Path, prefix: &str) -> Vec<PathBuf> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for mount in mounts.values() {
            let Ok(relative) = dir.strip_prefix(&mount.root) else {
                continue;
            };

            let key = relative.to_string_lossy();
            let Some(names) = mount.dirs.get(key.as_ref()) else {
                continue;
            };

            for name in names {
                if name.starts_with(prefix) {
                    paths.push(dir.join(name.as_ref()));
                }
            }
        }

        paths
    }

    pub fn persist(&self, hash: u64) {
        let Ok(mounts) = self.mounts.read() else {
            return;
        };

        disk::save(&mounts, hash);
    }

    pub fn restore(&self, hash: u64) -> bool {
        let Some((stored, index)) = disk::load() else {
            return false;
        };

        if stored != hash {
            return false;
        }

        let Ok(mut mounts) = self.mounts.write() else {
            return false;
        };

        *mounts = index;
        true
    }

    pub fn reconcile(&self) -> Vec<Box<str>> {
        let cached: Vec<Box<str>> = {
            let Ok(cache) = self.cache.read() else {
                return Vec::new();
            };

            cache.keys().cloned().collect()
        };

        let mut stale = Vec::new();
        for filename in cached {
            if self.current(&filename) {
                continue;
            }

            self.evict(&filename);
            stale.push(filename);
        }

        stale
    }

    fn current(&self, filename: &str) -> bool {
        let Some(path) = self.find(filename) else {
            return false;
        };

        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            return false;
        };

        let Ok(mounts) = self.mounts.read() else {
            return false;
        };

        mounts
            .values()
            .filter(|mount| path.starts_with(&mount.root))
            .find_map(|mount| mount.files.get(name))
            .is_some_and(|entry| walk::stat(&path) == (entry.mtime, entry.len))
    }
}

impl Mount for &Path {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError> {
        let Some(key) = self.file_name().and_then(OsStr::to_str) else {
            return Err(VfsError::InvalidPath(self.to_path_buf()));
        };

        if key == MOUNT_GAME && !is_canonical_game(self) {
            warn!(
                path = %self.display(),
                "refusing to index a directory claiming the reserved '{}' mount key",
                MOUNT_GAME
            );
            return Err(VfsError::ReservedMount(self.to_path_buf()));
        }

        let (mount, conflicts) = walk::walk(self)?;

        let Ok(mut mounts) = vfs.mounts.write() else {
            return Err(VfsError::Unavailable);
        };

        mounts.insert(key.into(), mount);
        Ok(conflicts)
    }

    fn unmount(self, vfs: &Vfs) {
        let Some(key) = self.file_name().and_then(OsStr::to_str) else {
            return;
        };

        if let Ok(mut mounts) = vfs.mounts.write() {
            mounts.remove(key);
        }
    }
}

impl Mount for (&str, &Path) {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError> {
        let (key, file) = self;

        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            return Err(VfsError::InvalidPath(file.to_path_buf()));
        };

        let Ok(mut mounts) = vfs.mounts.write() else {
            return Err(VfsError::Unavailable);
        };

        let Some(mount) = mounts.get_mut(key) else {
            return Err(VfsError::UnknownMount(key.into()));
        };

        let Ok(relative) = file.strip_prefix(&mount.root) else {
            return Err(VfsError::UnknownMount(key.into()));
        };

        let (mtime, len) = walk::stat(file);
        let relative = relative.to_path_buf();

        if let Some(parent) = relative.parent() {
            let listing = mount.dirs.entry(parent.to_string_lossy().into()).or_default();

            if !listing.iter().any(|existing| existing.as_ref() == name) {
                listing.push(name.into());
            }
        }

        mount.files.insert(name.into(), Entry { path: relative, mtime, len });
        Ok(Vec::new())
    }

    fn unmount(self, vfs: &Vfs) {
        let (key, file) = self;

        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            return;
        };

        vfs.evict(name);

        let Ok(mut mounts) = vfs.mounts.write() else {
            return;
        };

        let Some(mount) = mounts.get_mut(key) else {
            return;
        };

        let removed = mount.files.remove(name);

        if let Some(entry) = removed
            && let Some(parent) = entry.path.parent()
            && let Some(listing) = mount.dirs.get_mut(parent.to_string_lossy().as_ref())
        {
            listing.retain(|existing| existing.as_ref() != name);
        }
    }
}

fn resolve(mounts: &Index, name: &str) -> Option<PathBuf> {
    mounts
        .iter()
        .filter(|(key, _)| key.as_ref() != MOUNT_GAME)
        .chain(mounts.iter().filter(|(key, _)| key.as_ref() == MOUNT_GAME))
        .find_map(|(_, mount)| mount.files.get(name).map(|entry| mount.root.join(&entry.path)))
}

fn is_canonical_game(path: &Path) -> bool {
    path.components().count() == 1 && path.file_name() == Some(OsStr::new(MOUNT_GAME))
}
