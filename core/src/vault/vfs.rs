//! Virtual File System
mod disk;
mod regional;
mod walk;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::modules::data::architecture;
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
    conflicts: Vec<Conflict>,
}

impl MountedDir {
    fn prune(&mut self, relative: &Path) -> Vec<MountKey> {
        let mut removed = Vec::new();

        if let Some(name) = relative.file_name().and_then(OsStr::to_str)
            && self.files.get(name).is_some_and(|entry| entry.path == relative)
        {
            self.files.remove(name);
            self.unlink(relative, name, true);

            return vec![name.into()];
        }

        let targets: Vec<MountKey> = self
            .dirs
            .keys()
            .filter(|dir| Path::new(dir.as_ref()).starts_with(relative))
            .cloned()
            .collect();

        for dir in targets {
            self.folders.remove(&dir);

            let Some(names) = self.dirs.remove(&dir) else {
                continue;
            };

            for name in names {
                if self.files.get(name.as_ref()).is_some_and(|entry| entry.path.starts_with(relative)) {
                    self.files.remove(name.as_ref());
                    removed.push(name);
                }
            }
        }

        if let Some(name) = relative.file_name().and_then(OsStr::to_str) {
            self.unlink(relative, name, false);
        }

        removed
    }

    fn unlink(&mut self, relative: &Path, name: &str, is_file: bool) {
        let Some(parent) = relative.parent() else {
            return;
        };

        let listing = if is_file {
            self.dirs.get_mut(parent.to_string_lossy().as_ref())
        } else {
            self.folders.get_mut(parent.to_string_lossy().as_ref())
        };

        if let Some(listing) = listing {
            listing.retain(|existing| existing.as_ref() != name);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[error("{path} does not sit under the '{root}' mount")]
    Unrooted { root: PathBuf, path: PathBuf },
    #[error("directory claims the reserved '{MOUNT_GAME}' mount key: {0}")]
    ReservedMount(PathBuf),
    #[error("failed to read {path}: {source}")]
    Walk { path: PathBuf, source: std::io::Error },
    #[error("virtual file system state is unavailable")]
    Unavailable,
}

pub trait Target {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R;
}

impl Target for &str {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&[self])
    }
}

impl Target for &String {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&[self.as_str()])
    }
}

impl<S: AsRef<str>> Target for &[S] {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&self.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
    }
}

impl<S: AsRef<str>, const N: usize> Target for &[S; N] {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&self.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
    }
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

    pub(crate) fn detached() -> Self {
        let mut order = Vec::new();
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

    pub fn find<T: Target>(&self, target: T) -> Option<PathBuf> {
        target.resolve(|names| self.first(names))
    }

    pub fn list<T: Target>(&self, target: T) -> Vec<PathBuf> {
        target.resolve(|names| self.collect(names))
    }

    pub fn locate(&self, filename: &str) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;

        resolve(&mounts, filename)
    }

    fn first(&self, filenames: &[&str]) -> Option<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return None;
        };

        let Ok(mounts) = self.mounts.read() else {
            return None;
        };

        regional::interleaved(filenames, &order).find_map(|candidate| resolve(&mounts, &candidate))
    }

    fn collect(&self, filenames: &[&str]) -> Vec<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return Vec::new();
        };

        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for candidate in regional::interleaved(filenames, &order) {
            if let Some(path) = resolve(&mounts, &candidate) {
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

    pub fn purge(&self, filenames: &[Box<str>]) {
        if let Ok(mut cache) = self.cache.write() {
            for filename in filenames {
                cache.remove(filename.as_ref());
            }
        }
    }

    pub fn prune(&self, mount: &str, path: &Path) -> Vec<Box<str>> {
        let Ok(mut mounts) = self.mounts.write() else {
            return Vec::new();
        };

        let Some(indexed) = mounts.get_mut(mount) else {
            return Vec::new();
        };

        let Some(relative) = relative_to(&indexed.root, path) else {
            return Vec::new();
        };

        indexed.prune(&relative)
    }

    pub fn count(&self, mount: &str) -> usize {
        self.mounts
            .read()
            .map_or(0, |mounts| mounts.get(mount).map_or(0, |indexed| indexed.files.len()))
    }

    pub fn glob(&self, prefix: &str) -> Vec<Box<str>> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut names: Vec<Box<str>> = mounts
            .values()
            .flat_map(|indexed| indexed.files.keys())
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();

        names.sort_unstable();
        names.dedup();
        names
    }

    pub fn conflicts(&self) -> Vec<Conflict> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut collisions: Vec<Conflict> = mounts.values().flat_map(|indexed| indexed.conflicts.iter().cloned()).collect();
        collisions.sort_by(|a, b| a.key.cmp(&b.key));
        collisions
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
        let encoded = self.mounts.read().ok().and_then(|mounts| disk::encode(&mounts, hash));

        let Some(bytes) = encoded else {
            return;
        };

        disk::store(&bytes);
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

        let skip: &[&str] = if key == MOUNT_GAME { &[] } else { &architecture::MOD_TRANSIENT };
        let mount = walk::walk(self, skip)?;
        let conflicts = mount.conflicts.clone();

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

        let Some(relative) = relative_to(&mount.root, file) else {
            return Err(VfsError::Unrooted { root: mount.root.clone(), path: file.to_path_buf() });
        };

        let (mtime, len) = walk::stat(file);

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

        {
            let Ok(mut mounts) = vfs.mounts.write() else {
                return;
            };

            let Some(mount) = mounts.get_mut(key) else {
                return;
            };

            let Some(relative) = relative_to(&mount.root, file) else {
                return;
            };

            if mount.files.get(name).is_none_or(|entry| entry.path != relative) {
                return;
            }

            let removed = mount.files.remove(name);

            if let Some(entry) = removed
                && let Some(parent) = entry.path.parent()
                && let Some(listing) = mount.dirs.get_mut(parent.to_string_lossy().as_ref())
            {
                listing.retain(|existing| existing.as_ref() != name);
            }
        }

        vfs.evict(name);
    }
}

fn relative_to(root: &Path, file: &Path) -> Option<PathBuf> {
    if let Ok(relative) = file.strip_prefix(root) {
        return Some(relative.to_path_buf());
    }

    let absolute = env::current_dir().ok()?.join(root);

    if let Ok(relative) = file.strip_prefix(&absolute) {
        return Some(relative.to_path_buf());
    }

    file.strip_prefix(fs::canonicalize(&absolute).ok()?).ok().map(Path::to_path_buf)
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
