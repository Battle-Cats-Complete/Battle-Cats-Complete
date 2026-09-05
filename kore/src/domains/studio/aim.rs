use std::ffi::OsStr;
use std::path::PathBuf;

use crate::systems::animation::{self, Role};
use crate::systems::animation::authoring::{Imgcut, Mamodel};

use super::{Set, Slot};

pub(super) const FORMS: [char; 4] = ['f', 'c', 's', 'u'];
pub(super) const ENEMY_SUFFIX: char = 'e';

const ZIP: &str = ".zip";
const ZOMBIE: &str = "_zombie";
const ZOMBIE_BURROW: usize = 0;
const ZOMBIE_SURFACE: usize = 2;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Aim {
    Blank,
    Zip(String),
    Cat { id: u32, form: usize, name: String },
    Enemy { id: u32, name: String },
}

impl Aim {
    pub fn stem(&self) -> Option<String> {
        match self {
            Aim::Cat { id, form, .. } => {
                Some(format!("{:03}_{}", id, FORMS.get(*form).copied().unwrap_or('f')))
            }
            Aim::Enemy { id, .. } => Some(format!("{:03}_{}", id, ENEMY_SUFFIX)),
            Aim::Blank | Aim::Zip(_) => None,
        }
    }

    pub fn unit(&self) -> Option<i32> {
        match self {
            Aim::Cat { id, .. } | Aim::Enemy { id, .. } => i32::try_from(*id).ok(),
            Aim::Blank | Aim::Zip(_) => None,
        }
    }

    pub fn parted(&self) -> Option<(String, String)> {
        let titled = self.titled()?;

        match self {
            Aim::Cat { id, form, .. } => Some((titled, format!("{:03}-{}", id, form + 1))),
            Aim::Enemy { id, .. } => Some((titled, format!("{:03}-E", id))),
            Aim::Blank | Aim::Zip(_) => None,
        }
    }

    pub fn caption(&self, set: &str) -> String {
        match self.parted() {
            Some((titled, id)) => format!("{} ({})", titled, id),
            None => match self {
                Aim::Zip(name) => format!("{}{}", name, ZIP),
                _ => format!("{}{}", set, ZIP),
            },
        }
    }

    fn titled(&self) -> Option<String> {
        let held = match self {
            Aim::Cat { name, .. } | Aim::Enemy { name, .. } => name.trim(),
            Aim::Blank | Aim::Zip(_) => return None,
        };

        match held.is_empty() {
            true => self.stem(),
            false => Some(held.to_owned()),
        }
    }
}

pub trait Roster {
    fn cat(&self, id: u32, form: usize) -> Option<String>;
    fn cat_named(&self, name: &str) -> Option<(u32, usize)>;
    fn enemy(&self, id: u32) -> Option<String>;
    fn enemy_named(&self, name: &str) -> Option<u32>;
}

pub fn resolve(typed: &str, roster: &impl Roster) -> Aim {
    let typed = typed.trim();

    if typed.is_empty() {
        return Aim::Blank;
    }

    if let Some(stem) = strip_zip(typed) {
        return Aim::Zip(stem.to_owned());
    }

    if let Some(aim) = by_id(typed, roster) {
        return aim;
    }

    if let Some((id, form)) = roster.cat_named(typed) {
        return Aim::Cat { id, form, name: typed.to_owned() };
    }

    if let Some(id) = roster.enemy_named(typed) {
        return Aim::Enemy { id, name: typed.to_owned() };
    }

    Aim::Zip(typed.to_owned())
}

fn strip_zip(typed: &str) -> Option<&str> {
    let cut = typed.len().checked_sub(ZIP.len())?;

    typed[cut..].eq_ignore_ascii_case(ZIP).then(|| typed[..cut].trim_end())
}

fn by_id(typed: &str, roster: &impl Roster) -> Option<Aim> {
    let filed = typed.split_once('_');
    let scored = typed.split_once('-');

    let (head, tail, filed) = match (filed, scored) {
        (Some((head, tail)), None) => (head, tail, true),
        (None, Some((head, tail))) => (head, tail, false),
        _ => return None,
    };

    let id = head.parse::<u32>().ok().filter(|_| !head.is_empty())?;

    if tail.eq_ignore_ascii_case(&ENEMY_SUFFIX.to_string()) {
        let enemy = tail.chars().next()?;

        if (filed && enemy == ENEMY_SUFFIX) || (!filed && enemy == ENEMY_SUFFIX.to_ascii_uppercase()) {
            return roster.enemy(id).map(|name| Aim::Enemy { id, name });
        }

        return None;
    }

    let form = match filed {
        true => FORMS.iter().position(|known| tail.len() == 1 && tail.starts_with(*known))?,
        false => tail.parse::<usize>().ok().filter(|slot| (1..=FORMS.len()).contains(slot))? - 1,
    };

    roster.cat(id, form).map(|name| Aim::Cat { id, form, name })
}

pub(super) struct Plan {
    pub(super) seats: Vec<(PathBuf, String)>,
    pub(super) stray: usize,
}

pub(super) fn plan(set: &Set, stem: &str) -> Plan {
    let was = set.model.as_deref().map_or_else(String::new, super::stem_of);
    let mut seats: Vec<(PathBuf, String)> = Slot::ALL
        .into_iter()
        .filter_map(|slot| set.slot(slot).map(|path| (path, slot)))
        .map(|(path, slot)| (path.to_path_buf(), format!("{}.{}", stem, slot.extension())))
        .collect();

    let mut taken: Vec<String> = Vec::new();
    let mut stray = 0;

    for anim in &set.anims {
        let Some(held) = anim.file_stem().and_then(OsStr::to_str) else {
            continue;
        };

        let seat = match tagged(held, &was).filter(|tag| !taken.contains(tag)) {
            Some(tag) => {
                let seat = format!("{}{}.{}", stem, tag, super::ANIM_EXT);

                taken.push(tag);
                seat
            }
            None => {
                stray += 1;
                format!("{}.{}", held, super::ANIM_EXT)
            }
        };

        seats.push((anim.to_path_buf(), seat));
    }

    Plan { seats, stray }
}

fn tagged(held: &str, was: &str) -> Option<String> {
    if !was.is_empty()
        && let Some(rest) = held.strip_prefix(was)
    {
        return Some(rest.to_owned());
    }

    let lowered = held.to_lowercase();
    let word = lowered.trim_end_matches(|glyph: char| glyph.is_ascii_digit());

    match word {
        "burrow" | "dig" => Some(format!("{}{:02}", ZOMBIE, ZOMBIE_BURROW)),
        "surface" => Some(format!("{}{:02}", ZOMBIE, ZOMBIE_SURFACE)),
        _ => role(word).and_then(ordinal).map(|at| format!("{:02}", at)),
    }
}

fn role(word: &str) -> Option<Role> {
    match word {
        "walk" | "move" => Some(Role::Walk),
        "idle" | "wait" => Some(Role::Idle),
        "atk" | "attack" => Some(Role::Attack),
        "kb" | "knockback" => Some(Role::Knockback),
        _ => None,
    }
}

fn ordinal(role: Role) -> Option<usize> {
    (0..)
        .take_while(|at| animation::standard(*at).is_some())
        .find(|at| animation::standard(*at).is_some_and(|(_, _, held)| held == role))
}

pub(super) fn restamped(bytes: &[u8], unit: i32) -> Option<Vec<u8>> {
    let mut doc = Mamodel::parse(bytes).ok()?;

    (doc.restamp(unit) > 0).then(|| doc.write())
}

pub(super) fn repointed(bytes: &[u8], to: &str) -> Vec<u8> {
    let named = format!("{}.{}", to, super::SHEET_EXT);

    Imgcut::parse(bytes)
        .ok()
        .and_then(|mut doc| doc.set_sheet(&named).then(|| doc.write()))
        .unwrap_or_else(|| bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig(stem: &str, anims: &[&str]) -> Set {
        Set {
            name: stem.to_owned(),
            sheet: Some(PathBuf::from(format!("studio/a/{stem}.png"))),
            cuts: Some(PathBuf::from(format!("studio/a/{stem}.imgcut"))),
            model: Some(PathBuf::from(format!("studio/a/{stem}.mamodel"))),
            anims: anims.iter().map(|held| PathBuf::from(format!("studio/a/{held}.maanim"))).collect(),
        }
    }

    struct Fake;

    impl Roster for Fake {
        fn cat(&self, id: u32, form: usize) -> Option<String> {
            match id {
                0 => (form < 3).then(|| ["Cat", "Tank Cat", "Axe Cat"][form].to_owned()),
                1 => (form == 3).then(String::new),
                _ => None,
            }
        }

        fn cat_named(&self, name: &str) -> Option<(u32, usize)> {
            match name {
                "Cat" => Some((0, 0)),
                "Tank Cat" => Some((0, 1)),
                _ => None,
            }
        }

        fn enemy(&self, id: u32) -> Option<String> {
            match id {
                2 => Some("Snache".to_owned()),
                297 => Some(String::new()),
                _ => None,
            }
        }

        fn enemy_named(&self, name: &str) -> Option<u32> {
            (name == "Snache").then_some(2)
        }
    }

    #[test]
    fn both_id_spellings_resolve_and_neither_mixes() {
        assert_eq!(resolve("000_f", &Fake), Aim::Cat { id: 0, form: 0, name: "Cat".to_owned() });
        assert_eq!(resolve("000-1", &Fake), Aim::Cat { id: 0, form: 0, name: "Cat".to_owned() });
        assert_eq!(resolve("000-2", &Fake), Aim::Cat { id: 0, form: 1, name: "Tank Cat".to_owned() });
        assert_eq!(resolve("002_e", &Fake), Aim::Enemy { id: 2, name: "Snache".to_owned() });
        assert_eq!(resolve("002-E", &Fake), Aim::Enemy { id: 2, name: "Snache".to_owned() });

        // Crossed spellings are not ids, so they fall through to a plain file name.
        assert_eq!(resolve("000_1", &Fake), Aim::Zip("000_1".to_owned()));
        assert_eq!(resolve("000-f", &Fake), Aim::Zip("000-f".to_owned()));
        assert_eq!(resolve("002-e", &Fake), Aim::Zip("002-e".to_owned()));
        assert_eq!(resolve("002_E", &Fake), Aim::Zip("002_E".to_owned()));
    }

    #[test]
    fn an_exact_name_wins_and_anything_else_is_a_file() {
        assert_eq!(resolve("Snache", &Fake), Aim::Enemy { id: 2, name: "Snache".to_owned() });
        assert_eq!(resolve("Tank Cat", &Fake), Aim::Cat { id: 0, form: 1, name: "Tank Cat".to_owned() });
        assert_eq!(resolve("tank cat", &Fake), Aim::Zip("tank cat".to_owned()), "matching is exact");
        assert_eq!(resolve("my rig", &Fake), Aim::Zip("my rig".to_owned()));
        assert_eq!(resolve("   ", &Fake), Aim::Blank);
    }

    #[test]
    fn a_typed_extension_overrules_every_entity_rule() {
        assert_eq!(resolve("Snache.zip", &Fake), Aim::Zip("Snache".to_owned()));
        assert_eq!(resolve("000_f.zip", &Fake), Aim::Zip("000_f".to_owned()));
        assert_eq!(resolve("000_f.ZIP", &Fake), Aim::Zip("000_f".to_owned()));

        // and the caption must not double it
        assert_eq!(resolve("Snache.zip", &Fake).caption("set"), "Snache.zip");
    }

    #[test]
    fn the_caption_names_what_will_actually_be_written() {
        assert_eq!(resolve("", &Fake).caption("MyRig"), "MyRig.zip");
        assert_eq!(resolve("000_f", &Fake).caption("MyRig"), "Cat (000-1)");
        assert_eq!(resolve("002_e", &Fake).caption("MyRig"), "Snache (002-E)");
        assert_eq!(resolve("loose", &Fake).caption("MyRig"), "loose.zip");
    }

    #[test]
    fn an_entity_aim_carries_the_stem_and_unit_the_files_need() {
        let cat = resolve("000-3", &Fake);

        assert_eq!(cat.stem().as_deref(), Some("000_s"));
        assert_eq!(cat.unit(), Some(0));

        let enemy = resolve("002-E", &Fake);

        assert_eq!(enemy.stem().as_deref(), Some("002_e"));
        assert_eq!(enemy.unit(), Some(2));
        assert_eq!(resolve("loose", &Fake).stem(), None);
    }

    #[test]
    fn a_nameless_entity_still_resolves_and_wears_its_stem() {
        // Plenty of enemies ship with an id and no name; they are still a valid target.
        let enemy = resolve("297_e", &Fake);

        assert_eq!(enemy, Aim::Enemy { id: 297, name: String::new() });
        assert_eq!(enemy.parted(), Some(("297_e".to_owned(), "297-E".to_owned())));
        assert_eq!(enemy.caption("MyRig"), "297_e (297-E)");
        assert_eq!(resolve("297-E", &Fake), enemy);

        let cat = resolve("001-4", &Fake);

        assert_eq!(cat.parted(), Some(("001_u".to_owned(), "001-4".to_owned())));
        assert_eq!(cat.stem().as_deref(), Some("001_u"));
    }

    #[test]
    fn a_track_carrying_the_source_stem_keeps_its_ordinal() {
        let set = rig("034_s", &["034_s00", "034_s02", "034_s_zombie00"]);
        let plan = plan(&set, "297_e");
        let seats: Vec<&str> = plan.seats.iter().map(|(_, seat)| seat.as_str()).collect();

        assert_eq!(
            seats,
            vec![
                "297_e.png",
                "297_e.imgcut",
                "297_e.mamodel",
                "297_e00.maanim",
                "297_e02.maanim",
                "297_e_zombie00.maanim",
            ]
        );
        assert_eq!(plan.stray, 0);
    }

    #[test]
    fn a_track_named_for_its_role_lands_on_that_ordinal() {
        // A modder's own names are the whole reason install stopped renaming by prefix.
        let set = rig("MyRig", &["Attack", "knockback", "move", "WAIT00"]);
        let plan = plan(&set, "297_e");
        let seats: Vec<&str> = plan.seats.iter().map(|(_, seat)| seat.as_str()).collect();

        assert_eq!(
            seats,
            vec![
                "297_e.png",
                "297_e.imgcut",
                "297_e.mamodel",
                "297_e02.maanim",
                "297_e03.maanim",
                "297_e00.maanim",
                "297_e01.maanim",
            ]
        );
        assert_eq!(plan.stray, 0);
    }

    #[test]
    fn a_zombie_track_lands_on_the_ordinal_the_corpus_uses() {
        // Measured over all 20 shipped zombies: 00 goes under, 01 is the buried hold, 02 comes up.
        let set = rig("MyRig", &["dig", "surface", "burrow"]);
        let seats: Vec<String> = plan(&set, "297_e").seats.into_iter().map(|(_, seat)| seat).collect();

        assert_eq!(seats[3], "297_e_zombie00.maanim");
        assert_eq!(seats[4], "297_e_zombie02.maanim");
        assert_eq!(seats[5], "burrow.maanim", "dig already claimed the burrow slot");
        assert_eq!(plan(&set, "297_e").stray, 1);
    }

    #[test]
    fn every_file_lands_and_the_ones_we_cannot_place_are_counted() {
        // Eight tracks onto an entity that ships three: all eight are still written.
        let set = rig("034_s", &["034_s00", "034_s01", "034_s02", "atk", "spin", "extra", "cheer", "kb"]);
        let plan = plan(&set, "297_e");

        assert_eq!(plan.seats.len(), set.files().len(), "nothing is dropped");
        assert_eq!(plan.stray, 4, "atk collides with 02, and spin/extra/cheer say nothing");
        assert_eq!(plan.seats.last().map(|(_, seat)| seat.as_str()), Some("297_e03.maanim"), "kb still lands");

        let stranded: Vec<&str> = plan
            .seats
            .iter()
            .map(|(_, seat)| seat.as_str())
            .filter(|seat| !seat.starts_with("297_e"))
            .collect();

        assert_eq!(stranded, vec!["atk.maanim", "spin.maanim", "extra.maanim", "cheer.maanim"]);
    }

    #[test]
    fn the_sheet_and_cut_list_are_renamed_whatever_they_were_called() {
        let mut set = rig("MyRig", &[]);

        set.sheet = Some(PathBuf::from("studio/a/atlas final (2).png"));
        set.cuts = Some(PathBuf::from("studio/a/cuts.imgcut"));

        let seats: Vec<String> = plan(&set, "000_f").seats.into_iter().map(|(_, seat)| seat).collect();

        assert_eq!(seats, vec!["000_f.png", "000_f.imgcut", "000_f.mamodel"]);
    }

    #[test]
    fn the_imgcut_texture_line_follows_the_rename() {
        let source = "[imgcut]\n0\n034_s.png\n1\n1,1,2,2,a\n".as_bytes();
        let moved = repointed(source, "000_f");

        assert_eq!(String::from_utf8(moved).expect("text"), "[imgcut]\n0\n000_f.png\n1\n1,1,2,2,a\n");
    }

    #[test]
    fn repointing_keeps_a_byte_order_mark_and_leaves_an_unreadable_list_alone() {
        let body = b"[imgcut]\n0\nwhatever.png\n1\n1,1,2,2,a\n".to_vec();
        let source: Vec<u8> = [0xef, 0xbb, 0xbf].iter().copied().chain(body).collect();
        let moved = repointed(&source, "000_f");

        assert!(moved.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(String::from_utf8_lossy(&moved).contains("000_f.png"));

        let torn = b"[imgcut]\n0\n034_s.png\n".to_vec();

        assert_eq!(repointed(&torn, "000_f"), torn, "a list we cannot parse is passed through");
    }
}
