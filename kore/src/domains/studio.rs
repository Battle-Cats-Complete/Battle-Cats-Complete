mod blank;

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{debug, info, warn};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::common::architecture::{GAME, MODS, STUDIO};
use crate::systems::animation::{self, Clip, ClipSet, Loop, Rigging};

pub use blank::SEED_SUFFIX;

const EXPORT_DIR: &str = "exports";
const SHEET_EXT: &str = "png";
const CUTS_EXT: &str = "imgcut";
const MODEL_EXT: &str = "mamodel";
const ANIM_EXT: &str = "maanim";
const DEFAULT_NAME: &str = "New Set";
pub const SEED_NAME: &str = "New";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Sheet,
    Cuts,
    Model,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Sheet, Slot::Cuts, Slot::Model];

    pub fn label(self) -> &'static str {
        match self {
            Slot::Sheet => "PNG",
            Slot::Cuts => "IMGCUT",
            Slot::Model => "MAMODEL",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Slot::Sheet => SHEET_EXT,
            Slot::Cuts => CUTS_EXT,
            Slot::Model => MODEL_EXT,
        }
    }

    pub fn filter(self) -> &'static [&'static str] {
        match self {
            Slot::Sheet => &[SHEET_EXT],
            Slot::Cuts => &[CUTS_EXT],
            Slot::Model => &[MODEL_EXT],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Home {
    Game,
    Mod,
    Studio,
    Loose,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Set {
    pub name: String,
    pub sheet: Option<PathBuf>,
    pub cuts: Option<PathBuf>,
    pub model: Option<PathBuf>,
    pub anims: Vec<PathBuf>,
}

impl Set {
    pub fn slot(&self, slot: Slot) -> Option<&Path> {
        match slot {
            Slot::Sheet => self.sheet.as_deref(),
            Slot::Cuts => self.cuts.as_deref(),
            Slot::Model => self.model.as_deref(),
        }
    }

    pub fn place(&mut self, slot: Slot, path: Option<PathBuf>) {
        match slot {
            Slot::Sheet => self.sheet = path,
            Slot::Cuts => self.cuts = path,
            Slot::Model => self.model = path,
        }
    }

    pub fn rigged(&self) -> bool {
        self.sheet.is_some() && self.cuts.is_some() && self.model.is_some()
    }

    pub fn files(&self) -> Vec<PathBuf> {
        Slot::ALL
            .into_iter()
            .filter_map(|slot| self.slot(slot).map(Path::to_path_buf))
            .chain(self.anims.iter().cloned())
            .collect()
    }

    pub fn key(&self) -> String {
        let mut key = self.name.clone();

        for path in self.files() {
            key.push('|');
            key.push_str(&path.to_string_lossy());
        }

        key
    }

    pub fn rig_id(&self) -> String {
        let (Some(sheet), Some(cuts), Some(model)) = (&self.sheet, &self.cuts, &self.model) else {
            return String::new();
        };

        format!("{}|{}|{}", sheet.display(), cuts.display(), model.display())
    }

    pub fn home(&self) -> Home {
        self.files().first().map_or(Home::Loose, |path| home(path))
    }

    pub fn clips(&self) -> ClipSet {
        let (Some(sheet), Some(cuts), Some(model)) = (&self.sheet, &self.cuts, &self.model) else {
            return ClipSet::default();
        };

        let rig = Arc::new(Rigging {
            id: self.rig_id(),
            png: sheet.clone(),
            cut: cuts.clone(),
            model: model.clone(),
        });

        let stem = stem_of(model);
        let mut clips: Vec<Clip> = self
            .anims
            .iter()
            .map(|anim| {
                let named = ordinal(anim, &stem).and_then(animation::standard);

                Clip {
                    name: named.map(|(name, _, _)| name.to_owned()),
                    slot: named.map(|(_, slot, _)| slot),
                    role: named.map(|(_, _, role)| role),
                    looping: match named.is_some_and(|(_, _, role)| role.loops()) {
                        true => Loop::Exact,
                        false => Loop::Frames,
                    },
                    rig: Arc::clone(&rig),
                    anim: Some(anim.clone()),
                }
            })
            .collect();

        clips.push(Clip::model(rig));

        ClipSet { name: self.name.clone(), clips, offsets: vec!["Combat", "Gacha"] }
    }
}

pub fn root() -> PathBuf {
    PathBuf::from(STUDIO)
}

pub fn sets() -> Vec<String> {
    let Ok(entries) = fs::read_dir(root()) else {
        return Vec::new();
    };

    let mut listed: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();

    listed.sort_by_key(|name| name.to_lowercase());
    listed
}

pub fn home(path: &Path) -> Home {
    let anchored = env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());

    if anchored.starts_with(GAME) {
        return Home::Game;
    }

    if anchored.starts_with(MODS) {
        return Home::Mod;
    }

    if anchored.starts_with(STUDIO) {
        return Home::Studio;
    }

    Home::Loose
}

pub fn folder_name(set: &Set) -> Option<String> {
    let path = set.files().into_iter().find(|path| home(path) == Home::Studio)?;

    path.parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(str::to_owned)
}

pub fn pullable(set: &Set, unlocked: bool) -> bool {
    set.files().iter().any(|path| match home(path) {
        Home::Studio | Home::Mod => false,
        Home::Game => !unlocked,
        Home::Loose => true,
    })
}

pub fn load(name: &str) -> Set {
    let folder = root().join(name);
    let mut set = Set { name: name.to_owned(), ..Set::default() };

    let Ok(entries) = fs::read_dir(&folder) else {
        warn!(name, "Studio could not read the set folder");

        return set;
    };

    let mut found: Vec<PathBuf> =
        entries.flatten().map(|entry| entry.path()).filter(|path| path.is_file()).collect();

    found.sort();

    for path in found {
        match extension(&path).as_deref() {
            Some(SHEET_EXT) if set.sheet.is_none() => set.sheet = Some(path),
            Some(CUTS_EXT) if set.cuts.is_none() => set.cuts = Some(path),
            Some(MODEL_EXT) if set.model.is_none() => set.model = Some(path),
            Some(ANIM_EXT) => set.anims.push(path),
            _ => {}
        }
    }

    set
}

pub fn siblings(picked: &Path) -> Set {
    let stem = picked.with_extension("");
    let name = stem.file_name().and_then(OsStr::to_str).unwrap_or(DEFAULT_NAME).to_owned();

    let held = |ext: &str| {
        let path = stem.with_extension(ext);

        path.is_file().then_some(path)
    };

    let mut set = Set {
        name,
        sheet: held(SHEET_EXT),
        cuts: held(CUTS_EXT),
        model: held(MODEL_EXT),
        anims: Vec::new(),
    };

    if let Some(folder) = picked.parent() {
        set.anims = kin(folder, &stem);
    }

    set
}

fn kin(folder: &Path, stem: &Path) -> Vec<PathBuf> {
    let Some(base) = stem.file_name().and_then(OsStr::to_str) else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| extension(path).as_deref() == Some(ANIM_EXT))
        .filter(|path| {
            path.file_stem().and_then(OsStr::to_str).is_some_and(|name| name.starts_with(base))
        })
        .collect();

    found.sort();
    found
}

pub fn vacant(wanted: &str) -> String {
    let base = sanitize(wanted);

    if !root().join(&base).exists() {
        return base;
    }

    (1..)
        .map(|at| format!("{}{}", base, at))
        .find(|name| !root().join(name).exists())
        .unwrap_or(base)
}

pub fn sanitize(wanted: &str) -> String {
    let cleaned: String = wanted
        .chars()
        .filter(|glyph| !matches!(glyph, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();

    match cleaned.trim() {
        "" => DEFAULT_NAME.to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

pub fn adopt(name: &str, set: &Set) -> io::Result<Set> {
    let folder = root().join(name);

    fs::create_dir_all(&folder)?;

    let seat = |source: &Path| -> io::Result<PathBuf> {
        let file = source.file_name().unwrap_or_else(|| OsStr::new("asset"));
        let destination = folder.join(file);

        if source != destination {
            fs::copy(source, &destination)?;
        }

        Ok(destination)
    };

    let mut adopted = Set { name: name.to_owned(), ..Set::default() };

    for slot in Slot::ALL {
        if let Some(source) = set.slot(slot) {
            adopted.place(slot, Some(seat(source)?));
        }
    }

    for anim in &set.anims {
        adopted.anims.push(seat(anim)?);
    }

    info!(name, files = adopted.files().len(), "Studio adopted a set");

    Ok(adopted)
}

pub fn rename(from: &str, to: &str) -> io::Result<()> {
    let (source, destination) = (root().join(from), root().join(to));

    if source == destination {
        return Ok(());
    }

    fs::rename(&source, &destination)?;
    debug!(from, to, "Studio renamed a set folder");

    Ok(())
}

pub fn seed(name: &str) -> io::Result<Set> {
    let folder = root().join(name);

    fs::create_dir_all(&folder)?;

    let seated = |stem: &str, extension: &str| folder.join(format!("{}.{}", stem, extension));

    let sheet = seated(name, SHEET_EXT);
    let cuts = seated(name, CUTS_EXT);
    let model = seated(name, MODEL_EXT);
    let anim = seated(&format!("{}{}", name, blank::SEED_SUFFIX), ANIM_EXT);

    fs::write(&sheet, blank::sheet())?;
    fs::write(&cuts, blank::cuts(name))?;
    fs::write(&model, blank::model())?;
    fs::write(&anim, blank::anim())?;

    info!(name, "Studio seeded a new set");

    Ok(Set { name: name.to_owned(), sheet: Some(sheet), cuts: Some(cuts), model: Some(model), anims: vec![anim] })
}

pub fn export(set: &Set) -> io::Result<PathBuf> {
    let files = set.files();

    if files.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the set holds no files"));
    }

    let folder = PathBuf::from(EXPORT_DIR);

    fs::create_dir_all(&folder)?;

    let stem = match set.name.trim() {
        "" => set.model.as_deref().map_or_else(|| DEFAULT_NAME.to_owned(), stem_of),
        named => sanitize(named),
    };

    let mut path = folder.join(format!("{}.zip", stem));
    let mut counter = 1;

    while path.exists() {
        path = folder.join(format!("{}{}.zip", stem, counter));
        counter += 1;
    }

    let mut zip = ZipWriter::new(File::create(&path)?);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for file in &files {
        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        let body = fs::read(file)?;

        zip.start_file(name, options)?;
        io::Write::write_all(&mut zip, &body)?;
    }

    zip.finish()?;
    info!(path = %path.display(), files = files.len(), "Studio exported a set");

    Ok(path)
}

fn extension(path: &Path) -> Option<String> {
    path.extension().and_then(OsStr::to_str).map(str::to_lowercase)
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map_or_else(|| DEFAULT_NAME.to_owned(), |stem| stem.to_string_lossy().into_owned())
}

fn ordinal(anim: &Path, stem: &str) -> Option<usize> {
    anim.file_stem()
        .and_then(OsStr::to_str)
        .and_then(|name| name.strip_prefix(stem))
        .and_then(|suffix| suffix.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_standard_ordinal_names_its_clip_and_anything_else_stays_bare() {
        assert_eq!(ordinal(Path::new("game/044_f02.maanim"), "044_f"), Some(2));
        assert_eq!(ordinal(Path::new("game/044_f_zombie00.maanim"), "044_f"), None);
        assert_eq!(ordinal(Path::new("game/other.maanim"), "044_f"), None);
    }

    #[test]
    fn a_name_loses_path_separators_but_keeps_the_rest() {
        assert_eq!(sanitize("my/set"), "myset");
        assert_eq!(sanitize("  spaced  "), "spaced");
        assert_eq!(sanitize("   "), DEFAULT_NAME);
    }

    #[test]
    fn a_set_names_its_standard_clips_and_leaves_the_rest_bare() {
        // The four leading ordinals are the engine's own roles; anything else is
        // just a file the viewer lists by name.
        let set = Set {
            name: "044_f".to_owned(),
            sheet: Some(PathBuf::from("studio/a/044_f.png")),
            cuts: Some(PathBuf::from("studio/a/044_f.imgcut")),
            model: Some(PathBuf::from("studio/a/044_f.mamodel")),
            anims: vec![
                PathBuf::from("studio/a/044_f00.maanim"),
                PathBuf::from("studio/a/044_f02.maanim"),
                PathBuf::from("studio/a/044_f_zombie00.maanim"),
            ],
        };

        let named: Vec<Option<String>> = set.clips().clips.iter().map(|clip| clip.name.clone()).collect();

        assert_eq!(
            named,
            vec![
                Some("Walk".to_owned()),
                Some("Attack".to_owned()),
                None,
                Some("Model".to_owned()),
            ]
        );
    }

    #[test]
    fn a_set_is_only_rigged_once_all_three_assets_are_there() {
        let mut set = Set { sheet: Some(PathBuf::from("a.png")), ..Set::default() };

        assert!(!set.rigged());

        set.cuts = Some(PathBuf::from("a.imgcut"));
        assert!(!set.rigged());

        set.model = Some(PathBuf::from("a.mamodel"));
        assert!(set.rigged());
    }

    #[test]
    fn every_mount_is_told_apart() {
        assert_eq!(home(Path::new("game/cats/044/f/anim/044_f.mamodel")), Home::Game);
        assert_eq!(home(Path::new("mods/MyMod/044_f.mamodel")), Home::Mod);
        assert_eq!(home(Path::new("studio/Test/044_f.mamodel")), Home::Studio);
        assert_eq!(home(Path::new("/tmp/somewhere/044_f.mamodel")), Home::Loose);
    }

    #[test]
    fn only_an_unwritable_set_has_to_be_pulled_into_studio() {
        // A mod is writable by definition, `studio/` already is, and `game` is only
        // when the mount is unlocked.
        let seat = |path: &str| Set {
            sheet: Some(PathBuf::from(path)),
            cuts: Some(PathBuf::from(path)),
            model: Some(PathBuf::from(path)),
            ..Set::default()
        };

        assert!(!pullable(&seat("studio/a/x.png"), false));
        assert!(!pullable(&seat("mods/MyMod/x.png"), false));
        assert!(pullable(&seat("game/x.png"), false));
        assert!(!pullable(&seat("game/x.png"), true));
        assert!(pullable(&seat("/tmp/x.png"), true));
    }
}
