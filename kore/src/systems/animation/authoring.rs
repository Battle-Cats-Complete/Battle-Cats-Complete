use std::sync::Arc;

use nyanko::common::{scrub, Separator};
use nyanko::graphics::rig::{AnimModification, Animation, Keyframe, ModelPart, RigError};

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
const NAME_FIELD: usize = 5;

#[derive(Clone)]
pub struct Maanim {
    bom: bool,
    tag: Option<String>,
    names: Vec<String>,
    animation: Arc<Animation>,
}

impl Maanim {
    pub fn parse(bytes: &[u8]) -> Result<Self, RigError> {
        let animation = Animation::parse(bytes)?;
        let body = scrub(bytes);
        let lines: Vec<&str> = body.lines().filter(|line| !line.trim().is_empty()).collect();
        let tag = lines
            .first()
            .filter(|line| line.trim_start().starts_with('['))
            .map(|line| (*line).to_owned());

        let names = names(&lines, &body, animation.modifications.len());

        Ok(Self { bom: bytes.starts_with(&BOM), tag, names, animation: Arc::new(animation) })
    }

    pub fn shared(&self) -> Arc<Animation> {
        Arc::clone(&self.animation)
    }

    pub fn tracks(&self) -> &[AnimModification] {
        &self.animation.modifications
    }

    pub fn track(&self, at: usize) -> Option<&AnimModification> {
        self.animation.modifications.get(at)
    }

    pub fn edit(&mut self, at: usize) -> Option<&mut AnimModification> {
        Arc::make_mut(&mut self.animation).modifications.get_mut(at)
    }

    pub fn insert(&mut self, at: usize, track: AnimModification) {
        let at = at.min(self.animation.modifications.len());

        self.names.insert(at, track.name.clone());
        Arc::make_mut(&mut self.animation).modifications.insert(at, track);
    }

    pub fn remove(&mut self, at: usize) -> Option<AnimModification> {
        if at >= self.animation.modifications.len() {
            return None;
        }

        self.names.remove(at);

        Some(Arc::make_mut(&mut self.animation).modifications.remove(at))
    }

    pub fn add_key(&mut self, track: usize) -> Option<usize> {
        let last = self.track(track)?.keyframes.last().copied();
        let key = last.map_or(Keyframe::default(), |key| Keyframe { frame: key.frame + 1, ..key });
        let keyframes = &mut Arc::make_mut(&mut self.animation).modifications.get_mut(track)?.keyframes;

        let at = keyframes.partition_point(|existing| existing.frame <= key.frame);
        keyframes.insert(at, key);

        Some(at)
    }

    pub fn remove_key(&mut self, track: usize, at: usize) -> bool {
        let Some(keyframes) = Arc::make_mut(&mut self.animation)
            .modifications
            .get_mut(track)
            .map(|track| &mut track.keyframes)
        else {
            return false;
        };

        if at >= keyframes.len() {
            return false;
        }

        keyframes.remove(at);

        true
    }

    pub fn sort_keys(&mut self, track: usize) {
        if let Some(track) = Arc::make_mut(&mut self.animation).modifications.get_mut(track) {
            track.keyframes.sort_by_key(|key| key.frame);
        }
    }

    fn name(&self, at: usize) -> &str {
        let parsed = self.animation.modifications.get(at).map_or("", |track| track.name.as_str());

        self.names
            .get(at)
            .filter(|raw| raw.trim() == parsed)
            .map_or(parsed, String::as_str)
    }

    pub fn write(&self) -> Vec<u8> {
        let delimiter = Separator::Comma.char();
        let mut body = String::new();

        if let Some(tag) = &self.tag {
            body.push_str(tag);
            body.push('\n');
        }

        push_line(&mut body, &self.animation.version.to_string());
        push_line(&mut body, &self.animation.modifications.len().to_string());

        for (at, track) in self.animation.modifications.iter().enumerate() {
            for value in [track.part, track.kind, track.loop_count, track.min_value, track.max_value] {
                body.push_str(&value.to_string());
                body.push(delimiter);
            }

            push_line(&mut body, self.name(at));
            push_line(&mut body, &track.keyframes.len().to_string());

            for key in &track.keyframes {
                let row = [key.frame, key.value, key.ease, key.ease_power];
                let fields: Vec<String> = row.iter().map(i32::to_string).collect();

                push_line(&mut body, &fields.join(&delimiter.to_string()));
            }
        }

        let mut bytes = Vec::with_capacity(body.len() + BOM.len());

        if self.bom {
            bytes.extend_from_slice(&BOM);
        }

        bytes.extend_from_slice(body.as_bytes());
        bytes
    }
}

pub const KINDS: [i32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

pub fn rest_value(part: &ModelPart, kind: i32) -> i32 {
    match kind {
        0 => part.parent,
        1 => part.id,
        2 => part.sprite,
        3 => part.z,
        4 => part.x,
        5 => part.y,
        6 => part.pivot_x,
        7 => part.pivot_y,
        8 | 9 => part.scale_x,
        10 => part.scale_y,
        11 => part.angle,
        12 => part.opacity,
        _ => 0,
    }
}

pub fn resting_curve(part: usize, kind: i32, declared: Option<&ModelPart>) -> AnimModification {
    let value = declared.map_or(0, |declared| rest_value(declared, kind));

    AnimModification {
        part: i32::try_from(part).unwrap_or(0),
        kind,
        loop_count: 1,
        min_value: 0,
        max_value: 0,
        name: String::new(),
        keyframes: vec![Keyframe { frame: 0, value, ease: 0, ease_power: 0 }],
    }
}

pub fn kind_label(kind: i32) -> &'static str {
    match kind {
        0 => "Parent",
        1 => "Unit ID",
        2 => "Sprite",
        3 => "Z Order",
        4 => "X",
        5 => "Y",
        6 => "Pivot X",
        7 => "Pivot Y",
        8 => "Scale",
        9 => "Scale X",
        10 => "Scale Y",
        11 => "Angle",
        12 => "Opacity",
        13 => "Flip X",
        14 => "Flip Y",
        _ => "Unknown",
    }
}

pub const EASES: [&str; 4] = ["Linear", "Hold", "Exponential", "Polynomial"];

const EASE_EXPONENTIAL: i32 = 2;

pub fn ease_label(ease: i32) -> &'static str {
    usize::try_from(ease).ok().and_then(|at| EASES.get(at)).copied().unwrap_or("Unknown")
}

pub fn ease_takes_power(ease: i32) -> bool {
    ease == EASE_EXPONENTIAL
}

pub fn ease_value(label: &str) -> Option<i32> {
    EASES.iter().position(|known| *known == label).and_then(|at| i32::try_from(at).ok())
}

pub fn key_label(keys: usize) -> String {
    match keys {
        1 => "1 key".to_string(),
        other => format!("{} keys", other),
    }
}

pub fn loop_label(count: i32) -> String {
    match count {
        -1 => "Forever".to_string(),
        held if held <= 1 => "Once".to_string(),
        _ => "Count".to_string(),
    }
}

fn push_line(body: &mut String, text: &str) {
    body.push_str(text);
    body.push('\n');
}

fn names(lines: &[&str], body: &str, tracks: usize) -> Vec<String> {
    let delimiter = Separator::detect(body).unwrap_or(Separator::Comma).char();
    let mut cursor = usize::from(lines.first().is_some_and(|line| line.trim_start().starts_with('[')));
    cursor += 2;

    let mut found = Vec::with_capacity(tracks);

    for _ in 0..tracks {
        let Some(header) = lines.get(cursor) else { break };
        cursor += 1;

        let name = header.splitn(NAME_FIELD + 1, delimiter).nth(NAME_FIELD).unwrap_or_default();
        found.push(name.to_owned());

        let Some(count) = lines.get(cursor) else { break };
        cursor += 1;

        let declared = count
            .split(delimiter)
            .next()
            .and_then(|text| text.trim().parse::<usize>().ok())
            .unwrap_or(0);

        cursor += declared.min(lines.len().saturating_sub(cursor));
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const PADDED: &str = "[modelanim:animation]\n1\n2\n20,11,-1,0,0,\tネスト\t\n2\n-30,50,2,2\n10,50,0,0\n21,4,1,0,0,\n1\n0,0,0,0\n";

    fn round_trip(source: &[u8]) -> Vec<u8> {
        let doc = Maanim::parse(source).expect("the sample parses");

        doc.write()
    }

    #[test]
    fn a_padded_name_survives_a_round_trip_byte_for_byte() {
        // nyanko trims the name on parse, so the raw field is what has to be written back.
        assert_eq!(round_trip(PADDED.as_bytes()), PADDED.as_bytes());
    }

    #[test]
    fn the_byte_order_mark_is_kept_only_where_the_file_had_one() {
        let with_bom: Vec<u8> = BOM.iter().copied().chain(PADDED.bytes()).collect();

        assert_eq!(round_trip(&with_bom), with_bom);
        assert_eq!(round_trip(PADDED.as_bytes()), PADDED.as_bytes());
    }

    #[test]
    fn the_tag_line_is_copied_rather_than_spelled() {
        // Both tags ship in the real data; writing one for the other would relabel the file.
        let second = PADDED.replace("animation]", "animation2]");

        assert_eq!(round_trip(second.as_bytes()), second.as_bytes());
    }

    #[test]
    fn editing_a_name_drops_the_padding_it_no_longer_matches() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        doc.edit(0).expect("the track exists").name = "walk".to_string();

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("20,11,-1,0,0,walk\n"));
    }

    #[test]
    fn a_keyframe_edit_rewrites_only_its_own_row() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        doc.edit(0).expect("the track exists").keyframes[1].value = 75;

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("-30,50,2,2\n10,75,0,0\n"));
        assert!(written.contains("\tネスト\t\n"));
    }

    #[test]
    fn inserting_and_removing_tracks_keeps_the_names_aligned() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let added = AnimModification {
            part: 3,
            kind: 4,
            loop_count: 1,
            keyframes: vec![Keyframe { frame: 0, value: 9, ease: 0, ease_power: 0 }],
            name: "added".to_string(),
            ..AnimModification::default()
        };

        doc.insert(0, added);
        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("\n3\n3,4,1,0,0,added\n"));
        assert!(written.contains("\tネスト\t\n"));

        doc.remove(0);
        assert_eq!(doc.write(), PADDED.as_bytes());
    }

    #[test]
    fn a_new_keyframe_lands_in_frame_order_and_carries_the_last_value() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let at = doc.add_key(0).expect("the track exists");

        assert_eq!(at, 2);
        assert_eq!(doc.track(0).expect("the track exists").keyframes[2], Keyframe { frame: 11, value: 50, ease: 0, ease_power: 0 });

        assert!(doc.remove_key(0, 2));
        assert_eq!(doc.write(), PADDED.as_bytes());
    }

    #[test]
    fn sharing_the_animation_does_not_block_a_later_edit() {
        // The viewer holds an Arc every frame; editing must still land.
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let held = doc.shared();

        doc.edit(0).expect("the track exists").keyframes[0].value = 900;

        assert_eq!(held.modifications[0].keyframes[0].value, 50);
        assert_eq!(doc.shared().modifications[0].keyframes[0].value, 900);
    }

    #[test]
    fn every_replay_count_reads_as_a_word_or_a_multiplier() {
        // The engine wraps only on -1 and on counts above one; everything else
        // rests on the final keyframe, so it plays through exactly once.
        assert_eq!(loop_label(-1), "Forever");
        assert_eq!(loop_label(1), "Once");
        assert_eq!(loop_label(0), "Once");
        assert_eq!(loop_label(-2), "Once");
        assert_eq!(loop_label(4), "Count");
    }

    #[test]
    fn a_new_curve_starts_where_the_part_already_rests() {
        // Seeding a Scale curve at zero would collapse the part the moment it is added.
        let part = ModelPart { scale_x: 1000, opacity: 800, angle: 45, ..ModelPart::default() };

        assert_eq!(resting_curve(3, 9, Some(&part)).keyframes[0].value, 1000);
        assert_eq!(resting_curve(3, 12, Some(&part)).keyframes[0].value, 800);
        assert_eq!(resting_curve(3, 11, Some(&part)).keyframes[0].value, 45);
        assert_eq!(resting_curve(3, 13, Some(&part)).keyframes[0].value, 0);
    }

    #[test]
    fn a_new_curve_names_its_part_and_plays_once() {
        let track = resting_curve(7, 4, None);

        assert_eq!((track.part, track.kind, track.loop_count), (7, 4, 1));
        assert_eq!(track.keyframes.len(), 1);
    }

    #[test]
    fn a_single_keyframe_is_not_plural() {
        assert_eq!(key_label(0), "0 keys");
        assert_eq!(key_label(1), "1 key");
        assert_eq!(key_label(9), "9 keys");
    }

    #[test]
    fn only_the_exponential_ease_reads_a_power() {
        // Linear, Hold and Polynomial never touch ease_power in the engine.
        assert!(ease_takes_power(2));
        assert!(!ease_takes_power(0));
        assert!(!ease_takes_power(1));
        assert!(!ease_takes_power(3));
        assert_eq!(EASES[2], "Exponential");
    }
}
