use std::borrow::Cow;
use std::sync::Arc;

use nyanko::common::{scrub, Separator};
use nyanko::graphics::rig::{Model, ModelPart, RigError};

use super::hazards::SHEET_FIELD;

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
const PART_CELLS: usize = 13;
const GLOW_MODES: i32 = 3;
const NO_PARENT: i32 = -1;
const NOT_DRAWN: i32 = -1;

#[derive(Clone)]
struct Line {
    text: String,
    end: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Count,
    Parts,
    Units,
    AlignCount,
    Aligns,
    Skip,
}

struct Layout {
    roles: Vec<Option<Role>>,
    raws: Vec<String>,
    rows: Vec<String>,
    tails: [&'static str; 2],
}

#[derive(Clone)]
pub struct Mamodel {
    bom: bool,
    delimiter: char,
    lines: Vec<Line>,
    roles: Vec<Option<Role>>,
    raws: Vec<String>,
    rows: Vec<String>,
    tails: [&'static str; 2],
    parts: usize,
    aligns: usize,
    model: Arc<Model>,
}

impl Mamodel {
    pub fn parse(bytes: &[u8]) -> Result<Self, RigError> {
        let model = Model::parse(bytes)?;
        let delimiter = Separator::detect(&scrub(bytes)).unwrap_or(Separator::Comma).char();

        let bom = bytes.starts_with(&BOM);
        let body = String::from_utf8_lossy(if bom { &bytes[BOM.len()..] } else { bytes }).into_owned();

        let lines = split(&body);
        let Layout { roles, raws, rows, tails } = roles(&lines, &model);
        let (parts, aligns) = (model.parts.len(), model.alignment.len());

        Ok(Self { bom, delimiter, lines, roles, raws, rows, tails, parts, aligns, model: Arc::new(model) })
    }

    pub fn shared(&self) -> Arc<Model> {
        Arc::clone(&self.model)
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn count(&self) -> usize {
        self.model.parts.len()
    }

    pub fn field(&self, part: usize, at: usize) -> Option<i32> {
        self.model.parts.get(part).filter(|_| at < PART_CELLS).map(|part| field(part, at))
    }

    pub fn set_field(&mut self, part: usize, at: usize, value: i32) -> bool {
        if at >= PART_CELLS {
            return false;
        }

        let Some(held) = Arc::make_mut(&mut self.model).parts.get_mut(part) else {
            return false;
        };

        if field(held, at) == value {
            return false;
        }

        set_field(held, at, value);

        true
    }

    pub fn name(&self, part: usize) -> Option<&str> {
        self.model.parts.get(part).map(|part| part.name.as_str())
    }

    pub fn restamp(&mut self, unit: i32) -> usize {
        let borrowed: Vec<usize> = (0..self.count())
            .filter(|part| self.field(*part, SHEET_FIELD).is_some_and(|id| id >= 0 && id != unit))
            .collect();

        borrowed.iter().filter(|part| self.set_field(**part, SHEET_FIELD, unit)).count()
    }

    pub fn set_name(&mut self, part: usize, name: &str) -> bool {
        let Some(part) = Arc::make_mut(&mut self.model).parts.get_mut(part) else {
            return false;
        };

        if part.name == name {
            return false;
        }

        part.name = name.to_owned();

        true
    }

    pub fn parent(&self, part: usize) -> Option<usize> {
        self.model.parts.get(part).and_then(|part| usize::try_from(part.parent).ok())
    }

    pub fn offsets(&self) -> usize {
        self.model.alignment.len()
    }

    pub fn offset(&self, row: usize) -> Option<(i32, i32)> {
        self.model.alignment.get(row).map(|row| (row.x, row.y))
    }

    pub fn offset_name(&self, row: usize) -> Option<&str> {
        self.model.alignment.get(row).map(|row| row.name.as_str())
    }

    pub fn set_offset(&mut self, row: usize, axis: usize, value: i32) -> bool {
        let Some(row) = Arc::make_mut(&mut self.model).alignment.get_mut(row) else {
            return false;
        };

        let cell = if axis == 0 { &mut row.x } else { &mut row.y };

        if *cell == value {
            return false;
        }

        *cell = value;

        true
    }

    pub fn add_part(&mut self, parent: Option<usize>) -> usize {
        let seeded = blank_part(parent, &self.model);
        let model = Arc::make_mut(&mut self.model);

        model.parts.push(seeded);
        self.raws.push(String::new());

        model.parts.len() - 1
    }

    pub fn remove_part(&mut self, at: usize) -> Option<Vec<Option<usize>>> {
        let count = self.model.parts.len();

        if at >= count {
            return None;
        }

        let inherited = self.model.parts[at].parent;
        let moved: Vec<Option<usize>> =
            (0..count).map(|old| (old != at).then(|| old - usize::from(old > at))).collect();

        let model = Arc::make_mut(&mut self.model);

        for part in model.parts.iter_mut() {
            if i32::try_from(at) == Ok(part.parent) {
                part.parent = inherited;
            }
        }

        model.parts.remove(at);
        self.raws.remove(at);

        for part in model.parts.iter_mut() {
            part.parent = remap(&moved, part.parent);
        }

        Some(moved)
    }

    pub fn retarget_sprites(&mut self, moved: &[Option<usize>]) -> bool {
        let shifted = self.model.parts.iter().any(|part| {
            usize::try_from(part.sprite)
                .ok()
                .and_then(|at| moved.get(at))
                .is_some_and(|landed| landed.and_then(|at| i32::try_from(at).ok()) != Some(part.sprite))
        });

        if !shifted {
            return false;
        }

        for part in Arc::make_mut(&mut self.model).parts.iter_mut() {
            let Some(landed) = usize::try_from(part.sprite).ok().and_then(|at| moved.get(at)) else {
                continue;
            };

            part.sprite = landed.and_then(|at| i32::try_from(at).ok()).unwrap_or(NOT_DRAWN);
        }

        true
    }

    pub fn reparent(&mut self, at: usize, parent: Option<usize>) -> bool {
        let wanted = parent.and_then(|at| i32::try_from(at).ok()).unwrap_or(NO_PARENT);

        if self.model.parts.get(at).is_none_or(|part| part.parent == wanted) {
            return false;
        }

        if parent.is_some_and(|parent| self.descends(parent, at)) {
            return false;
        }

        if let Some(part) = Arc::make_mut(&mut self.model).parts.get_mut(at) {
            part.parent = wanted;
        }

        true
    }

    pub fn descends(&self, mut at: usize, of: usize) -> bool {
        for _ in 0..self.model.parts.len() {
            if at == of {
                return true;
            }

            let Some(parent) = self.parent(at).filter(|parent| *parent != at) else {
                return false;
            };

            at = parent;
        }

        false
    }

    pub fn write(&self) -> Vec<u8> {
        let mut body = String::with_capacity(self.lines.iter().map(|line| line.text.len() + 2).sum());

        for (at, line) in self.lines.iter().enumerate() {
            match self.roles.get(at).copied().flatten() {
                Some(Role::Skip) => continue,
                Some(Role::Parts) => self.push_parts(&mut body, line.end),
                Some(Role::Aligns) => self.push_aligns(&mut body, line.end),
                Some(role) => {
                    body.push_str(&self.render(role, &line.text));
                    body.push_str(line.end);
                }
                None => {
                    body.push_str(&line.text);
                    body.push_str(line.end);
                }
            }
        }

        let mut bytes = Vec::with_capacity(body.len() + BOM.len());

        if self.bom {
            bytes.extend_from_slice(&BOM);
        }

        bytes.extend_from_slice(body.as_bytes());
        bytes
    }

    fn push_parts(&self, body: &mut String, end: &str) {
        let last = self.model.parts.len().saturating_sub(1);
        let end = if end.is_empty() { "\n" } else { end };

        for (at, part) in self.model.parts.iter().enumerate() {
            let raw = self.raws.get(at).map_or("", String::as_str);
            let values: Vec<i32> = (0..PART_CELLS).map(|cell| field(part, cell)).collect();

            body.push_str(&cells(raw, &values, Some(part.name.as_str()), self.delimiter));
            body.push_str(if at == last { self.tails[0] } else { end });
        }
    }

    fn push_aligns(&self, body: &mut String, end: &str) {
        let last = self.model.alignment.len().saturating_sub(1);
        let end = if end.is_empty() { "\n" } else { end };

        for (at, row) in self.model.alignment.iter().enumerate() {
            let raw = self.rows.get(at).map_or("", String::as_str);
            let values = [row.unknown_0, row.unknown_1, row.x, row.y, row.unknown_4, row.unknown_5];

            body.push_str(&cells(raw, &values, Some(row.name.as_str()), self.delimiter));
            body.push_str(if at == last { self.tails[1] } else { end });
        }
    }

    fn render(&self, role: Role, raw: &str) -> String {
        match role {
            Role::Count => count_line(raw, self.parts, self.model.parts.len()),
            Role::AlignCount => count_line(raw, self.aligns, self.model.alignment.len()),
            Role::Units => self.units_row(raw),
            _ => raw.to_owned(),
        }
    }

    fn units_row(&self, raw: &str) -> String {
        let mut values = vec![self.model.scale_unit, self.model.angle_unit, self.model.opacity_unit];

        if let Some(extra) = self.model.unknown_3 {
            values.push(extra);
        }

        cells(raw, &values, None, self.delimiter)
    }
}

pub const FIELDS: [&str; 14] = [
    "Parent", "Sheet ID", "Sprite", "Z Order", "X", "Y", "Pivot X", "Pivot Y", "Scale X", "Scale Y",
    "Angle", "Opacity", "Glow", "Name",
];

pub const NAME_FIELD: usize = PART_CELLS;

pub fn bound(model: &Model, cuts: usize, at: usize, value: i32) -> i32 {
    let ceiling = |count: usize| i32::try_from(count).unwrap_or(i32::MAX).saturating_sub(1);

    match at {
        0 => value.clamp(NO_PARENT, ceiling(model.parts.len())),
        2 => value.clamp(NOT_DRAWN, ceiling(cuts).max(NOT_DRAWN)),
        11 => value.clamp(0, model.opacity_unit.max(0)),
        12 => value.clamp(0, GLOW_MODES),
        _ => value,
    }
}

pub fn nameable(text: &str) -> bool {
    !text.chars().any(|glyph| matches!(glyph, ',' | '|' | '\t' | '\n' | '\r'))
}

pub fn defaults(model: &Model) -> [i32; PART_CELLS] {
    let mut cells = [0; PART_CELLS];

    cells[0] = NO_PARENT;
    cells[1] = drawn_id(model);
    cells[8] = model.scale_unit;
    cells[9] = model.scale_unit;
    cells[11] = model.opacity_unit;

    cells
}

fn blank_part(parent: Option<usize>, model: &Model) -> ModelPart {
    let anchor = parent.and_then(|at| model.parts.get(at));
    let cells = defaults(model);
    let mut part = ModelPart::default();

    for (at, value) in cells.iter().enumerate() {
        set_field(&mut part, at, *value);
    }

    part.parent = parent.and_then(|at| i32::try_from(at).ok()).unwrap_or(NO_PARENT);
    part.z = anchor.map_or(cells[3], |anchor| anchor.z.saturating_add(1));

    part
}

fn drawn_id(model: &Model) -> i32 {
    model.parts.iter().map(|part| part.id).find(|id| *id != NOT_DRAWN).unwrap_or(NOT_DRAWN)
}

fn field(part: &ModelPart, at: usize) -> i32 {
    match at {
        0 => part.parent,
        1 => part.id,
        2 => part.sprite,
        3 => part.z,
        4 => part.x,
        5 => part.y,
        6 => part.pivot_x,
        7 => part.pivot_y,
        8 => part.scale_x,
        9 => part.scale_y,
        10 => part.angle,
        11 => part.opacity,
        _ => part.glow,
    }
}

fn set_field(part: &mut ModelPart, at: usize, value: i32) {
    let cell = match at {
        0 => &mut part.parent,
        1 => &mut part.id,
        2 => &mut part.sprite,
        3 => &mut part.z,
        4 => &mut part.x,
        5 => &mut part.y,
        6 => &mut part.pivot_x,
        7 => &mut part.pivot_y,
        8 => &mut part.scale_x,
        9 => &mut part.scale_y,
        10 => &mut part.angle,
        11 => &mut part.opacity,
        12 => &mut part.glow,
        _ => return,
    };

    *cell = value;
}

fn remap(moved: &[Option<usize>], parent: i32) -> i32 {
    let Ok(at) = usize::try_from(parent) else {
        return parent;
    };

    match moved.get(at) {
        Some(Some(landed)) => i32::try_from(*landed).unwrap_or(NO_PARENT),
        Some(None) => NO_PARENT,
        None => parent,
    }
}

fn split(body: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut rest = body;

    while !rest.is_empty() {
        let Some(at) = rest.find(['\n', '\r']) else {
            lines.push(Line { text: rest.to_owned(), end: "" });

            break;
        };

        let (text, tail) = rest.split_at(at);
        let (end, skip) = match (tail.starts_with('\r'), tail.starts_with("\r\n")) {
            (_, true) => ("\r\n", 2),
            (true, _) => ("\r", 1),
            _ => ("\n", 1),
        };

        lines.push(Line { text: text.to_owned(), end });
        rest = &tail[skip..];
    }

    lines
}

fn roles(lines: &[Line], model: &Model) -> Layout {
    let mut roles = vec![None; lines.len()];
    let live: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.text.trim().is_empty())
        .map(|(at, _)| at)
        .collect();

    let tagged = live.first().is_some_and(|at| lines[*at].text.trim_start().starts_with('['));
    let mut cursor = usize::from(tagged) + 1;

    if let Some(at) = live.get(cursor) {
        roles[*at] = Some(Role::Count);
    }

    cursor += 1;

    let (raws, held) = block(lines, &live, cursor, model.parts.len(), Role::Parts, &mut roles);
    cursor += model.parts.len();

    if let Some(at) = live.get(cursor) {
        roles[*at] = Some(Role::Units);
    }

    cursor += 1;

    if let Some(at) = live.get(cursor) {
        roles[*at] = Some(Role::AlignCount);
    }

    cursor += 1;

    let (rows, trailing) = block(lines, &live, cursor, model.alignment.len(), Role::Aligns, &mut roles);

    Layout { roles, raws, rows, tails: [held, trailing] }
}

fn block(
    lines: &[Line],
    live: &[usize],
    cursor: usize,
    count: usize,
    lead: Role,
    roles: &mut [Option<Role>],
) -> (Vec<String>, &'static str) {
    let mut raws = Vec::with_capacity(count);
    let mut tail = "\n";

    for at in 0..count {
        let Some(line) = live.get(cursor + at) else {
            raws.push(String::new());

            continue;
        };

        roles[*line] = Some(if at == 0 { lead } else { Role::Skip });
        tail = lines[*line].end;
        raws.push(lines[*line].text.clone());
    }

    (raws, tail)
}

fn cells(raw: &str, values: &[i32], name: Option<&str>, delimiter: char) -> String {
    let raw: Vec<&str> = match raw.is_empty() {
        true => Vec::new(),
        false => raw.split(delimiter).collect(),
    };

    let floor = match raw.is_empty() {
        true => values.len() + usize::from(name.is_some_and(|name| !name.is_empty())),
        false => raw.len(),
    };

    let mut count = floor;

    for (at, value) in values.iter().enumerate() {
        if read(raw.get(at).copied()) != *value {
            count = count.max(at + 1);
        }
    }

    if let Some(name) = name
        && clean(raw.get(values.len()).copied().unwrap_or_default()).trim() != name
    {
        count = count.max(values.len() + 1);
    }

    let written: Vec<String> = (0..count)
        .map(|at| match (values.get(at), name) {
            (Some(value), _) => keep(raw.get(at).copied(), *value),
            (None, Some(name)) if at == values.len() => raw
                .get(at)
                .filter(|text| clean(text).trim() == name)
                .map_or_else(|| name.to_owned(), |text| (*text).to_owned()),
            _ => raw.get(at).copied().unwrap_or_default().to_owned(),
        })
        .collect();

    written.join(&delimiter.to_string())
}

fn count_line(raw: &str, was: usize, now: usize) -> String {
    if was == now {
        return raw.to_owned();
    }

    let declared = clean(raw).trim().parse::<usize>().unwrap_or(was);

    declared.saturating_add(now).saturating_sub(was).to_string()
}

pub(super) fn clean(cell: &str) -> Cow<'_, str> {
    match cell.contains(['\0', '\u{feff}']) {
        true => Cow::Owned(cell.replace(['\0', '\u{feff}'], "")),
        false => Cow::Borrowed(cell),
    }
}

pub(super) fn read(cell: Option<&str>) -> i32 {
    cell.and_then(|text| clean(text).trim().parse().ok()).unwrap_or(0)
}

pub(super) fn keep(cell: Option<&str>, value: i32) -> String {
    cell.filter(|text| read(Some(text)) == value)
        .map_or_else(|| value.to_string(), |text| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[modelanim:model]\n1\n2\n-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n0,0,1,400,10,-20,5,5,1000,1000,0,1000,0,\t head \t\n1000,3600,1000\n1\n0,0,-30,120,1000,1000,align\n";

    fn round_trip(source: &str) -> String {
        let doc = Mamodel::parse(source.as_bytes()).expect("the sample parses");

        String::from_utf8(doc.write()).expect("the output is text")
    }

    #[test]
    fn an_untouched_model_writes_back_byte_for_byte() {
        assert_eq!(round_trip(SAMPLE), SAMPLE);
    }

    #[test]
    fn a_zero_padded_number_survives_because_the_raw_field_is_kept() {
        // Nine vanilla files pad their integers; rewriting them numerically would
        // change bytes the engine never asked us to touch.
        let padded = SAMPLE.replace("0,0,1,400,", "0,000,001,0400,");

        assert_eq!(round_trip(&padded), padded);
    }

    #[test]
    fn a_byte_order_mark_and_carriage_returns_are_both_kept() {
        let windows: String = SAMPLE.replace('\n', "\r\n");
        let with_bom: Vec<u8> = BOM.iter().copied().chain(windows.bytes()).collect();
        let doc = Mamodel::parse(&with_bom).expect("the sample parses");

        assert_eq!(doc.write(), with_bom);
    }

    #[test]
    fn a_blank_line_and_a_line_past_the_alignment_block_are_both_kept() {
        // One vanilla file carries each; the parser skips them, so the writer has to.
        let odd = format!("{}\n999,junk\n", SAMPLE.replace("1000,3600,1000\n", "\n1000,3600,1000\n"));

        assert_eq!(round_trip(&odd), odd);
    }

    #[test]
    fn editing_one_field_rewrites_only_that_field() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_field(1, 3, 500));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("0,0,1,500,10,-20,5,5,1000,1000,0,1000,0,\t head \t\n"));
        assert!(written.contains("-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n"));
    }

    #[test]
    fn renaming_a_part_drops_the_padding_it_no_longer_matches() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_name(1, "arm"));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.ends_with("0,1000,0,arm\n1000,3600,1000\n1\n0,0,-30,120,1000,1000,align\n"));
    }

    #[test]
    fn an_added_part_lands_after_the_block_and_updates_the_count() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert_eq!(doc.add_part(Some(0)), 2);

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("\n3\n-1,0,0,"), "{}", written);
        assert!(written.contains("\t head \t\n0,0,0,1,0,0,0,0,1000,1000,0,1000,0\n1000,3600,1000\n"), "{}", written);
    }

    #[test]
    fn moving_the_alignment_row_rewrites_only_its_own_columns() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_offset(0, 0, 44));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.ends_with("0,0,44,120,1000,1000,align\n"));
    }

    #[test]
    fn a_row_with_no_name_field_does_not_grow_one() {
        let nameless = SAMPLE.replace(",0,body\n", ",0\n");

        assert_eq!(round_trip(&nameless), nameless);
    }

    #[test]
    fn a_delimiter_can_never_enter_a_name() {
        // A bar would flip nyanko's reader to pipe mode and mis-parse the file.
        assert!(nameable("left arm"));
        assert!(!nameable("left,arm"));
        assert!(!nameable("left|arm"));
        assert!(!nameable("left\narm"));
    }

    #[test]
    fn a_new_part_inherits_the_unit_id_and_draws_in_front_of_its_parent() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        let added = doc.add_part(Some(1));

        assert_eq!(doc.field(added, 0), Some(1));
        assert_eq!(doc.field(added, 1), Some(0), "the unit id is taken from the first drawn part");
        assert_eq!(doc.field(added, 2), Some(0));
        assert_eq!(doc.field(added, 3), Some(401), "one layer in front of its parent's 400");
        assert_eq!(doc.field(added, 8), Some(1000));
        assert_eq!(doc.field(added, 11), Some(1000));
    }

    #[test]
    fn removing_a_part_hands_its_children_to_its_own_parent_and_renumbers() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        doc.add_part(Some(1));

        let moved = doc.remove_part(1).expect("the part exists");

        assert_eq!(moved, vec![Some(0), None, Some(1)]);
        assert_eq!(doc.count(), 2);
        assert_eq!(doc.field(1, 0), Some(0), "the orphan inherits the removed part's own parent");
    }

    #[test]
    fn a_part_cannot_be_reparented_under_its_own_descendant() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert!(!doc.reparent(0, Some(1)), "part 1 already hangs off part 0");
        assert!(!doc.reparent(0, Some(0)), "nor can a part hang off itself");
    }

    #[test]
    fn only_the_fields_the_engine_bounds_are_clamped() {
        let model = Model { parts: vec![ModelPart::default(); 4], opacity_unit: 1000, ..Model::default() };

        assert_eq!(bound(&model, 9, 0, 12), 3, "parent stops at the last part");
        assert_eq!(bound(&model, 9, 0, -7), -1, "and at the root sentinel");
        assert_eq!(bound(&model, 9, 2, 40), 8, "sprite stops at the last cut");
        assert_eq!(bound(&model, 9, 11, 4000), 1000, "opacity stops at its own unit");
        assert_eq!(bound(&model, 9, 12, 9), 3, "glow stops at the last blending mode");
        assert_eq!(bound(&model, 9, 3, i32::MAX), i32::MAX, "z order has no bound at all");
        assert_eq!(bound(&model, 9, 4, i32::MIN), i32::MIN, "and neither does an offset");
    }
}
