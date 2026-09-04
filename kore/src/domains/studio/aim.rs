use crate::systems::animation::authoring::Mamodel;

pub(super) const FORMS: [char; 4] = ['f', 'c', 's', 'u'];
pub(super) const ENEMY_SUFFIX: char = 'e';

const ZIP: &str = ".zip";

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
        match self {
            Aim::Cat { id, form, name } => Some((name.clone(), format!("{:03}-{}", id, form + 1))),
            Aim::Enemy { id, name } => Some((name.clone(), format!("{:03}-E", id))),
            Aim::Blank | Aim::Zip(_) => None,
        }
    }

    pub fn caption(&self, set: &str) -> String {
        match self {
            Aim::Blank => format!("{}{}", set, ZIP),
            Aim::Zip(name) => format!("{}{}", name, ZIP),
            Aim::Cat { id, form, name } => format!("{} ({:03}-{})", name, id, form + 1),
            Aim::Enemy { id, name } => format!("{} ({:03}-E)", name, id),
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

pub(super) fn renamed(file: &str, from: &str, to: &str) -> String {
    match file.strip_prefix(from) {
        Some(rest) => format!("{to}{rest}"),
        None => file.to_owned(),
    }
}

pub(super) fn restamped(bytes: &[u8], unit: i32) -> Option<Vec<u8>> {
    let mut doc = Mamodel::parse(bytes).ok()?;

    (doc.restamp(unit) > 0).then(|| doc.write())
}

pub(super) fn repointed(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    let (head, rest) = match bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        true => bytes.split_at(3),
        false => bytes.split_at(0),
    };

    let Ok(text) = std::str::from_utf8(rest) else {
        return bytes.to_vec();
    };

    let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
    let named = format!("{to}.png");
    let mut swapped = None;

    for line in lines.iter_mut() {
        if !line.trim_end().eq_ignore_ascii_case(&format!("{from}.png")) {
            continue;
        }

        swapped = Some(match line.strip_suffix("\r\n") {
            Some(_) => format!("{named}\r\n"),
            None => match line.strip_suffix('\n') {
                Some(_) => format!("{named}\n"),
                None => named.clone(),
            },
        });

        break;
    }

    let Some(swapped) = swapped else {
        return bytes.to_vec();
    };

    let rebuilt: String = lines
        .iter()
        .map(|line| match line.trim_end().eq_ignore_ascii_case(&format!("{from}.png")) {
            true => swapped.as_str(),
            false => line,
        })
        .collect();

    head.iter().copied().chain(rebuilt.into_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Roster for Fake {
        fn cat(&self, id: u32, form: usize) -> Option<String> {
            (id == 0 && form < 3).then(|| ["Cat", "Tank Cat", "Axe Cat"][form].to_owned())
        }

        fn cat_named(&self, name: &str) -> Option<(u32, usize)> {
            match name {
                "Cat" => Some((0, 0)),
                "Tank Cat" => Some((0, 1)),
                _ => None,
            }
        }

        fn enemy(&self, id: u32) -> Option<String> {
            (id == 2).then(|| "Snache".to_owned())
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
    fn renaming_swaps_only_the_stem() {
        assert_eq!(renamed("034_s02.maanim", "034_s", "000_f"), "000_f02.maanim");
        assert_eq!(renamed("034_s.imgcut", "034_s", "000_f"), "000_f.imgcut");
        assert_eq!(renamed("readme.txt", "034_s", "000_f"), "readme.txt");
    }

    #[test]
    fn the_imgcut_texture_line_follows_the_rename() {
        let source = "[imgcut]\n0\n034_s.png\n1\n1,1,2,2,a\n".as_bytes();
        let moved = repointed(source, "034_s", "000_f");

        assert_eq!(String::from_utf8(moved).expect("text"), "[imgcut]\n0\n000_f.png\n1\n1,1,2,2,a\n");
    }

    #[test]
    fn repointing_keeps_a_byte_order_mark_and_leaves_a_stranger_alone() {
        let source: Vec<u8> = [0xef, 0xbb, 0xbf].iter().copied().chain(b"[imgcut]\n0\n034_s.png\n".to_vec()).collect();
        let moved = repointed(&source, "034_s", "000_f");

        assert!(moved.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(String::from_utf8_lossy(&moved).contains("000_f.png"));

        let other = b"[imgcut]\n0\nsomething.png\n".to_vec();

        assert_eq!(repointed(&other, "034_s", "000_f"), other);
    }
}
