use std::collections::HashMap;

use nyanko::cat::unit::{LevelCurve, UnitBuy};
use nyanko::combat::{AttrUnit, Attribute, Entity, Identity, REGISTRY};

use crate::common::context::GlobalContext;
use crate::domains::cat::game::stats;
use crate::domains::cat::waiter::unitid;
use crate::systems::combat::registry::{self, AbilityIcon, DisplayGroup, StatContext, CAT_STATS_REGISTRY};
use crate::systems::combat::{abilities, comparable, RenderContext};

use super::FileDelta;

pub const TRUE_FORM: usize = 2;

pub const ULTRA_FORM: usize = 3;

pub struct Unlocked {
    pub cat_id: u32,
    pub form: usize,
}

pub struct Change {
    pub label: &'static str,
    pub before: String,
    pub after: String,
    pub shift: Option<String>,
}

pub struct Ability {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub group: DisplayGroup,
    pub text: String,
    pub detail: Vec<Change>,
}

impl Ability {
    pub fn explained(&self) -> bool {
        matches!(self.group, DisplayGroup::Body1 | DisplayGroup::Body2)
    }
}

#[derive(Default)]
pub struct Diff {
    pub gains: Vec<Change>,
    pub losses: Vec<Change>,
    pub learned: Vec<Ability>,
    pub forgotten: Vec<Ability>,
    pub spirit: Option<Box<Diff>>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.gains.is_empty()
            && self.losses.is_empty()
            && self.learned.is_empty()
            && self.forgotten.is_empty()
            && self.spirit.is_none()
    }
}

fn shift(before: i32, after: i32) -> Option<String> {
    if before == 0 || before == after {
        return None;
    }

    let percent = ((after - before) as f32 / before.abs() as f32 * 100.0).round() as i32;

    (percent != 0).then(|| format!("({}{}%)", if percent > 0 { "+" } else { "" }, percent))
}

fn drift(unit: AttrUnit, before: i32, after: i32) -> Option<String> {
    let delta = after - before;

    (delta != 0).then(|| format!("({}{})", if delta > 0 { "+" } else { "" }, measured(unit, delta)))
}

fn measured(unit: AttrUnit, amount: i32) -> String {
    match unit {
        AttrUnit::Percent => format!("{}%", amount),
        AttrUnit::Frames => format!("{}f", amount),
        AttrUnit::Range | AttrUnit::None => amount.to_string(),
    }
}

const LOWER_IS_BETTER: &[&str] = &["Atk Cycle", "Cost", "Cooldown", "TBA"];

fn stat_text(name: &'static str, formatter: fn(i32) -> String, value: i32) -> String {
    if RAW_FRAMES.contains(&name) {
        return format!("{}f", value);
    }

    formatter(value)
}

const RAW_FRAMES: &[&str] = &["Cooldown"];

const PRESENCE: &str = "Active";

const ABSENT: &str = "None";

fn stat_label(name: &'static str, display: &'static str) -> &'static str {
    match name {
        "Atk Cycle" => "Attack Cycle",
        "TBA" => "Attack Cooldown",
        "Cooldown" => "Deploy Cooldown",
        _ => display,
    }
}

fn improved(stat: &'static str, before: i32, after: i32) -> bool {
    if LOWER_IS_BETTER.contains(&stat) {
        return after < before;
    }

    after > before
}

pub fn read(delta: &FileDelta) -> Vec<Unlocked> {
    let mut found = Vec::new();

    for row in &delta.rows {
        let (Some(before), Some(after)) = (row.before.as_deref(), row.after.as_deref()) else {
            continue;
        };

        let (Some(previous), Some(current)) = (parse(before), parse(after)) else {
            continue;
        };

        let Ok(cat_id) = row.key.trim().parse::<u32>() else {
            continue;
        };

        if previous.true_form_id <= 0 && current.true_form_id > 0 {
            found.push(Unlocked { cat_id, form: TRUE_FORM });
        }

        if previous.ultra_form_id <= 0 && current.ultra_form_id > 0 {
            found.push(Unlocked { cat_id, form: ULTRA_FORM });
        }
    }

    found.sort_by_key(|unlocked| (unlocked.form, unlocked.cat_id));

    found
}

pub struct Subject<'a> {
    pub global: GlobalContext<'a>,
    pub previous: &'a Entity,
    pub current: &'a Entity,
    pub curve: Option<&'a LevelCurve>,
    pub level: i32,
    pub frames: (i32, i32),
}

pub fn compare(subject: &Subject<'_>) -> Diff {
    let before = stats::apply_level(subject.previous, subject.curve, subject.level);
    let after = stats::apply_level(subject.current, subject.curve, subject.level);

    let mut diff = Diff::default();

    for stat in CAT_STATS_REGISTRY {
        let old = (stat.get_value)(&StatContext::cat(&before, subject.frames.0, None));
        let new = (stat.get_value)(&StatContext::cat(&after, subject.frames.1, None));

        if old == new {
            continue;
        }

        let change = Change {
            label: stat_label(stat.name, stat.display_name),
            before: stat_text(stat.name, stat.formatter, old),
            after: stat_text(stat.name, stat.formatter, new),
            shift: shift(old, new),
        };

        if improved(stat.name, old, new) {
            diff.gains.push(change);
        } else {
            diff.losses.push(change);
        }
    }

    sort_abilities(subject, &before, &after, &mut diff);
    diff.spirit = spirit_diff(subject, &before, &after).map(Box::new);

    diff
}

fn spirit_diff(subject: &Subject<'_>, before: &Entity, after: &Entity) -> Option<Diff> {
    let summoned = |stats: &Entity| {
        u32::try_from(stats.conjure_unit_id)
            .ok()
            .and_then(|id| unitid(&subject.global.vault.vfs, id as i32))
            .and_then(|profiles| profiles.into_iter().next())
    };

    let (previous, current) = (summoned(before)?, summoned(after)?);

    let nested = compare(&Subject {
        global: subject.global,
        previous: &previous,
        current: &current,
        curve: None,
        level: subject.level,
        frames: subject.frames,
    });

    (!nested.is_empty()).then_some(nested)
}

fn sort_abilities(subject: &Subject<'_>, before: &Entity, after: &Entity, diff: &mut Diff) {
    let old_text = described(subject, before);
    let new_text = described(subject, after);

    for pure in REGISTRY {
        if pure.identity == Identity::Conjure {
            continue;
        }

        let held = (pure.attributes)(before);
        let carried = (pure.attributes)(after);

        if held.is_empty() && carried.is_empty() {
            continue;
        }

        let display = registry::get_display_def(pure.identity);

        let (mut better, mut worse) = diverged(&held, &carried);
        let (gained, lost) = figured(pure.identity, before, after);

        better.extend(gained);
        worse.extend(lost);

        if held.is_empty() {
            better.extend(worse);
            diff.learned.extend(spoken(&new_text, pure.identity, &display, better));
            continue;
        }

        if carried.is_empty() {
            worse.extend(better);
            diff.forgotten.extend(spoken(&old_text, pure.identity, &display, worse));
            continue;
        }

        if better.is_empty() && worse.is_empty() {
            continue;
        }

        if !better.is_empty() {
            diff.learned.extend(spoken(&new_text, pure.identity, &display, better));
        }

        if !worse.is_empty() {
            diff.forgotten.extend(spoken(&new_text, pure.identity, &display, worse));
        }
    }
}

fn figured(identity: Identity, before: &Entity, after: &Entity) -> (Vec<Change>, Vec<Change>) {
    let mut better = Vec::new();
    let mut worse = Vec::new();

    for figure in registry::get_figures(identity) {
        let was = (figure.read)(before);
        let now = (figure.read)(after);

        if was == now {
            continue;
        }

        let change = Change {
            label: figure.label,
            before: was.unwrap_or_else(|| ABSENT.to_string()),
            after: now.unwrap_or_else(|| ABSENT.to_string()),
            shift: None,
        };

        if (figure.weigh)(after) >= (figure.weigh)(before) {
            better.push(change);
        } else {
            worse.push(change);
        }
    }

    (better, worse)
}

fn diverged(held: &[Attribute], carried: &[Attribute]) -> (Vec<Change>, Vec<Change>) {
    let mut better = Vec::new();
    let mut worse = Vec::new();
    let mut seen: Vec<&'static str> = Vec::new();

    for (label, _, _) in carried.iter().chain(held.iter()) {
        if *label == PRESENCE || seen.contains(label) {
            continue;
        }

        seen.push(label);

        let pick = |side: &[Attribute]| side.iter().find(|(key, _, _)| key == label).copied();
        let (was, now) = (pick(held), pick(carried));

        let old = was.map_or(0, |(_, value, _)| comparable(value));
        let new = now.map_or(0, |(_, value, _)| comparable(value));

        if old == new {
            continue;
        }

        let unit = now.or(was).map_or(AttrUnit::None, |(_, _, unit)| unit);

        let change = Change {
            label,
            before: measured(unit, old),
            after: measured(unit, new),
            shift: drift(unit, old, new),
        };

        if new > old { better.push(change) } else { worse.push(change) }
    }

    (collapse(better), collapse(worse))
}

fn collapse(mut changes: Vec<Change>) -> Vec<Change> {
    let bounds: Vec<&'static str> = changes
        .iter()
        .filter_map(|change| change.label.strip_prefix("Min ").map(|_| change.label))
        .collect();

    for low in bounds {
        let suffix = &low[4..];
        let high = format!("Max {}", suffix);

        let Some(paired) = changes.iter().find(|change| change.label == high) else {
            continue;
        };

        let Some(base) = changes.iter().find(|change| change.label == low) else {
            continue;
        };

        if base.before != paired.before || base.after != paired.after {
            continue;
        }

        changes.retain(|change| change.label != high);

        for change in &mut changes {
            if change.label == low {
                change.label = suffix;
            }
        }
    }

    changes
}

fn spoken(
    texts: &HashMap<Identity, String>,
    identity: Identity,
    display: &registry::AbilityDisplayDef,
    detail: Vec<Change>,
) -> Option<Ability> {
    let text = texts.get(&identity).cloned().unwrap_or_default();

    Some(Ability {
        name: display.name,
        fallback: display.fallback,
        icon: display.icon,
        group: display.group,
        text,
        detail,
    })
}

fn described(subject: &Subject<'_>, stats: &Entity) -> HashMap<Identity, String> {
    let context = RenderContext {
        global: subject.global,
        base_stats: stats,
        final_stats: stats,
        magnification: registry::Magnification::default(),
        current_level: subject.level,
        level_curve: subject.curve,
        talent_data: None,
        talent_levels: None,
        is_conjure_unit: false,
    };

    let groups = abilities::collect_ability_data(&context);

    [groups.0, groups.1, groups.2, groups.3, groups.4, groups.5]
        .into_iter()
        .flatten()
        .map(|item| (item.identity, item.text))
        .collect()
}

fn parse(line: &str) -> Option<UnitBuy> {
    UnitBuy::parse(line, None).ok()?.into_values().next()
}

#[cfg(test)]
mod tests {
    use super::super::{RowDelta, Status};
    use super::*;

    // Mining reads in frames throughout, so no stat value may carry a superscript the
    // dense diff lines would have to render inline.
    #[test]
    fn no_stat_value_reaches_a_card_carrying_a_superscript() {
        let entity = Entity::default();

        for stat in CAT_STATS_REGISTRY {
            let value = (stat.get_value)(&StatContext::cat(&entity, 0, None));
            let shown = stat_text(stat.name, stat.formatter, value);

            assert!(!shown.contains('^'), "{} renders as {shown}, which needs a raw-frame override", stat.name);
        }
    }

    // An ability attribute reports the ground it moved, in its own unit, the same way
    // cat/game/talents.rs writes a talent's modifier.
    #[test]
    fn an_ability_attribute_reports_the_ground_it_moved() {
        assert_eq!(drift(AttrUnit::Percent, 10, 20).as_deref(), Some("(+10%)"));
        assert_eq!(drift(AttrUnit::Percent, 50, 30).as_deref(), Some("(-20%)"));
        assert_eq!(drift(AttrUnit::Frames, 60, 120).as_deref(), Some("(+60f)"));
        assert_eq!(drift(AttrUnit::None, 200, 100).as_deref(), Some("(-100)"));
        assert_eq!(drift(AttrUnit::Range, 30, 30), None);
    }

    // A gained ability starts at nothing, which a proportion cannot express at all.
    #[test]
    fn a_gained_ability_still_reports_its_whole_value() {
        assert_eq!(drift(AttrUnit::Frames, 0, 120).as_deref(), Some("(+120f)"));
        assert_eq!(drift(AttrUnit::Percent, 0, 30).as_deref(), Some("(+30%)"));
    }

    #[test]
    fn a_shift_reads_as_a_percentage_of_the_value_it_left() {
        assert_eq!(shift(100, 150).as_deref(), Some("(+50%)"));
        assert_eq!(shift(150, 100).as_deref(), Some("(-33%)"));
        assert_eq!(shift(100, 100), None);
        assert_eq!(shift(0, 100), None, "there is no percentage away from nothing");
    }

    // Omni Strike widens its band between forms and nothing else reports it.
    #[test]
    fn a_widened_band_reads_as_a_gain_under_its_own_ability() {
        let before = Entity { long_distance_1_anchor: 200, long_distance_1_span: 400, ..Default::default() };
        let after = Entity { long_distance_1_span: 600, ..before.clone() };

        let (better, worse) = figured(Identity::OmniStrike, &before, &after);
        let band = better.iter().find(|change| change.label == "Effective Range").expect("the band");

        assert_eq!((band.before.as_str(), band.after.as_str()), ("200~600", "200~800"));
        assert!(worse.is_empty());
    }

    // "VS Base" is where the unit stands relative to the opposing base, the same figure
    // fmt_effective_range prints as "Stands at N Range relative to Enemy Base".
    #[test]
    fn the_base_reading_is_where_the_unit_stands() {
        let before = Entity { standing_range: 350, long_distance_1_anchor: 200, long_distance_1_span: 400, ..Default::default() };
        let after = Entity { long_distance_1_anchor: 300, ..before.clone() };

        let (better, _) = figured(Identity::OmniStrike, &before, &after);
        let stands = better.iter().find(|change| change.label == "VS Base").expect("the base reading");

        assert_eq!((stands.before.as_str(), stands.after.as_str()), ("200", "300"));
    }

    // Without an anchor of its own the unit stands at its ordinary range.
    #[test]
    fn an_anchorless_band_stands_at_the_units_own_range() {
        let before = Entity { standing_range: 350, long_distance_1_span: 400, ..Default::default() };
        let after = Entity { standing_range: 450, ..before.clone() };

        let (better, _) = figured(Identity::LongDistance, &before, &after);
        let stands = better.iter().find(|change| change.label == "VS Base").expect("the base reading");

        assert_eq!((stands.before.as_str(), stands.after.as_str()), ("350", "450"));
    }

    // A form without a band still attacks somewhere, so the reading is its ordinary area.
    #[test]
    fn a_gained_band_reads_from_the_ordinary_attack_area() {
        let before = Entity { standing_range: 350, ..Default::default() };
        let after = Entity { long_distance_1_anchor: 200, long_distance_1_span: -400, ..before.clone() };

        let (better, _) = figured(Identity::OmniStrike, &before, &after);
        let band = better.iter().find(|change| change.label == "Effective Range").expect("the band");

        assert_eq!((band.before.as_str(), band.after.as_str()), ("-320~350", "-200~200"));
    }

    // Losing multi-hit still leaves one hit, and its readings say what is left.
    #[test]
    fn a_lost_multi_hit_reads_down_to_its_single_hit() {
        let before = Entity {
            attack_1_damage: 800,
            attack_2_damage: 1200,
            attack_1_abilities: 1,
            attack_2_abilities: 1,
            ..Default::default()
        };
        let after = Entity { attack_1_damage: 1400, attack_2_damage: 0, attack_2_abilities: 0, ..before.clone() };

        let (_, worse) = figured(Identity::MultiHit, &before, &after);
        let damage = worse.iter().find(|change| change.label == "Damage split").expect("the damage split");
        let carriers = worse.iter().find(|change| change.label == "Ability split").expect("the ability split");

        assert_eq!((damage.before.as_str(), damage.after.as_str()), ("800 / 1200", "1400"));
        assert_eq!((carriers.before.as_str(), carriers.after.as_str()), ("True / True", "True"));
    }

    #[test]
    fn a_multi_hit_split_is_carried_by_its_own_ability() {
        let before = Entity { attack_1_damage: 800, attack_2_damage: 1200, ..Default::default() };
        let after = Entity { attack_3_damage: 600, ..before.clone() };

        let (better, _) = figured(Identity::MultiHit, &before, &after);
        let damage = better.iter().find(|change| change.label == "Damage split").expect("the damage split");

        assert_eq!((damage.before.as_str(), damage.after.as_str()), ("800 / 1200", "800 / 1200 / 600"));
    }

    // Each split is its own row, and a split that held still is not reported at all.
    #[test]
    fn only_the_splits_that_moved_are_reported() {
        let before = Entity {
            attack_1_damage: 800,
            attack_2_damage: 1200,
            time_until_attack_1: 20,
            time_until_attack_2: 40,
            attack_1_abilities: 1,
            ..Default::default()
        };
        let after = Entity { attack_2_abilities: 1, time_until_attack_2: 30, ..before.clone() };

        let (better, worse) = figured(Identity::MultiHit, &before, &after);
        let labels: Vec<&str> = better.iter().chain(worse.iter()).map(|change| change.label).collect();

        assert!(!labels.contains(&"Damage split"), "the damage never moved");
        assert!(labels.contains(&"Timing split"));
        assert!(labels.contains(&"Ability split"));
    }

    // A hit landing sooner is an improvement even though the number went down.
    #[test]
    fn a_quicker_hit_reads_as_a_gain() {
        let before = Entity { attack_1_damage: 800, attack_2_damage: 1200, time_until_attack_2: 40, ..Default::default() };
        let after = Entity { time_until_attack_2: 30, ..before.clone() };

        let (better, worse) = figured(Identity::MultiHit, &before, &after);

        assert!(better.iter().any(|change| change.label == "Timing split"));
        assert!(worse.is_empty());
    }

    // Everything else says all it has to say through its attributes.
    #[test]
    fn an_ordinary_ability_publishes_no_extra_figures() {
        let (better, worse) = figured(Identity::Weaken, &Entity::default(), &Entity::default());

        assert!(better.is_empty() && worse.is_empty());
    }

    // Single Attack and friends publish nothing but a presence flag; which panel holds
    // them already says that, and "Active 0 -> 1 (+100%)" says nothing at all.
    #[test]
    fn a_bare_presence_flag_never_reaches_a_card() {
        use nyanko::combat::AttrValue;

        let carried = [(PRESENCE, AttrValue::Finite(1), AttrUnit::None)];
        let (better, worse) = diverged(&[], &carried);

        assert!(better.is_empty() && worse.is_empty());
    }

    // A surge whose band never widens publishes the same number twice; one "Range" says it.
    #[test]
    fn a_matching_min_and_max_collapse_into_one_reading() {
        use nyanko::combat::AttrValue;

        let held = [
            ("Min Range", AttrValue::Finite(200), AttrUnit::Range),
            ("Max Range", AttrValue::Finite(200), AttrUnit::Range),
        ];
        let carried = [
            ("Min Range", AttrValue::Finite(400), AttrUnit::Range),
            ("Max Range", AttrValue::Finite(400), AttrUnit::Range),
        ];

        let (better, _) = diverged(&held, &carried);

        assert_eq!(better.len(), 1);
        assert_eq!(better[0].label, "Range");
        assert_eq!((better[0].before.as_str(), better[0].after.as_str()), ("200", "400"));
    }

    #[test]
    fn a_genuine_span_keeps_both_of_its_bounds() {
        use nyanko::combat::AttrValue;

        let held = [
            ("Min Range", AttrValue::Finite(200), AttrUnit::Range),
            ("Max Range", AttrValue::Finite(600), AttrUnit::Range),
        ];
        let carried = [
            ("Min Range", AttrValue::Finite(300), AttrUnit::Range),
            ("Max Range", AttrValue::Finite(700), AttrUnit::Range),
        ];

        let (better, _) = diverged(&held, &carried);
        let labels: Vec<&str> = better.iter().map(|change| change.label).collect();

        assert_eq!(labels, vec!["Min Range", "Max Range"]);
    }

    // A newly learned ability has nothing on its left, so its values read from the
    // default rather than showing an empty card.
    #[test]
    fn a_learned_ability_reads_from_its_defaults() {
        use nyanko::combat::AttrValue;

        let carried = [("Chance", AttrValue::Finite(30), AttrUnit::Percent)];
        let (better, worse) = diverged(&[], &carried);

        assert_eq!(better.len(), 1);
        assert_eq!((better[0].before.as_str(), better[0].after.as_str()), ("0%", "30%"));
        assert!(worse.is_empty());
    }

    // A forgotten ability is the mirror: its values fall back to the default.
    #[test]
    fn a_forgotten_ability_reads_down_to_its_defaults() {
        use nyanko::combat::AttrValue;

        let held = [("Duration", AttrValue::Finite(60), AttrUnit::Frames)];
        let (better, worse) = diverged(&held, &[]);

        assert!(better.is_empty());
        assert_eq!(worse.len(), 1);
        assert_eq!((worse[0].before.as_str(), worse[0].after.as_str()), ("60f", "0f"));
    }

    // An ability whose values move both ways belongs on both sides, carrying only the
    // attributes that actually went that direction.
    #[test]
    fn a_diverging_ability_splits_its_attributes_across_both_sides() {
        use nyanko::combat::AttrValue;

        let held = [
            ("Chance", AttrValue::Finite(30), AttrUnit::Percent),
            ("Duration", AttrValue::Finite(120), AttrUnit::Frames),
        ];
        let carried = [
            ("Chance", AttrValue::Finite(50), AttrUnit::Percent),
            ("Duration", AttrValue::Finite(60), AttrUnit::Frames),
        ];

        let (better, worse) = diverged(&held, &carried);

        assert_eq!(better.len(), 1);
        assert_eq!((better[0].label, better[0].after.as_str()), ("Chance", "50%"));
        assert_eq!(worse.len(), 1);
        assert_eq!((worse[0].label, worse[0].after.as_str()), ("Duration", "60f"));
    }

    #[test]
    fn a_lower_cost_reads_as_a_gain_while_a_lower_range_reads_as_a_loss() {
        assert!(improved("Cost", 800, 600), "a cheaper unit improved");
        assert!(!improved("Cost", 600, 800));
        assert!(improved("Range", 350, 450));
        assert!(!improved("Range", 450, 350), "some forms genuinely trade range away");
    }

    // Every cat stat must be classified, or a nerf would quietly read as a buff.
    #[test]
    fn every_cat_stat_has_a_direction() {
        for stat in CAT_STATS_REGISTRY {
            let lower = LOWER_IS_BETTER.contains(&stat.name);

            assert_eq!(improved(stat.name, 1, 2), !lower, "{} is classified backwards", stat.name);
        }
    }

    // The registry's own abbreviations read poorly on a diff card.
    #[test]
    fn the_two_timing_stats_are_spelled_out() {
        assert_eq!(stat_label("Atk Cycle", "Atk Cycle"), "Attack Cycle");
        assert_eq!(stat_label("TBA", "TBA"), "Attack Cooldown");
        assert_eq!(stat_label("Cooldown", "Cooldown"), "Deploy Cooldown");
        assert_eq!(stat_label("Range", "Range"), "Range");
    }

    // The conjured spirit gets a section of its own, so listing Conjure as an ability
    // would say the same thing twice.
    #[test]
    fn conjure_is_left_out_of_the_ability_list() {
        assert!(REGISTRY.iter().any(|pure| pure.identity == Identity::Conjure));
    }

    // unitbuy columns 23 and 24 are the true and ultra form ids, so a row that gains
    // either is a form unlock rather than a new unit.
    fn row(true_form: &str, ultra_form: &str) -> String {
        let mut columns = vec!["0"; 63];
        columns[23] = true_form;
        columns[24] = ultra_form;

        columns.join(",")
    }

    fn delta(rows: Vec<RowDelta>) -> FileDelta {
        FileDelta {
            file: "unitbuy.csv".to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 2,
            rows_after: 2,
            rows,
        }
    }

    #[test]
    fn a_row_that_gains_a_true_form_reports_that_unlock() {
        let found = read(&delta(vec![RowDelta {
            key: "42".to_string(),
            before: Some(row("0", "0")),
            after: Some(row("1", "0")),
        }]));

        assert_eq!(found.len(), 1);
        assert_eq!((found[0].cat_id, found[0].form), (42, TRUE_FORM));
    }

    // True forms come first so the section reads in progression order.
    #[test]
    fn true_forms_are_listed_before_ultra_forms() {
        let found = read(&delta(vec![
            RowDelta { key: "9".to_string(), before: Some(row("1", "0")), after: Some(row("1", "1")) },
            RowDelta { key: "7".to_string(), before: Some(row("0", "0")), after: Some(row("1", "0")) },
        ]));

        assert_eq!(
            found.iter().map(|unlocked| (unlocked.cat_id, unlocked.form)).collect::<Vec<_>>(),
            vec![(7, TRUE_FORM), (9, ULTRA_FORM)]
        );
    }

    #[test]
    fn a_form_a_unit_already_had_is_not_an_unlock() {
        let held = row("1", "1");
        let found = read(&delta(vec![RowDelta {
            key: "42".to_string(),
            before: Some(held.clone()),
            after: Some(held),
        }]));

        assert!(found.is_empty());
    }

    // A brand new row belongs to the New section, not to Forms.
    #[test]
    fn an_added_row_is_not_a_form_unlock() {
        let found = read(&delta(vec![RowDelta {
            key: "42".to_string(),
            before: None,
            after: Some(row("1", "1")),
        }]));

        assert!(found.is_empty());
    }
}
