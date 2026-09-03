use super::*;
use iced::widget::{container, row, text};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Held {
    pub(super) part: i32,
    pub(super) kind: i32,
    pub(super) ordinal: usize,
}

pub(super) struct TreeRow {
    pub(super) label: String,
    pub(super) depth: u16,
    pub(super) mark: &'static str,
    pub(super) part: Option<usize>,
    pub(super) owner: Option<usize>,
    pub(super) track: Option<usize>,
    pub(super) warn: bool,
    pub(super) bucket: bool,
}

impl TreeRow {
    pub(super) fn span(&self) -> f32 {
        ROW_PADDING * 2.0
            + MARKER_WIDTH
            + INDENT * f32::from(self.depth)
            + CHAR_WIDTH * glyphs::columns(&self.label)
    }

    pub(super) fn view(
        &self,
        index: usize,
        picked: Option<usize>,
        by_part: bool,
        carried: bool,
        onto: Option<Mark>,
        width: f32,
    ) -> Element<'_, Message> {
        let label = text(self.label.as_str())
            .font(Font::MONOSPACE)
            .size(TREE_TEXT_SIZE)
            .wrapping(text::Wrapping::None);

        let label = if self.warn { label.style(text::danger) } else { label };

        let body = row![
            text(self.mark)
                .font(Font::MONOSPACE)
                .size(MARKER_SIZE)
                .line_height(MARKER_LINE_HEIGHT)
                .width(Length::Fixed(MARKER_WIDTH)),
            label,
        ]
        .align_y(Vertical::Center);

        let content = container(body)
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding(
                Padding::default()
                    .left(ROW_PADDING + INDENT * f32::from(self.depth))
                    .right(ROW_PADDING),
            );

        let held = if by_part { self.part } else { self.track };
        let selected = held.is_some() && held == picked;

        let seated = match self.part.is_some() {
            true => {
                let grip = mouse_area(content).on_press(Message::Press(index));

                list_row(grip, selected, false, Length::Fixed(width), Message::DragEnd)
            }
            false => list_row(content, selected, false, Length::Fixed(width), Message::Row(index)),
        };

        let row: Element<'_, Message> = container(seated)
            .width(Length::Fixed(width))
            .style(move |theme: &Theme| seat(theme, carried, onto))
            .into();

        match (self.track, self.owner) {
            (Some(track), _) => editor::target(row, Target::AnimCurve(track)),
            (_, Some(part)) => editor::target(row, Target::AnimPart(part)),
            _ => row,
        }
    }
}

fn seat(theme: &Theme, carried: bool, onto: Option<Mark>) -> container::Style {
    let palette = theme.palette();

    let background = match carried {
        true => Color { a: CARRIED_TINT, ..palette.primary },
        false => Color::TRANSPARENT,
    };

    let border = match onto {
        Some(Mark::Nest) => Border::default().rounded(4.0).width(NEST_BORDER).color(palette.success),
        _ => Border::default().rounded(4.0).width(0.0).color(palette.primary),
    };

    let seam = |lift: f32| iced::Shadow {
        color: palette.success,
        offset: iced::Vector::new(0.0, lift),
        blur_radius: 0.0,
    };

    container::Style {
        background: Some(background.into()),
        border,
        shadow: match onto {
            Some(Mark::Above) => seam(-SEAM_HEIGHT),
            Some(Mark::Below) => seam(SEAM_HEIGHT),
            _ => iced::Shadow::default(),
        },
        ..container::Style::default()
    }
}

fn curve_label(doc: &Maanim, at: usize) -> Option<(String, bool)> {
    let track = doc.track(at)?;

    let shadowed = doc
        .tracks()
        .iter()
        .skip(at + 1)
        .any(|later| later.part == track.part && later.kind == track.kind);

    let mut label = format!("{} \u{00b7} {}", kind_label(track.kind), key_label(track.keyframes.len()));

    if track.loop_count != 1 {
        label.push_str(&format!(" \u{00b7} {}", loop_label(track.loop_count)));
    }

    if shadowed {
        label.push_str(&format!(" \u{00b7} {}", SHADOWED_MARK));
    }

    Some((label, shadowed))
}

fn part_label(model: &Model, at: usize) -> String {
    let Some(declared) = model.parts.get(at) else {
        return format!("Part {}", at);
    };

    let mut label = match declared.name.trim() {
        "" => format!("Part {}", at),
        name => format!("Part {} \u{00b7} {}", at, name),
    };

    if let Some(mark) = hidden(declared) {
        label.push_str(&format!(" \u{00b7} {}", mark));
    }

    label
}

fn leaf(label: String, depth: u16, track: Option<usize>, warn: bool) -> TreeRow {
    TreeRow { label, depth, mark: "", part: None, owner: None, track, warn, bucket: false }
}

pub(super) fn listing(
    doc: Option<&Maanim>,
    model: Option<&Model>,
    expanded: &HashSet<usize>,
    loose_open: bool,
) -> Vec<TreeRow> {
    let tracks = doc.map_or(0, |doc| doc.tracks().len());

    let Some(model) = model else {
        return (0..tracks)
            .filter_map(|at| doc.and_then(|doc| curve_label(doc, at)))
            .enumerate()
            .map(|(at, (label, warn))| leaf(label, 0, Some(at), warn))
            .collect();
    };

    let mut listed = Vec::new();
    let count = model.parts.len();

    for (part, depth) in rows(model, expanded) {
        let curves: Vec<usize> = (0..tracks)
            .filter(|at| {
                doc.and_then(|doc| doc.track(*at)).is_some_and(|track| usize::try_from(track.part) == Ok(part))
            })
            .collect();

        let open = expanded.contains(&part);
        let depth = depth as u16;
        let barest = curves.is_empty() && !bears(model, part);

        listed.push(TreeRow {
            label: part_label(model, part),
            depth,
            mark: match (barest, open) {
                (true, _) => "",
                (_, true) => FOLDER_OPEN,
                (_, false) => FOLDER_SHUT,
            },
            part: Some(part),
            owner: Some(part),
            track: None,
            warn: false,
            bucket: false,
        });

        if !open {
            continue;
        }

        for at in curves {
            if let Some((label, warn)) = doc.and_then(|doc| curve_label(doc, at)) {
                listed.push(leaf(label, depth + 1, Some(at), warn));
            }
        }
    }

    let loose: Vec<usize> = (0..tracks)
        .filter(|at| {
            doc.and_then(|doc| doc.track(*at))
                .is_none_or(|track| !usize::try_from(track.part).is_ok_and(|part| part < count))
        })
        .collect();

    if loose.is_empty() {
        return listed;
    }

    listed.push(TreeRow {
        label: format!("{} \u{00b7} {}", LOOSE_LABEL, key_label(loose.len())),
        depth: 0,
        mark: if loose_open { FOLDER_OPEN } else { FOLDER_SHUT },
        part: None,
        owner: None,
        track: None,
        warn: true,
        bucket: true,
    });

    if !loose_open {
        return listed;
    }

    for at in loose {
        if let Some((label, warn)) = doc.and_then(|doc| curve_label(doc, at)) {
            listed.push(leaf(label, 1, Some(at), warn));
        }
    }

    listed
}

fn lineage(model: &Model) -> (Vec<Vec<usize>>, Vec<usize>) {
    let count = model.parts.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut roots = Vec::new();

    for (at, part) in model.parts.iter().enumerate() {
        let parent = usize::try_from(part.parent).ok().filter(|parent| *parent < count && *parent != at);

        match parent {
            Some(parent) => children[parent].push(at),
            None => roots.push(at),
        }
    }

    (children, roots)
}

pub(super) fn roots(model: &Model) -> Vec<usize> {
    lineage(model).1
}

pub(super) fn rows(model: &Model, expanded: &HashSet<usize>) -> Vec<(usize, usize)> {
    let count = model.parts.len();
    let (children, roots) = lineage(model);

    let mut listed = Vec::with_capacity(count);
    let mut seen = vec![false; count];

    for root in roots {
        descend(root, 0, &children, expanded, &mut seen, &mut listed, true);
    }

    for at in 0..count {
        descend(at, 0, &children, expanded, &mut seen, &mut listed, true);
    }

    listed
}

fn descend(
    at: usize,
    depth: usize,
    children: &[Vec<usize>],
    expanded: &HashSet<usize>,
    seen: &mut [bool],
    listed: &mut Vec<(usize, usize)>,
    visible: bool,
) {
    if seen.get(at).copied().unwrap_or(true) {
        return;
    }

    seen[at] = true;

    if visible {
        listed.push((at, depth));
    }

    let open = visible && expanded.contains(&at);

    for child in children.get(at).into_iter().flatten() {
        descend(*child, depth + 1, children, expanded, seen, listed, open);
    }
}

pub(super) fn hidden(part: &ModelPart) -> Option<&'static str> {
    if part.id < 0 {
        return Some("not drawn");
    }

    if part.sprite < 0 {
        return Some("no sprite");
    }

    if part.opacity == 0 {
        return Some("transparent");
    }

    if part.scale_x == 0 || part.scale_y == 0 {
        return Some("no scale");
    }

    None
}

pub(super) fn held_curve(doc: &Maanim, at: usize) -> Option<Held> {
    let track = doc.track(at)?;
    let ordinal = doc
        .tracks()
        .iter()
        .take(at)
        .filter(|other| other.part == track.part && other.kind == track.kind)
        .count();

    Some(Held { part: track.part, kind: track.kind, ordinal })
}

pub(super) fn locate_curve(doc: &Maanim, held: &Held) -> Option<usize> {
    doc.tracks()
        .iter()
        .enumerate()
        .filter(|(_, track)| track.part == held.part && track.kind == held.kind)
        .nth(held.ordinal)
        .map(|(at, _)| at)
}

pub(super) fn bears(model: &Model, part: usize) -> bool {
    let wanted = i32::try_from(part).ok();

    model.parts.iter().enumerate().any(|(at, other)| at != part && Some(other.parent) == wanted)
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    const SAMPLE: &str = "[modelanim:animation]\n1\n2\n0,11,-1,0,0,\n1\n0,0,0,0\n1,4,-1,0,0,\n1\n0,0,0,0\n";
    const DOUBLED: &str = "[modelanim:animation]\n1\n3\n5,11,-1,0,0,\n1\n0,0,0,0\n2,4,-1,0,0,\n1\n0,0,0,0\n5,11,-1,0,0,\n1\n0,0,0,0\n";

    fn model(parents: &[i32]) -> Model {
        Model {
            parts: parents.iter().map(|parent| ModelPart { parent: *parent, ..ModelPart::default() }).collect(),
            ..Model::default()
        }
    }

    fn doc() -> Maanim {
        Maanim::parse(SAMPLE.as_bytes()).expect("the sample parses")
    }

    #[test]
    fn a_remembered_channel_survives_another_being_inserted_before_it() {
        // Two channels share part 5 and kind 11, so an index alone cannot name one.
        let mut doc = Maanim::parse(DOUBLED.as_bytes()).expect("the sample parses");
        let held = held_curve(&doc, 2).expect("the track exists");

        assert_eq!(held, Held { part: 5, kind: 11, ordinal: 1 });

        doc.insert(0, authoring::blank_curve(9, 12, None));

        assert_eq!(locate_curve(&doc, &held), Some(3));
    }

    #[test]
    fn a_remembered_channel_is_gone_once_its_occurrence_is() {
        let mut doc = Maanim::parse(DOUBLED.as_bytes()).expect("the sample parses");
        let held = held_curve(&doc, 2).expect("the track exists");

        doc.remove(2);

        assert_eq!(locate_curve(&doc, &held), None);
        assert_eq!(locate_curve(&doc, &Held { part: 5, kind: 11, ordinal: 0 }), Some(0));
    }

    #[test]
    fn a_part_owns_its_channels_and_its_children_sit_beside_them() {
        // Part 0 is the root and part 1 hangs off it, each driven by one channel.
        let listed = listing(Some(&doc()), Some(&model(&[-1, 0])), &HashSet::from([0, 1]), false);

        let shape: Vec<(u16, bool)> =
            listed.iter().map(|row| (row.depth, row.track.is_some())).collect();

        assert_eq!(shape, vec![(0, false), (1, true), (1, false), (2, true)]);
    }

    #[test]
    fn a_leaf_says_nothing_and_shows_no_folder_mark() {
        // The tree has to end somewhere, so a part with nothing under it is not an
        // empty state to announce — it simply does not open.
        let listed = listing(None, Some(&model(&[-1, 0])), &HashSet::from([0, 1]), false);

        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|row| row.track.is_none()));
        assert_eq!(listed[1].mark, "", "a childless part drops its caret");
        assert_eq!(listed[0].mark, FOLDER_OPEN, "one that bears a child keeps it");
    }

    #[test]
    fn a_lone_root_is_worth_opening_but_several_are_not() {
        // Most units hang everything off part 0, so opening it saves a click every time.
        assert_eq!(roots(&model(&[-1, 0, 1])), vec![0]);
        assert_eq!(roots(&model(&[-1, -1, 0])), vec![0, 1]);
    }

    #[test]
    fn a_tree_starts_fully_collapsed() {
        let listed = listing(Some(&doc()), Some(&model(&[-1, 0])), &HashSet::new(), false);

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].mark, FOLDER_SHUT);
    }

    #[test]
    fn a_channel_naming_a_part_the_model_lacks_folds_away_but_stays_reachable() {
        // The engine does not bound check the part index, so the channel has to stay
        // reachable — but a broken file can hold hundreds, so the bucket starts shut.
        let model = model(&[-1]);
        let shut = listing(Some(&doc()), Some(&model), &HashSet::from([0]), false);

        let bucket = shut.iter().find(|row| row.bucket).expect("the bucket is listed");
        assert!(bucket.warn && bucket.mark == FOLDER_SHUT);
        assert_eq!(shut.iter().filter(|row| row.track.is_some()).count(), 1, "only part 0's own");

        let open = listing(Some(&doc()), Some(&model), &HashSet::from([0]), true);
        assert_eq!(open.iter().filter(|row| row.track.is_some()).count(), 2);
    }

    #[test]
    fn without_a_model_every_channel_still_lists_flat() {
        let listed = listing(Some(&doc()), None, &HashSet::new(), false);

        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|row| row.depth == 0 && row.track.is_some()));
    }

    #[test]
    fn a_hierarchy_lists_parents_before_their_children() {
        // 0 is the root, 1 and 3 hang off it, 2 hangs off 1.
        let listed = rows(&model(&[-1, 0, 1, 0]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (2, 2), (3, 1)]);
    }

    #[test]
    fn a_folded_part_hides_its_descendants_but_not_its_siblings() {
        let listed = rows(&model(&[-1, 0, 1, 0]), &HashSet::from([0]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (3, 1)]);
    }

    #[test]
    fn a_parent_cycle_still_lists_every_part() {
        // The file is not bound checked, so 1 and 2 pointing at each other has to
        // stay visible rather than dropping out of the tree entirely.
        let listed = rows(&model(&[-1, 2, 1]), &HashSet::from([0, 1, 2]));

        assert_eq!(listed.len(), 3);
        assert!(listed.iter().any(|(part, _)| *part == 1));
        assert!(listed.iter().any(|(part, _)| *part == 2));
    }

    #[test]
    fn a_parent_past_the_end_is_treated_as_a_root() {
        let listed = rows(&model(&[-1, 9]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn a_part_parented_to_itself_does_not_recurse() {
        let listed = rows(&model(&[-1, 1]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }
}
