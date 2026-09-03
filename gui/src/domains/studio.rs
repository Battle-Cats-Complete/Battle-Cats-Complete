use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::{button, column, container, pick_list, row, rule, scrollable, space, stack, text, text_input, Column, Space};
use iced::widget::{mouse_area, operation, responsive, tooltip};
use iced::border::Border;
use iced::{widget, Color, Element, Font, Length, Padding, Point, Size, Task, Theme};
use tracing::{info, warn};

use kore::common::architecture;
use kore::common::preview::{self, Stamp};
use kore::domains::settings::{AnimSettings, Settings};
use kore::domains::studio as sets;
use kore::systems::animation::authoring::{self as authoring, bound, Imgcut, CUT_FIELDS, CUT_NAME_FIELD, ease_label, ease_takes_power, ease_value, key_label, kind_label, loop_label, nameable, Maanim, Mamodel, EASES, FIELDS, NAME_FIELD};
use image::RgbaImage;
use nyanko::graphics::rig::{Keyframe, Model, ModelPart, Opaque, Rig, SpriteCut};
use nyanko::graphics::tools::timeline;

use crate::app::state::AnimState;
use crate::app::theme;
use crate::editor::{self, Target};
use crate::systems::animation::{self as viewer, controls, overlay};

use crate::common::feedback::{Slot, LOCKED_NOTICE};
use crate::common::{dialog, glyphs};
use crate::common::row_window::{self, RowWindow};
use crate::widget::{list_row, picture, popup, slide, smooth_scroll, Slide};

mod history;
mod manage;

use history::{History, Tag};

pub(crate) use sets::Set;

const PANEL_WIDTH: f32 = 372.0;
const PANEL_PADDING: f32 = 6.0;
const BODY_PADDING: f32 = 6.0;
const GAP: f32 = 6.0;
const ROW_GAP: f32 = 3.0;

const CLOSE_LABEL: &str = "\u{00d7}";
const CONFIRM_MARK: &str = "?";

const LABEL_SIZE: f32 = 12.0;
const CELL_SIZE: f32 = 12.0;
const CELL_PADDING: f32 = 3.0;
const INDEX_WIDTH: f32 = 22.0;
const ROW_HEIGHT: f32 = 21.0;
const ROW_SPACING: f32 = 0.0;
const ROW_PADDING: f32 = 6.0;
const INDENT: f32 = 11.0;

const TREE_TEXT_SIZE: f32 = 11.0;
const CHAR_WIDTH: f32 = TREE_TEXT_SIZE * 0.6;

const MARKER_SIZE: f32 = 18.0;
const MARKER_LINE_HEIGHT: f32 = ROW_HEIGHT / MARKER_SIZE;
const MARKER_WIDTH: f32 = 14.0;

const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const SCROLLBAR_ALLOWANCE: f32 = SCROLLBAR_WIDTH + SCROLLBAR_MARGIN * 2.0;
const SCROLL_TAIL: f32 = 14.0;
const KEY_ROW_HEIGHT: f32 = 44.0;
const KEY_ROW_PAD: f32 = 3.0;
const KEY_ROW_INSET: f32 = 2.0;

const PLAYING_TICK: Duration = Duration::from_millis(16);
const RESTING_TICK: Duration = Duration::from_millis(200);
const RECALL_CAP: usize = 5;
const NOTICE_EXPIRY: Duration = Duration::from_secs(6);
const NOTICE_PAD_X: f32 = 7.0;
const NOTICE_PAD_Y: f32 = 7.0;
const NOTICE_OVERHANG: f32 = 4.0;
const NOTICE_TEXT_SIZE: f32 = 13.0;
const RENAME_DELAY: Duration = Duration::from_millis(700);
const KEY_HEAD_HEIGHT: f32 = 19.0;
const DEBUG_WIDTH: f32 = 104.0;

const FOLDER_OPEN: &str = "\u{25be}";
const FOLDER_SHUT: &str = "\u{25b8}";
const EASE_WIDTH: f32 = 86.0;
const DROP_WIDTH: f32 = 18.0;
const STEP_WIDTH: f32 = 44.0;
const ACTIVE_TINT: f32 = 0.68;
const FACT_ROW_HEIGHT: f32 = 20.0;
const FACT_ROW_PAD: f32 = 4.0;
const FACT_LABEL: f32 = 62.0;
const LOOP_WIDTH: f32 = 56.0;
const LOOP_CARD_PAD: f32 = 3.0;
const LOOP_DEFAULT: i32 = 1;
const LOOP_HINT: &str = "1";
const CELL_HINT: &str = "0";
const HEAD_HEIGHT: f32 = 22.0;
const FIELD_ROW_HEIGHT: f32 = 24.0;
const TREE_TOP: f32 = BODY_PADDING + PANEL_PADDING + HEAD_HEIGHT + ROW_GAP + 1.0 + ROW_GAP + theme::CONSOLE_BORDER_WIDTH;
const TREE_LEFT: f32 = BODY_PADDING + PANEL_PADDING + theme::CONSOLE_BORDER_WIDTH;
const TREE_RIGHT: f32 = BODY_PADDING + PANEL_WIDTH - PANEL_PADDING;
const NEST_BAND: f32 = 0.28;
const NEST_BORDER: f32 = 2.0;
const SEAM_HEIGHT: f32 = 2.0;
const CARRIED_TINT: f32 = 0.35;
const DRAG_HASTE: f32 = 2.0;
const GHOST_FILL: f32 = 0.22;
const GHOST_EDGE: f32 = 0.55;
const GHOST_INK: f32 = 0.7;
const GHOST_PAD: f32 = 3.0;
const OFFSET_WIDTH: f32 = 74.0;
const CUT_CELL_WIDTH: f32 = 30.0;
const CUT_STEP_WIDTH: f32 = 46.0;
const FRAME_HINT: &str = "Right click & drag to set the cut";
const ALIGN_WIDTH: f32 = 44.0;
const BUFFER_MARK: char = '!';

const LOADING_NOTICE: &str = "Loading animation\u{2026}";
const NO_SET_NOTICE: &str = "No set is loaded";
const NO_SET_HINT: &str = "Open Manage to import, pick or create a set";
const NO_CLIP_NOTICE: &str = "This clip has no channels to edit";
const UNREADABLE_NOTICE: &str = "This animation could not be read";
const EMPTY_TRACK_NOTICE: &str = "This channel holds no keyframes";
const NO_CURVE_CHOSEN: &str = "Select a channel to edit its keyframes";
const WRITE_FAILED_NOTICE: &str = "The last change could not be saved";
const NO_SPRITE_NOTICE: &str = "none";
const PAST_ATLAS_NOTICE: &str = "past the atlas";
const BLANK_CUT_NOTICE: &str = "nothing visible";
const WHOLE_CUT_NOTICE: &str = "the whole cut";
const UNNAMED_CUT_NOTICE: &str = "unnamed";
const NO_PART_NOTICE: &str = "This channel drives a part the model does not declare";
const LOST_PART_NOTICE: &str = "This part is no longer in the loaded model";
const SPRITE_KIND: i32 = 2;
const ALPHA_FLOOR: u8 = 8;
const ADRIFT_TINT: f32 = 0.35;
const OUT_OF_BOUNDS: &str = "Coordinate is out of bounds";
const NO_ATLAS_NOTICE: &str = "This atlas could not be read";
const NO_CUTS_NOTICE: &str = "This atlas declares no regions";
const NO_PART_CHOSEN: &str = "Select a part to edit its rest pose";
const NO_MODEL_NOTICE: &str = "This model could not be read";
const LOOSE_LABEL: &str = "Channels with no declared part";
const SHADOWED_MARK: &str = "overridden";

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    Atlas,
    #[default]
    Entity,
}

impl Mode {
    fn other(self) -> Mode {
        match self {
            Mode::Atlas => Mode::Entity,
            Mode::Entity => Mode::Atlas,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::Atlas => "Atlas",
            Mode::Entity => "Entity",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Swap {
    Same,
    Fresh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Focus {
    #[default]
    Curve,
    Part,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lens {
    Rig,
    Selected,
    Hierarchy,
    Origin,
    World,
}

impl Lens {
    const ALL: [Lens; 5] = [Lens::Rig, Lens::Selected, Lens::Hierarchy, Lens::Origin, Lens::World];

    fn label(self) -> &'static str {
        match self {
            Lens::Rig => "Rig",
            Lens::Selected => "Selected",
            Lens::Hierarchy => "Hierarchy",
            Lens::Origin => "Origin",
            Lens::World => "World",
        }
    }

    fn of(self, anim: &mut AnimSettings) -> &mut bool {
        match self {
            Lens::Rig => &mut anim.show_rig,
            Lens::Selected => &mut anim.show_selected,
            Lens::Hierarchy => &mut anim.show_hierarchy,
            Lens::Origin => &mut anim.show_origin,
            Lens::World => &mut anim.show_world,
        }
    }

    fn on(self, anim: &AnimSettings) -> bool {
        match self {
            Lens::Rig => anim.show_rig,
            Lens::Selected => anim.show_selected,
            Lens::Hierarchy => anim.show_hierarchy,
            Lens::Origin => anim.show_origin,
            Lens::World => anim.show_world,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Frame,
    Value,
    Ease,
    Power,
}

impl Field {
    const ALL: [Field; 4] = [Field::Frame, Field::Value, Field::Ease, Field::Power];
    const TYPED: [Field; 3] = [Field::Frame, Field::Value, Field::Power];

    fn label(self) -> &'static str {
        match self {
            Field::Frame => "Frame",
            Field::Value => "Value",
            Field::Ease => "Ease",
            Field::Power => "Power",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }

    fn default(self, neutral: i32) -> i32 {
        match self {
            Field::Value => neutral,
            _ => 0,
        }
    }
}

pub(crate) struct Channels {
    pub(crate) part: usize,
    pub(crate) label: String,
    pub(crate) present: Vec<(i32, usize)>,
    pub(crate) channelled: bool,
    pub(crate) mount: String,
}

#[derive(Clone)]
pub(crate) struct Plan {
    set: sets::Set,
    target_mod: Option<String>,
    clip: Option<String>,
}

pub(crate) fn plan(set: sets::Set, target_mod: Option<String>, clip: Option<String>) -> Plan {
    Plan { set, target_mod, clip }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Viewer(viewer::Message),
    Row(usize),
    Scrolled(f32),
    StripScrolled(f32),
    Changed(usize, Field, String),
    LoopChanged(String),
    AddKey,
    DropKey(usize),
    Seek(usize),
    Bound(usize),
    EaseChanged(usize, i32),
    DropExpired,
    Overlay(Lens),
    Locate,
    Switch(Mode),
    Press(usize),
    DragMove(Point),
    DragEnd,
    Offset(usize),
    Picture(picture::Message),
    Carved(Option<Arc<Sheet>>),
    Cut(usize, usize, String),
    Frame(usize),
    Trim(usize),
    Slice(usize),
    Find(usize),
    DropCut(usize),
    DropCutExpired,
    Field(usize, String),
    OffsetChanged(usize, String),
    AddPart,
    AddCut,
    Undo,
    Persisted(u64, PathBuf, Option<Stamp>),
    OpenManage,
    ManagePopup(popup::Message),
    Manage(manage::Message),
    Export,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Drag {
    #[default]
    Idle,
    Pressed {
        row: usize,
        part: usize,
        since: Instant,
    },
    Moving {
        part: usize,
        at: Point,
        onto: Option<Landing>,
    },
}

impl Drag {
    fn carrying(self) -> Option<usize> {
        match self {
            Drag::Moving { part, .. } => Some(part),
            _ => None,
        }
    }

    fn landing(self) -> Option<Landing> {
        match self {
            Drag::Moving { onto, .. } => onto,
            _ => None,
        }
    }

    fn ripe(self) -> bool {
        match self {
            Drag::Pressed { since, .. } => {
                since.elapsed() >= Duration::from_secs_f32(controls::HOLD_DELAY_SECS / DRAG_HASTE)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Landing {
    Onto(usize),
    Seam(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Nest,
    Above,
    Below,
}

impl Landing {
    fn mark(self, row: usize, last: usize) -> Option<Mark> {
        match self {
            Landing::Onto(at) if at == row => Some(Mark::Nest),
            Landing::Seam(gap) if gap == row => Some(Mark::Above),
            Landing::Seam(gap) if gap == row + 1 && row == last => Some(Mark::Below),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Held {
    part: i32,
    kind: i32,
    ordinal: usize,
}

struct Recall {
    key: String,
    expanded: HashSet<usize>,
    clip: Option<String>,
    curve: Option<Held>,
    part: Option<usize>,
    focus: Focus,
}

pub struct State {
    unlocked: bool,
    session: Option<Session>,
    idle: viewer::State,
    recalled: Vec<Recall>,
    manage: manage::State,
    managing: bool,
    popup: popup::State,
    mode: Mode,
    notice: Option<Instant>,
    notice_text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            unlocked: false,
            session: None,
            idle: viewer::State::with_popup(popup::Kind::Animator),
            recalled: Vec::new(),
            manage: manage::State::default(),
            managing: false,
            popup: popup::State::default(),
            mode: Mode::default(),
            notice: None,
            notice_text: String::new(),
        }
    }
}

struct Showing {
    viewer: viewer::State,
    draft: Option<Draft>,
    pose: Option<Pose>,
    atlas: Option<Sheet>,
    opened: Option<PathBuf>,
    posed: Option<PathBuf>,
    cutting: Option<PathBuf>,
    rows: Vec<TreeRow>,
    widest: f32,
    listed: String,
}

impl Default for Showing {
    fn default() -> Self {
        Self {
            viewer: viewer::State::with_popup(popup::Kind::Animator),
            draft: None,
            pose: None,
            atlas: None,
            opened: None,
            posed: None,
            cutting: None,
            rows: Vec::new(),
            widest: 0.0,
            listed: String::new(),
        }
    }
}

struct Session {
    plan: Plan,
    viewer: viewer::State,
    draft: Option<Draft>,
    pose: Option<Pose>,
    atlas: Option<Sheet>,
    mode: Mode,
    opened: Option<PathBuf>,
    posed: Option<PathBuf>,
    cutting: Option<PathBuf>,
    expanded: HashSet<usize>,
    wanted: Option<Held>,
    wanted_part: Option<usize>,
    rows: Vec<TreeRow>,
    widest: f32,
    listed: String,
    seeded: bool,
    scroll: f32,
    scroll_id: widget::Id,
    strip_scroll: f32,
    strip_id: widget::Id,
    fields_id: widget::Id,
    picture: picture::State,
    framing: Option<usize>,
    slice: Option<usize>,
    carving: bool,
    slicing: Slot<usize>,
    drag: Drag,
    confirm: Slot<usize>,
    focus: Focus,
    loose_open: bool,
    history: History,
    key: String,
    primed: bool,
}

impl State {
    fn stash(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };

        let held = Recall {
            key: session.plan.set.name.clone(),
            expanded: session.expanded.clone(),
            clip: session.viewer.selected_label(),
            curve: session.draft.as_ref().and_then(|draft| held_curve(&draft.doc, draft.track?)),
            part: session.pose.as_ref().and_then(|pose| pose.part),
            focus: session.focus,
        };

        self.recalled.retain(|known| known.key != held.key);
        self.recalled.push(held);

        while self.recalled.len() > RECALL_CAP {
            self.recalled.remove(0);
        }
    }

    pub(crate) fn begin(&mut self, mut plan: Plan) {
        self.flush_now();
        self.stash();

        let Showing {
            viewer,
            draft,
            pose,
            atlas,
            opened,
            posed,
            cutting,
            rows,
            widest,
            listed,
        } = self.session.take().map_or_else(Showing::default, Session::showing);

        let held = self.recalled.iter().find(|known| known.key == plan.set.name);
        let expanded = held.map(|held| held.expanded.clone()).unwrap_or_default();
        let seeded = held.is_some();

        let wanted = held.and_then(|held| held.curve);
        let wanted_part = held.and_then(|held| held.part);
        let focus = held.map_or_else(Focus::default, |held| held.focus);

        if let Some(clip) = held.and_then(|held| held.clip.clone()) {
            plan.clip = Some(clip);
        }

        let key = plan.set.key();

        self.session = Some(Session {
            plan,
            viewer,
            draft,
            pose,
            atlas,
            mode: self.mode,
            opened,
            posed,
            cutting,
            expanded,
            wanted,
            wanted_part,
            rows,
            widest,
            listed,
            seeded,
            scroll: 0.0,
            scroll_id: widget::Id::unique(),
            strip_scroll: 0.0,
            strip_id: widget::Id::unique(),
            fields_id: widget::Id::unique(),
            picture: picture::State::default(),
            framing: None,
            slice: None,
            carving: false,
            slicing: Slot::default(),
            drag: Drag::default(),
            confirm: Slot::default(),
            focus,
            loose_open: false,
            history: History::default(),
            key,
            primed: false,
        });
    }

    pub(crate) fn adopt(
        &mut self,
        set: Set,
        target_mod: Option<String>,
        clip: Option<String>,
        copied: bool,
    ) {
        let name = set.name.clone();

        match copied {
            true => self.manage.adopt(set.clone()),
            false => self.manage.seal(set.clone()),
        }

        self.begin(plan(set, target_mod, clip));

        if !copied {
            return;
        }

        self.raise(format!(
            "{}\nAnimation data copied to \"{}/{}\"",
            LOCKED_NOTICE,
            architecture::STUDIO,
            name
        ));
    }

    pub(crate) fn channels(&self, part: Option<usize>) -> Option<Channels> {
        let session = self.session.as_ref().filter(|session| session.mode == Mode::Entity)?;
        let pose = session.pose.as_ref()?;
        let part = part.filter(|at| *at < pose.doc.count())?;
        let wanted = i32::try_from(part).ok();

        let present = session.draft.as_ref().map_or_else(Vec::new, |draft| {
            draft
                .doc
                .tracks()
                .iter()
                .enumerate()
                .filter(|(_, track)| Some(track.part) == wanted)
                .map(|(at, track)| (track.kind, at))
                .collect()
        });

        Some(Channels {
            part,
            label: format!("Part {}", part),
            present,
            channelled: session.draft.is_some(),
            mount: session.mount(),
        })
    }

    fn mount_of(plan: &Plan) -> String {
        plan.target_mod.clone().unwrap_or_else(|| match sets::folder_name(&plan.set) {
            Some(folder) => format!("{}/{}", architecture::STUDIO, folder),
            None => architecture::GAME.to_owned(),
        })
    }

    pub(crate) fn holder(&self, track: usize) -> Option<usize> {
        let session = self.session.as_ref().filter(|session| session.mode == Mode::Entity)?;
        let draft = session.draft.as_ref()?;

        usize::try_from(draft.doc.track(track)?.part).ok()
    }

    pub(crate) fn add_part(&mut self, parent: Option<usize>) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Parts);

        let Some(pose) = session.pose.as_mut() else {
            return false;
        };

        pose.grow(parent);

        let settled = pose.persist_now();

        if let Some(parent) = parent {
            session.expanded.insert(parent);
        }

        session.focus = Focus::Part;
        session.settle_pose();

        settled != Settled::Failed
    }

    pub(crate) fn drop_part(&mut self, part: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Bulk);

        let Some(moved) = session.pose.as_mut().and_then(|pose| pose.doc.remove_part(part)) else {
            return false;
        };

        session.restructure(moved);

        true
    }

    pub(crate) fn add_channel(&mut self, part: usize, kind: i32) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Keys);

        let seeded = authoring::blank_curve(part, kind, session.viewer.rig().map(|rig| &rig.model));

        let Some(draft) = session.draft.as_mut() else {
            return false;
        };

        let at = draft.doc.tracks().len();
        draft.doc.insert(at, seeded);
        draft.retrack(at);
        draft.backing.dirty = true;

        session.focus = Focus::Curve;
        session.aim();
        session.relist();

        true
    }

    pub(crate) fn drop_channel(&mut self, track: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Keys);

        let Some(draft) = session.draft.as_mut() else {
            return false;
        };

        if draft.doc.remove(track).is_none() {
            return false;
        }

        draft.track = match draft.track {
            Some(held) if held == track => None,
            Some(held) if held > track => Some(held - 1),
            held => held,
        };

        draft.restate();
        draft.backing.dirty = true;

        session.aim();
        session.relist();

        true
    }

    pub(crate) fn pace(&self) -> Option<Duration> {
        let session = self.session.as_ref()?;
        let live = session.viewer.playing()
            || session.draft.as_ref().is_some_and(|draft| draft.backing.busy())
            || session.pose.as_ref().is_some_and(|pose| pose.backing.busy())
            || session.atlas.as_ref().is_some_and(|atlas| atlas.backing.busy());

        Some(if live { PLAYING_TICK } else { RESTING_TICK })
    }

    pub(crate) fn flush_now(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.flush();
        }
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        settings: &mut Settings,
        anim: &mut AnimState,
    ) -> Task<Message> {
        self.unlocked = settings.files.unlock_game_mount;

        if let Some(task) = self.chrome(&message) {
            return task;
        }

        let Some(session) = self.session.as_mut() else {
            return Task::none();
        };

        if let Some(draft) = session.draft.as_mut() {
            let typing = matches!(&message, Message::Changed(at, field, _) if draft.buffering(*at, *field));

            if !typing {
                draft.resolve_buffer();
            }

            if !matches!(&message, Message::LoopChanged(_)) {
                draft.resolve_looping();
            }
        }

        if let Some(pose) = session.pose.as_mut() {
            let typing = match &message {
                Message::Field(at, _) => pose.buffering(Slotted::Cell(*at)),
                Message::OffsetChanged(axis, _) => pose.buffering(Slotted::Axis(*axis)),
                _ => false,
            };

            if !typing {
                pose.resolve_buffer();
            }
        }

        if let Some(atlas) = session.atlas.as_mut() {
            let typing = matches!(&message, Message::Cut(at, cell, _) if atlas.buffering(*at, *cell));

            if !typing {
                atlas.resolve_buffer();
            }
        }

        match message {
            Message::Tick => {
                let priming = session.sync(settings, anim);
                session.viewer.tick();
                session.repose();

                let carving = session.recarve();
                let flush =
                    session.draft.as_mut().map_or_else(Task::none, |draft| draft.persist_if_dirty());
                let shaped =
                    session.pose.as_mut().map_or_else(Task::none, |pose| pose.persist_if_dirty());
                let carved =
                    session.atlas.as_mut().map_or_else(Task::none, |atlas| atlas.persist_if_dirty());

                Task::batch([priming, carving, flush, shaped, carved])
            }
            Message::Viewer(msg) => {
                session.viewer.update(msg, settings, anim).map(Message::Viewer)
            }
            Message::Press(index) => {
                let part = session.rows.get(index).and_then(|row| row.part);

                session.drag = match part {
                    Some(part) => Drag::Pressed { row: index, part, since: Instant::now() },
                    None => Drag::Idle,
                };

                Task::none()
            }
            Message::DragMove(at) => {
                let carried = match session.drag {
                    Drag::Moving { part, .. } => Some(part),
                    Drag::Pressed { part, .. } if session.drag.ripe() => Some(part),
                    _ => None,
                };

                if let Some(part) = carried {
                    session.drag = Drag::Moving { part, at, onto: session.landing(part, at) };
                }

                Task::none()
            }
            Message::DragEnd => {
                let settled = std::mem::take(&mut session.drag);

                match settled {
                    Drag::Pressed { row, .. } => {
                        session.select(row);

                        return Task::none();
                    }
                    Drag::Moving { onto: None, .. } => {}
                    Drag::Moving { part, onto: Some(onto), .. } => session.land(part, onto),
                    _ => {}
                }

                Task::none()
            }
            Message::Picture(picture::Message::Framed(outline)) => {
                let Some(at) = session.framing.take() else {
                    return Task::none();
                };

                session.remember(Tag::Cuts);

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                let region =
                    [outline.x as i32, outline.y as i32, outline.width as i32, outline.height as i32];

                atlas.hidden = None;
                atlas.place(at, region);
                atlas.restate();

                atlas.persist_if_dirty()
            }
            Message::Picture(picture::Message::Cancelled) => {
                session.framing = None;
                session.redraw();

                Task::none()
            }
            Message::Picture(picture::Message::Picked(at)) => {
                let hit = session.atlas.as_ref().and_then(|atlas| atlas.over(at));

                session.reslice(hit);

                Task::none()
            }
            Message::Picture(msg) => {
                session.picture.update(msg);

                Task::none()
            }
            Message::Slice(at) => {
                session.reslice(Some(at));

                Task::none()
            }
            Message::Find(at) => {
                if let Some(atlas) = session.atlas.as_ref()
                    && let Some(centre) = atlas.find(at)
                {
                    session.picture.focus(&atlas.source, centre);
                }

                Task::none()
            }
            Message::Carved(carved) => {
                session.carving = false;
                session.atlas = carved.and_then(Arc::into_inner);
                session.framing = None;
                session.slice = None;
                session.picture.reset();

                Task::none()
            }
            Message::Cut(at, cell, value) => {
                session.remember(Tag::Cut(at, cell));

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                atlas.edit(at, cell, &value);

                atlas.persist_if_dirty()
            }
            Message::Frame(at) => {
                session.framing = match session.framing {
                    Some(held) if held == at => None,
                    _ => Some(at),
                };

                session.redraw();

                Task::none()
            }
            Message::Trim(at) => {
                session.remember(Tag::Cuts);

                let region = session.atlas.as_ref().and_then(|atlas| atlas.opaque(at));

                let (Some(region), Some(atlas)) = (region, session.atlas.as_mut()) else {
                    return Task::none();
                };

                atlas.place(at, region);

                atlas.persist_if_dirty()
            }
            Message::DropCut(at) => {
                if !session.slicing.take(&at) {
                    return session.slicing.set(at, Message::DropCutExpired);
                }

                session.remember(Tag::Bulk);

                let Some(moved) = session.atlas.as_mut().and_then(|atlas| atlas.doc.remove_cut(at)) else {
                    return Task::none();
                };

                session.recut(moved);

                Task::none()
            }
            Message::DropCutExpired => {
                session.slicing.expire();

                Task::none()
            }
            Message::Offset(row) => {
                if let Some(pose) = session.pose.as_mut() {
                    pose.aim(row);
                }

                Task::none()
            }
            Message::Row(index) => {
                session.select(index);

                Task::none()
            }
            Message::Scrolled(offset) => {
                session.scroll = offset;

                if let Drag::Moving { part, at, .. } = session.drag {
                    session.drag = Drag::Moving { part, at, onto: session.landing(part, at) };
                }

                Task::none()
            }
            Message::StripScrolled(offset) => {
                session.strip_scroll = offset;

                Task::none()
            }
            Message::Switch(mode) => {
                session.mode = mode;
                session.focus = Focus::Curve;
                session.framing = None;
                session.repose();
                session.aim();
                session.relist();

                session.recarve()
            }
            Message::Field(at, value) => {
                session.remember(Tag::Field(at));

                let Some(pose) = session.pose.as_mut() else {
                    return Task::none();
                };

                pose.edit(at, &value);

                pose.persist_if_dirty()
            }
            Message::OffsetChanged(axis, value) => {
                session.remember(Tag::Axis(axis));

                let Some(pose) = session.pose.as_mut() else {
                    return Task::none();
                };

                pose.shift(axis, &value);

                pose.persist_if_dirty()
            }
            Message::AddPart => {
                let parent = session.pose.as_ref().and_then(|pose| pose.part);

                self.add_part(parent);

                Task::none()
            }
            Message::Locate => {
                session.viewer.locate();

                Task::none()
            }
            Message::Overlay(lens) => {
                let flag = lens.of(&mut settings.animation);
                *flag = !*flag;

                Task::none()
            }
            Message::LoopChanged(value) => {
                session.remember(Tag::Loop);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.set_looping(&value);

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::AddKey => {
                session.remember(Tag::Keys);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.add_key();

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::Seek(at) => {
                if let Some(frame) = session.draft.as_ref().and_then(|draft| draft.key_at(at)) {
                    session.viewer.seek(frame as f32);
                }

                Task::none()
            }
            Message::Bound(at) => {
                if let Some((start, end)) = session.draft.as_ref().and_then(|draft| draft.key_span(at)) {
                    session.viewer.bound(start as f32, end as f32);
                }

                Task::none()
            }
            Message::DropKey(at) => {
                if !session.confirm.take(&at) {
                    return session.confirm.set(at, Message::DropExpired);
                }

                session.remember(Tag::Keys);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.drop_key(at);

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::DropExpired => {
                session.confirm.expire();

                Task::none()
            }
            Message::EaseChanged(at, ease) => {
                session.remember(Tag::Ease(at));

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.set_ease(at, ease);

                draft.persist_if_dirty()
            }
            Message::Changed(at, field, value) => {
                session.remember(Tag::Key(at, field));

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                let moved = draft.edit(at, field, &value);
                let chase = moved
                    .and_then(|to| Field::TYPED.iter().position(|known| *known == field).map(|column| (to, column)))
                    .map(|(to, column)| operation::focus(draft.cursor(to, column)));

                let flush = draft.persist_if_dirty();

                match chase {
                    Some(chase) => Task::batch([flush, chase]),
                    None => flush,
                }
            }
            Message::Persisted(token, path, stamp) => {
                if let Some(atlas) = session.atlas.as_mut().filter(|atlas| atlas.backing.token == token) {
                    atlas.backing.settle(path, stamp);

                    return Task::none();
                }

                if let Some(pose) = session.pose.as_mut().filter(|pose| pose.backing.token == token) {
                    let saved = pose.backing.settle(path, stamp) == Settled::Saved;
                    let updated = pose.doc.shared();

                    if saved {
                        session.viewer.adopt_model(updated);
                    }

                    session.relist();

                    return Task::none();
                }

                let Some(draft) = session.draft.as_mut().filter(|draft| draft.backing.token == token) else {
                    return Task::none();
                };

                let saved = draft.backing.settle(path, stamp) == Settled::Saved;
                let updated = draft.doc.shared();

                if saved && let Some(showing) = session.viewer.selected_anim().cloned() {
                    session.viewer.adopt_anim(&showing, updated);
                }

                Task::none()
            }
            Message::AddCut => {
                session.remember(Tag::Cuts);

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                let added = atlas.doc.add_cut();

                atlas.backing.dirty = true;
                atlas.restate();

                let task = atlas.persist_if_dirty();

                session.reslice(Some(added));

                task
            }
            Message::Undo => session.undo(),
            Message::OpenManage
            | Message::ManagePopup(_)
            | Message::Manage(_)
            | Message::Export => Task::none(),
        }
    }

    fn chrome(&mut self, message: &Message) -> Option<Task<Message>> {
        match message {
            Message::OpenManage => {
                self.managing = true;
                self.manage.restock();

                Some(Task::none())
            }
            Message::ManagePopup(msg) => {
                if self.popup.update(msg.clone(), manage::SPEC) {
                    self.managing = false;
                }

                Some(Task::none())
            }
            Message::Manage(msg) => Some(self.manage(msg.clone())),
            Message::Switch(mode) => {
                self.mode = *mode;

                None
            }
            Message::Export => {
                let set = self.manage.set().clone();

                match sets::export(&set) {
                    Ok(path) => info!(path = %path.display(), "Studio exported a set"),
                    Err(err) => warn!("Studio could not export the set: {}", err),
                }

                Some(Task::none())
            }
            _ => None,
        }
    }

    fn sealed(&self) -> bool {
        self.manage.sealed()
    }

    fn droppable(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.viewer.selected_anim().is_some())
    }

    fn manage(&mut self, message: manage::Message) -> Task<Message> {
        match message {
            manage::Message::NameChanged(typed) => {
                if self.sealed() {
                    return Task::none();
                }

                self.manage.rename(typed);

                self.manage
                    .renamer
                    .set_after((), Message::Manage(manage::Message::Rename), RENAME_DELAY)
            }
            manage::Message::Rename => {
                self.manage.renamer.expire();
                self.resettle(Swap::Same)
            }
            manage::Message::Import => {
                if self.manage.set().rigged() && !self.manage.importing.take(&()) {
                    return self
                        .manage
                        .importing
                        .set((), Message::Manage(manage::Message::ImportExpired));
                }

                Task::perform(dialog::file("Animation Set", &["png", "imgcut", "mamodel"]), |picked| {
                    Message::Manage(manage::Message::Imported(picked))
                })
            }
            manage::Message::ImportExpired => {
                self.manage.importing.expire();

                Task::none()
            }
            manage::Message::Imported(picked) => {
                let Some(source) = picked else {
                    return Task::none();
                };

                self.manage.adopt(sets::siblings(&source));

                self.resettle(Swap::Fresh)
            }
            manage::Message::New => {
                let name = sets::vacant(self.manage.name());

                match sets::seed(&name) {
                    Ok(set) => self.manage.adopt(set),
                    Err(err) => {
                        warn!(name, "Studio could not seed a new set: {}", err);

                        return Task::none();
                    }
                }

                self.resettle(Swap::Fresh)
            }
            manage::Message::Recall(name) => {
                let picked = match name == manage::NONE_ENTRY {
                    true => sets::Set::default(),
                    false => sets::load(&name),
                };

                self.manage.adopt(picked);

                self.resettle(Swap::Fresh)
            }
            manage::Message::Pick(slot) => {
                if self.manage.set().slot(slot).is_some() && !self.manage.picking.take(&slot) {
                    return self
                        .manage
                        .picking
                        .set(slot, Message::Manage(manage::Message::PickExpired));
                }

                Task::perform(dialog::file(slot.label(), slot.filter()), move |picked| {
                    Message::Manage(manage::Message::Picked(slot, picked))
                })
            }
            manage::Message::PickExpired => {
                self.manage.picking.expire();

                Task::none()
            }
            manage::Message::Picked(slot, picked) => {
                let Some(source) = picked else {
                    return Task::none();
                };

                let seated = self.manage.set().slot(slot).map(Path::to_path_buf);
                let replaceable = seated.as_deref().is_some_and(|held| sets::home(held) != sets::Home::Loose);

                let Some(seated) = seated.filter(|_| replaceable) else {
                    self.manage.place(slot, source);

                    return self.resettle(Swap::Same);
                };

                if source == seated {
                    return Task::none();
                }

                if let Some(session) = self.session.as_mut() {
                    session.remember(Tag::Bulk);
                }

                if let Err(err) = fs::copy(&source, &seated) {
                    warn!(path = %seated.display(), "Studio could not replace the file: {}", err);

                    return Task::none();
                }

                self.reopen()
            }
            manage::Message::AddAnims => {
                Task::perform(dialog::files("Animation", &["maanim"]), |picked| {
                    Message::Manage(manage::Message::AnimsPicked(picked))
                })
            }
            manage::Message::AnimsPicked(picked) => {
                if picked.is_empty() {
                    return Task::none();
                }

                for path in picked {
                    if !self.manage.set().anims.contains(&path) {
                        self.manage.anims_mut().push(path);
                    }
                }

                self.resettle(Swap::Same)
            }
            manage::Message::DropAnim => {
                if !self.manage.dropping.take(&()) {
                    return self.manage.dropping.set((), Message::Manage(manage::Message::DropExpired));
                }

                let Some(held) =
                    self.session.as_ref().and_then(|session| session.viewer.selected_anim().cloned())
                else {
                    return Task::none();
                };

                self.flush_now();

                if sets::home(&held) == sets::Home::Studio
                    && let Err(err) = fs::remove_file(&held)
                {
                    warn!(path = %held.display(), "Studio could not delete the track: {}", err);
                }

                self.manage.anims_mut().retain(|path| *path != held);

                self.resettle(Swap::Same)
            }
            manage::Message::DropExpired => {
                self.manage.dropping.expire();

                Task::none()
            }
        }
    }

    fn resettle(&mut self, swap: Swap) -> Task<Message> {
        self.settle_folder();

        if !self.manage.set().rigged() {
            self.flush_now();
            self.stash();
            self.session = None;

            return Task::none();
        }

        if self.session.as_ref().is_some_and(|session| session.plan.set == *self.manage.set()) {
            return Task::none();
        }

        let wanted = self.manage.set().clone();

        if swap == Swap::Same
            && let Some(session) = self.session.as_mut()
        {
            session.reseat(wanted);
            self.manage.restock();

            return Task::none();
        }

        let target_mod = self.session.as_ref().and_then(|session| session.plan.target_mod.clone());
        let clip = self.session.as_ref().and_then(|session| session.viewer.selected_label());

        self.begin(plan(wanted, target_mod, clip));
        self.manage.restock();

        Task::none()
    }

    fn reopen(&mut self) -> Task<Message> {
        if let Some(session) = self.session.as_mut() {
            session.draft = None;
            session.pose = None;
            session.atlas = None;
            session.reload();
        }

        Task::none()
    }

    fn settle_folder(&mut self) {
        if self.manage.set().files().is_empty() || self.sealed() {
            return;
        }

        self.pull_in();
        self.rename_folder();
    }

    fn pull_in(&mut self) {
        if !sets::pullable(self.manage.set(), self.unlocked) {
            return;
        }

        let name = sets::folder_name(self.manage.set())
            .unwrap_or_else(|| sets::vacant(&sets::sanitize(self.manage.name())));

        match sets::adopt(&name, self.manage.set()) {
            Ok(adopted) => self.manage.adopt(adopted),
            Err(err) => warn!(name, "Studio could not adopt the set: {}", err),
        }
    }

    fn rename_folder(&mut self) {
        let Some(held) = sets::folder_name(self.manage.set()) else {
            return;
        };

        let wanted = sets::sanitize(self.manage.name());

        if wanted == held {
            return;
        }

        let name = sets::vacant(&wanted);

        if let Err(err) = sets::rename(&held, &name) {
            warn!(from = %held, to = %name, "Studio could not rename the set folder: {}", err);

            return;
        }

        self.manage.adopt(sets::load(&name));
    }

    pub(crate) fn view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Element<'a, Message> {
        let body = match self.session.as_ref() {
            Some(session) => session.view(settings, anim),
            None => self.vacant_view(settings, anim),
        };

        stack![body, self.view_notice()].into()
    }


    fn vacant_view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Element<'a, Message> {
        let (side, stage): (Element<'_, Message>, Element<'_, Message>) = match self.mode {
            Mode::Atlas => (
                column![vacant_cuts(), vacant_slices()].spacing(GAP).height(Length::Fill).into(),
                stack![console_card(centred(NO_SET_HINT)), console_edge()].into(),
            ),
            Mode::Entity => (
                column![console_card(centred(NO_SET_NOTICE)), vacant_keys()]
                    .spacing(GAP)
                    .height(Length::Fill)
                    .into(),
                column![
                    container(self.idle.view(settings, anim).map(Message::Viewer))
                        .width(Length::Fill)
                        .height(Length::Fill),
                    strip(Focus::Curve, None, None, None, false, &settings.animation)
                ]
                .spacing(GAP)
                .into(),
            ),
        };

        let body = row![panel_frame(self.mode, false, side), stage].spacing(GAP);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..container::Style::default()
            })
            .into()
    }

    fn view_notice(&self) -> Element<'_, Message> {
        let banner = container(theme::centered_text(self.notice_text.as_str()).size(NOTICE_TEXT_SIZE))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(Padding {
                top: NOTICE_OVERHANG + NOTICE_PAD_Y,
                right: NOTICE_PAD_X,
                bottom: NOTICE_PAD_Y,
                left: NOTICE_PAD_X,
            })
            .style(theme::notice_banner);

        container(slide(banner, self.noticing(), Slide::Up).floating())
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Top)
            .padding(Padding::default().top(-NOTICE_OVERHANG))
            .into()
    }

    fn noticing(&self) -> bool {
        self.notice.is_some_and(|at| at.elapsed() < NOTICE_EXPIRY)
    }

    fn raise(&mut self, text: String) {
        self.notice_text = text;
        self.notice = Some(Instant::now());
    }



    pub(crate) fn managing(&self) -> bool {
        self.managing
    }

    pub(crate) fn manage_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.managing.then(|| {
            self.popup.view(
                manage::TITLE,
                manage::SPEC,
                window,
                Message::ManagePopup,
                || self.manage.view(self.sealed(), self.droppable()).map(Message::Manage),
                None,
            )
        })
    }

    pub(crate) fn export_popup_visible(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.viewer.export_popup_open())
    }

    pub(crate) fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.session
            .as_ref()
            .and_then(|session| session.viewer.export_popup_view(window))
            .map(|view| view.map(Message::Viewer))
    }

    pub(crate) fn expanded(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.viewer.is_expanded())
    }

    pub(crate) fn expanded_view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Option<Element<'a, Message>> {
        self.session
            .as_ref()
            .and_then(|session| session.viewer.expanded_view(settings, anim))
            .map(|view| view.map(Message::Viewer))
    }
}

impl Session {
    fn showing(self) -> Showing {
        Showing {
            viewer: self.viewer,
            draft: self.draft,
            pose: self.pose,
            atlas: self.atlas,
            opened: self.opened,
            posed: self.posed,
            cutting: self.cutting,
            rows: self.rows,
            widest: self.widest,
            listed: self.listed,
        }
    }

    fn sync(&mut self, settings: &Settings, anim: &AnimState) -> Task<Message> {
        let set = self.plan.set.clone();
        let mut priming = Task::none();

        if std::mem::replace(&mut self.primed, true) {
            self.viewer.sync(&self.key, || set.clips(), settings, anim);
        } else {
            priming = self.viewer.preload(&self.key, || set.clips(), anim).map(Message::Viewer);

            if let Some(label) = self.plan.clip.as_deref() {
                self.viewer.select_label(label);
            }
        }

        let selected = self.viewer.selected_anim().cloned();

        if selected == self.opened && !self.draft.as_ref().is_some_and(|draft| draft.backing.drifted()) {
            if self.listed != self.viewer.loaded_rig() {
                self.relist();
            }

            return priming;
        }

        let wanted = self
            .draft
            .as_ref()
            .and_then(|draft| held_curve(&draft.doc, draft.track?))
            .or_else(|| self.wanted.take());

        self.wanted = None;
        self.opened = selected.clone();
        self.draft = selected.as_deref().and_then(Draft::load);

        if let Some(wanted) = wanted {
            self.recover(wanted);
        }

        self.aim();
        self.relist();

        priming
    }

    fn reseat(&mut self, set: sets::Set) {
        let rigged = |set: &sets::Set| (set.sheet.clone(), set.cuts.clone(), set.model.clone());
        let moved = rigged(&self.plan.set) != rigged(&set);

        self.plan.set = set;
        self.key = self.plan.set.key();

        if !moved {
            return;
        }

        self.wanted = self.draft.as_ref().and_then(|draft| held_curve(&draft.doc, draft.track?));
        self.wanted_part = self.pose.as_ref().and_then(|pose| pose.part);
        self.plan.clip = self.viewer.selected_label();

        self.history.clear();
        self.primed = false;

        self.opened = None;
        self.posed = None;
        self.cutting = None;
    }

    fn select(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };

        let (part, track, bucket) = (row.part, row.track, row.bucket);

        if bucket {
            self.loose_open = !self.loose_open;
            self.relist();

            return;
        }

        if let Some(track) = track {
            self.focus = Focus::Curve;

            if let Some(draft) = self.draft.as_mut() {
                draft.retrack(track);
            }

            self.aim();

            return;
        }

        let Some(part) = part else {
            return;
        };

        let held =
            self.focus == Focus::Part && self.pose.as_ref().is_some_and(|pose| pose.part == Some(part));

        self.focus = Focus::Part;

        if let Some(pose) = self.pose.as_mut() {
            pose.pick(part);
        }

        self.aim();

        if held && !self.expanded.remove(&part) {
            self.expanded.insert(part);
        }

        self.relist();
    }

    fn recover(&mut self, wanted: Held) {
        let found = self.draft.as_ref().and_then(|draft| locate_curve(&draft.doc, &wanted));

        if let Some(at) = found
            && let Some(draft) = self.draft.as_mut()
        {
            draft.retrack(at);
            self.focus = Focus::Curve;

            return;
        }

        let Ok(part) = usize::try_from(wanted.part) else {
            return;
        };

        if let Some(pose) = self.pose.as_mut().filter(|pose| part < pose.doc.count()) {
            pose.pick(part);
            self.focus = Focus::Part;
        }
    }

    fn mount(&self) -> String {
        State::mount_of(&self.plan)
    }

    fn remember(&mut self, tag: Tag) {
        let anchor = match tag.subject() {
            history::Subject::Anim => self.draft.as_ref().map(|draft| draft.backing.read_from.clone()),
            history::Subject::Model => self.pose.as_ref().map(|pose| pose.backing.read_from.clone()),
            history::Subject::Cuts => self.atlas.as_ref().map(|atlas| atlas.backing.read_from.clone()),
            history::Subject::Rig => None,
        };

        if !self.history.wanted(tag, anchor.as_deref()) {
            return;
        }

        let shot = match (tag.subject(), anchor) {
            (history::Subject::Anim, Some(path)) => {
                self.draft.as_ref().map(|draft| history::Shot::Anim(path, draft.doc.clone()))
            }
            (history::Subject::Model, Some(path)) => {
                self.pose.as_ref().map(|pose| history::Shot::Model(path, pose.doc.clone()))
            }
            (history::Subject::Cuts, Some(path)) => {
                self.atlas.as_ref().map(|atlas| history::Shot::Cuts(path, atlas.doc.clone()))
            }
            (history::Subject::Rig, _) => {
                self.flush();

                Some(history::Shot::Rig(self.bytes()))
            }
            _ => None,
        };

        if let Some(shot) = shot {
            self.history.push(tag, shot);
        }
    }

    fn bytes(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.plan
            .set
            .files()
            .into_iter()
            .filter_map(|path| fs::read(&path).ok().map(|body| (path, body)))
            .collect()
    }

    fn undo(&mut self) -> Task<Message> {
        let Some(shot) = self.history.pop() else {
            return Task::none();
        };

        match shot {
            history::Shot::Anim(path, doc) => {
                if let Some(draft) = self.draft.as_mut().filter(|draft| draft.backing.read_from == path) {
                    draft.doc = doc;
                    draft.retrack_clamped();
                    draft.backing.dirty = true;

                    let task = draft.persist_if_dirty();
                    self.relist();

                    return task;
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Model(path, doc) => {
                if let Some(pose) = self.pose.as_mut().filter(|pose| pose.backing.read_from == path) {
                    pose.doc = doc;
                    pose.reseat();
                    pose.restate();
                    pose.backing.dirty = true;

                    let task = pose.persist_if_dirty();

                    self.settle_pose();

                    return task;
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Cuts(path, doc) => {
                if let Some(atlas) = self.atlas.as_mut().filter(|atlas| atlas.backing.read_from == path) {
                    atlas.doc = doc;
                    atlas.restate();
                    atlas.backing.dirty = true;

                    return atlas.persist_if_dirty();
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Rig(files) => {
                self.flush();

                for (path, body) in files {
                    self.rewrite(&path, &body);
                }

                self.draft = None;
                self.pose = None;
                self.atlas = None;
            }
        }

        self.reload();

        Task::none()
    }

    fn rewrite(&self, path: &Path, body: &[u8]) {
        let Some(stamp) = preview::stamp(path) else {
            warn!(path = %path.display(), "Studio could not stamp a file it is undoing");

            return;
        };

        write_now(path, body, stamp);
    }

    fn flush(&mut self) {
        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        if let Some(pose) = self.pose.as_mut() {
            pose.persist_now();
        }

        if let Some(atlas) = self.atlas.as_mut() {
            atlas.persist_now();
        }
    }

    fn repose(&mut self) {
        let Some(path) = self.viewer.selected_model().map(Path::to_path_buf) else {
            return;
        };

        let stale = Some(&path) != self.posed.as_ref()
            || self.pose.as_ref().is_some_and(|pose| pose.backing.drifted());

        if !stale {
            return;
        }

        let path = Some(path);

        let held = self.pose.as_ref().and_then(|pose| pose.part).or(self.wanted_part.take());

        self.posed = path.clone();
        self.pose = path.as_deref().and_then(Pose::load);

        if let Some(pose) = self.pose.as_mut()
            && let Some(at) = held.filter(|at| *at < pose.doc.count())
        {
            pose.pick(at);
        }

        self.aim();
        self.relist();
    }

    fn ghost(&self, part: usize, at: Point) -> Element<'_, Message> {
        let label = self
            .rows
            .iter()
            .find(|row| row.part == Some(part))
            .map_or("", |row| row.label.as_str());

        let carried = text(label)
            .font(Font::MONOSPACE)
            .size(TREE_TEXT_SIZE)
            .wrapping(text::Wrapping::None)
            .style(|theme: &Theme| text::Style {
                color: Some(Color { a: GHOST_INK, ..theme.palette().text }),
            });

        let card = container(carried).padding([GHOST_PAD, GHOST_PAD * 2.0]).style(|theme: &Theme| {
            let palette = theme.palette();

            container::Style {
                background: Some(Color { a: GHOST_FILL, ..palette.primary }.into()),
                border: Border::default()
                    .rounded(4.0)
                    .width(1.0)
                    .color(Color { a: GHOST_EDGE, ..palette.primary }),
                ..container::Style::default()
            }
        });

        let width = CHAR_WIDTH * glyphs::columns(label) + GHOST_PAD * 4.0;
        let height = TREE_TEXT_SIZE + GHOST_PAD * 2.0;

        let placed = Padding::default()
            .top((at.y - height / 2.0).max(0.0))
            .left((at.x - width / 2.0).max(0.0));

        container(card).padding(placed).width(Length::Fill).height(Length::Fill).into()
    }

    fn landing(&self, part: usize, at: Point) -> Option<Landing> {
        if at.x < TREE_LEFT || at.x > TREE_RIGHT || at.y < TREE_TOP || self.rows.is_empty() {
            return None;
        }

        let travelled = at.y - TREE_TOP + self.scroll;

        if travelled < 0.0 {
            return None;
        }

        let row = ((travelled / ROW_HEIGHT).floor() as usize).min(self.rows.len() - 1);
        let within = (travelled - row as f32 * ROW_HEIGHT) / ROW_HEIGHT;

        let landing = match within {
            _ if (NEST_BAND..1.0 - NEST_BAND).contains(&within) => Landing::Onto(row),
            _ if within < NEST_BAND => Landing::Seam(row),
            _ => Landing::Seam(row + 1),
        };

        self.settle(part, landing).map(|_| landing)
    }

    fn settle(&self, part: usize, onto: Landing) -> Option<Option<usize>> {
        let pose = self.pose.as_ref()?;

        let parent = match onto {
            Landing::Onto(row) => Some(self.rows.get(row)?.part?),
            Landing::Seam(gap) => self.seam(gap, pose),
        };

        match parent.filter(|anchor| pose.doc.descends(*anchor, part)) {
            Some(_) => None,
            None => Some(parent),
        }
    }

    fn seam(&self, gap: usize, pose: &Pose) -> Option<usize> {
        let above = gap.checked_sub(1).and_then(|row| self.rows.get(row)).and_then(|row| row.part)?;
        let below = self.rows.get(gap).and_then(|row| row.part);

        match below.and_then(|below| pose.doc.parent(below)) {
            Some(parent) if parent == above => Some(above),
            _ => pose.doc.parent(above),
        }
    }

    fn land(&mut self, part: usize, onto: Landing) {
        let Some(parent) = self.settle(part, onto) else {
            return;
        };

        if self.pose.as_ref().is_none_or(|pose| pose.doc.parent(part) == parent) {
            return;
        }

        self.remember(Tag::Parts);

        let Some(pose) = self.pose.as_mut() else {
            return;
        };

        if !pose.doc.reparent(part, parent) {
            return;
        }

        pose.backing.dirty = true;

        pose.persist_now();

        if let Some(parent) = parent {
            self.expanded.insert(parent);
        }

        if let Some(pose) = self.pose.as_mut() {
            pose.pick(part);
        }

        self.settle_pose();
    }

    fn settle_pose(&mut self) {
        if let Some(model) = self.pose.as_ref().map(|pose| pose.doc.shared()) {
            self.viewer.adopt_model(model);
        }

        self.aim();
        self.relist();
    }

    fn restructure(&mut self, moved: Vec<Option<usize>>) {
        let Some(pose) = self.pose.as_mut() else {
            return;
        };

        pose.backing.dirty = true;
        pose.persist_now();

        self.wanted_part = self
            .pose
            .as_ref()
            .and_then(|pose| pose.part)
            .and_then(|at| moved.get(at).copied().flatten());

        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        for path in self.viewer.anim_paths() {
            retarget_file(&path, &moved);
        }

        self.expanded = self
            .expanded
            .iter()
            .filter_map(|at| moved.get(*at).copied().flatten())
            .collect();

        self.reload();
    }

    fn reload(&mut self) {
        self.opened = None;
        self.posed = None;
        self.cutting = None;
        self.wanted = None;
        self.viewer.invalidate_paths();
        self.repose();
    }

    fn recarve(&mut self) -> Task<Message> {
        if self.mode != Mode::Atlas || self.carving {
            return Task::none();
        }

        let Some(path) = self.viewer.selected_cuts().map(Path::to_path_buf) else {
            return Task::none();
        };

        let stale = Some(&path) != self.cutting.as_ref()
            || self.atlas.as_ref().is_some_and(|atlas| atlas.backing.drifted());

        if !stale {
            return Task::none();
        }

        self.cutting = Some(path.clone());

        let opened = Backing::open(&path);
        let art = self.viewer.selected_sheet().map(fs::read).and_then(Result::ok);

        let Some(((backing, bytes), art)) = opened.zip(art) else {
            self.atlas = None;

            return Task::none();
        };

        self.carving = true;

        Task::perform(
            smol::unblock(move || Sheet::assemble(backing, bytes, art).map(Arc::new)),
            Message::Carved,
        )
    }

    fn reslice(&mut self, at: Option<usize>) {
        let held = self.slice;
        self.slice = at.filter(|at| held != Some(*at));

        self.redraw();
    }

    fn redraw(&mut self) {
        let (picked, hidden) = (self.slice, self.framing);

        if let Some(atlas) = self.atlas.as_mut() {
            atlas.picked = picked;
            atlas.hidden = hidden;
            atlas.restate();
        }
    }

    fn recut(&mut self, moved: Vec<Option<usize>>) {
        let Some(atlas) = self.atlas.as_mut() else {
            return;
        };

        atlas.backing.dirty = true;
        atlas.picked = None;
        atlas.hidden = None;
        atlas.restate();
        atlas.persist_now();

        if let Some(pose) = self.pose.as_mut()
            && pose.doc.retarget_sprites(&moved)
        {
            pose.backing.dirty = true;
            pose.persist_now();
        }

        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        for path in self.viewer.anim_paths() {
            revalue_file(&path, &moved);
        }

        self.framing = None;
        self.reload();
    }

    fn chosen(&self) -> Option<usize> {
        match (self.mode, self.focus) {
            (Mode::Atlas, _) => None,
            (_, Focus::Part) => self.pose.as_ref().and_then(|pose| pose.part),
            (_, Focus::Curve) => {
                self.draft.as_ref().and_then(Draft::part).and_then(|part| usize::try_from(part).ok())
            }
        }
    }

    fn relist(&mut self) {
        if !self.rows.is_empty() && self.viewer.loaded_rig() != self.plan.set.rig_id() {
            return;
        }

        self.seed();

        let tracks = match self.mode {
            Mode::Entity => self.draft.as_ref().map(|draft| &draft.doc),
            Mode::Atlas => None,
        };

        let listed =
            listing(tracks, self.viewer.rig().map(|rig| &rig.model), &self.expanded, self.loose_open);

        self.listed = self.viewer.loaded_rig().to_owned();
        self.widest = listed.iter().map(TreeRow::span).fold(0.0, f32::max);
        self.rows = listed;
    }

    fn seed(&mut self) {
        if self.seeded {
            return;
        }

        let Some(model) = self.viewer.rig().map(|rig| &rig.model) else {
            return;
        };

        self.seeded = true;

        if let [only] = roots(model).as_slice() {
            self.expanded.insert(*only);
        }
    }

    fn aim(&mut self) {
        let cuts = self.viewer.rig().map_or(0, |rig| rig.sheet.cuts.len());

        if let Some(pose) = self.pose.as_mut() {
            pose.cuts = cuts;
        }

        let part = self.chosen();
        let kind = self.draft.as_ref().and_then(|draft| draft.curve()).map(|track| track.kind);

        let neutral = match (part, kind) {
            (Some(part), Some(kind)) => {
                authoring::neutral_value(kind, part, self.viewer.rig().map(|rig| &rig.model))
            }
            _ => 0,
        };

        if let Some(draft) = self.draft.as_mut() {
            draft.neutral = neutral;
            draft.hint = neutral.to_string();
            draft.restate();
        }

        self.viewer.set_highlight(part);
    }

    fn view<'a>(&'a self, settings: &'a Settings, anim: &'a AnimState) -> Element<'a, Message> {
        let showing: Element<'_, Message> = if self.viewer.resolved() {
            self.viewer.view(settings, anim).map(Message::Viewer)
        } else {
            container(text(LOADING_NOTICE).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let stage = container(showing).width(Length::Fill).height(Length::Fill);

        let right: Element<'_, Message> = match self.mode {
            Mode::Atlas => self.canvas(),
            _ => column![stage, self.strip(settings)].spacing(GAP).into(),
        };

        let body = row![self.side(), right].spacing(GAP);

        let content = container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                ..container::Style::default()
            });

        let mut layers = stack![content];

        if self.drag != Drag::Idle {
            let carried = self.drag.carrying();
            let ghost: Element<'_, Message> = match self.drag {
                Drag::Moving { part, at, .. } => self.ghost(part, at),
                _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
            };

            layers = layers.push(
                mouse_area(ghost)
                    .interaction(match carried.is_some() {
                        true => mouse::Interaction::Grabbing,
                        false => mouse::Interaction::Grab,
                    })
                    .on_move(Message::DragMove)
                    .on_release(Message::DragEnd),
            );
        }

        layers.into()
    }

    fn side(&self) -> Element<'_, Message> {
        let body = match (self.mode, self.focus) {
            (Mode::Atlas, _) => column![self.loaders(), self.cuts()],
            (Mode::Entity, Focus::Curve) => column![self.tree(), self.keys()],
            (Mode::Entity, Focus::Part) => column![self.tree(), self.fields()],
        };

        panel_frame(self.mode, true, body.spacing(GAP).height(Length::Fill).into())
    }

    fn canvas(&self) -> Element<'_, Message> {
        let showing: Element<'_, Message> = match self.atlas.as_ref() {
            Some(atlas) => self
                .picture
                .view_outlined(&atlas.source, &atlas.outlines, self.framing.is_some())
                .map(Message::Picture),
            None => centred(NO_ATLAS_NOTICE),
        };

        let framing = self.framing.is_some();

        stack![
            editor::suppress(console_card(showing), framing),
            overlay::hint(FRAME_HINT, framing),
            console_edge()
        ]
        .into()
    }

    fn loaders(&self) -> Element<'_, Message> {
        let sheet = self.viewer.selected_sheet().and_then(named).unwrap_or_default();
        let cuts = self.viewer.selected_cuts().and_then(named).unwrap_or_default();
        let atlas = self.atlas.as_ref();

        let size = self
            .viewer
            .rig()
            .and_then(|rig| rig.sheet.image_data.as_ref())
            .map_or_else(String::new, |art| format!("{} \u{00d7} {}", art.width(), art.height()));

        let facts = [
            ("Sheet", sheet),
            ("Cuts", cuts),
            ("Size", size),
            ("Regions", atlas.map_or_else(String::new, |atlas| atlas.doc.count().to_string())),
        ];

        facts_card(facts)
    }

    fn cuts(&self) -> Element<'_, Message> {
        let table = container(
            container(responsive(move |size: Size| self.view_cuts(size)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let notice: Element<'_, Message> =
            match self.atlas.as_ref().is_some_and(|atlas| atlas.backing.failed) {
                true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
                false => Space::new().height(Length::Fixed(0.0)).into(),
            };

        let add = footer_button("Add Cut", self.atlas.as_ref().map(|_| Message::AddCut));

        column![notice, table, add].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_cuts(&self, size: Size) -> Element<'_, Message> {
        let atlas = self.atlas.as_ref();
        let rows = atlas.map_or(0, |atlas| atlas.inputs.len());

        if rows == 0 {
            let blank = container(theme::centered_text(NO_CUTS_NOTICE).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return column![cuts_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();
        }

        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let tail = if rows as f32 * KEY_ROW_HEIGHT > body { SCROLLBAR_ALLOWANCE } else { 0.0 };
        let width = (size.width - tail).max(0.0);

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(rows, body, self.strip_scroll, KEY_ROW_HEIGHT, 0.0);

        let mut list = Column::with_capacity(range.len() + 2).width(Length::Fixed(width));

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for at in range {
            if let Some(atlas) = atlas {
                let framing = self.framing == Some(at);

                let armed = self.slicing.armed_for(&at);

                list = list.push(atlas.cut_row(at, framing, armed, self.slice == Some(at), width));
            }
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        let scrolled = smooth_scroll(
            scrollable(list)
                .id(self.strip_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .on_scroll(|viewport| Message::StripScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        column![cuts_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into()
    }

    fn tree(&self) -> Element<'_, Message> {
        let by_part = self.focus == Focus::Part;
        let picked = match by_part {
            true => self.pose.as_ref().and_then(|pose| pose.part),
            false => self.draft.as_ref().and_then(|draft| draft.track),
        };

        let dragged = self.drag.carrying();
        let landing = self.drag.landing();
        let body = responsive(move |size: Size| self.view_rows(size, picked, by_part, dragged, landing));

        container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::mock_console_container)
            .into()
    }

    fn view_rows(
        &self,
        size: Size,
        picked: Option<usize>,
        by_part: bool,
        dragged: Option<usize>,
        landing: Option<Landing>,
    ) -> Element<'_, Message> {
        let tail = if self.widest > size.width { SCROLL_TAIL } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } = row_window::compute_with(
            self.rows.len(),
            size.height - tail,
            self.scroll,
            ROW_HEIGHT,
            ROW_SPACING,
        );

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let mut list = Column::with_capacity(range.len() + 3).spacing(ROW_SPACING);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for index in range {
            let Some(row) = self.rows.get(index) else {
                continue;
            };

            let carried = dragged.is_some_and(|part| row.part == Some(part));
            let onto = landing.and_then(|landing| landing.mark(index, self.rows.len() - 1));

            list = list.push(row.view(index, picked, by_part, carried, onto, width));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if tail > 0.0 {
            list = list.push(space().height(Length::Fixed(tail)));
        }

        smooth_scroll(
            scrollable(list)
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
    }

    fn strip<'a>(&'a self, settings: &'a Settings) -> Element<'a, Message> {
        let index = self.chosen().and_then(|at| i32::try_from(at).ok());

        let part = index
            .and_then(|index| usize::try_from(index).ok())
            .zip(self.viewer.rig())
            .and_then(|(at, rig)| rig.model.parts.get(at));

        strip(self.focus, index, part, self.viewer.rig(), self.viewer.locatable(), &settings.animation)
    }

    fn keys(&self) -> Element<'_, Message> {
        let draft = self.draft.as_ref();

        let add = footer_button("Add Keyframe", draft.map(|_| Message::AddKey));

        let active = draft
            .and_then(|draft| draft.curve())
            .and_then(|track| timeline::playhead(track, self.viewer.frame()))
            .map(|playhead| playhead.key);

        let notice: Element<'_, Message> = match draft.is_some_and(|draft| draft.backing.failed) {
            true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
            false => Space::new().height(Length::Fixed(0.0)).into(),
        };

        let table = container(
            container(responsive(move |size: Size| self.view_keys(size, active)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let footer: Element<'_, Message> = match draft {
            Some(draft) => row![draft.looping(), add].spacing(GAP).align_y(Vertical::Center).into(),
            None => add,
        };

        column![notice, table, footer].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_keys(&self, size: Size, active: Option<usize>) -> Element<'_, Message> {
        let draft = self.draft.as_ref();
        let rows = draft.map_or(0, |draft| draft.inputs.len());
        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let spans = rows as f32 * KEY_ROW_HEIGHT;
        let tail = if spans > body { SCROLLBAR_ALLOWANCE } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(rows, body, self.strip_scroll, KEY_ROW_HEIGHT, 0.0);

        let width = (size.width - tail).max(0.0);
        let mut list = Column::with_capacity(range.len() + 2).width(Length::Fixed(width));

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for at in range {
            if let Some(draft) = draft {
                let armed = self.confirm.armed_for(&at);

                list = list.push(draft.key_row(at, active == Some(at), at, armed, width));
            }
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if rows == 0 {
            let blank = container(theme::centered_text(self.vacancy()).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return column![keys_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();
        }

        let scrolled = smooth_scroll(
            scrollable(list)
                .id(self.strip_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .on_scroll(|viewport| Message::StripScrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        column![keys_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into()
    }

    fn fields(&self) -> Element<'_, Message> {
        let pose = self.pose.as_ref();

        let add = footer_button("Add Part", pose.map(|_| Message::AddPart));

        let notice: Element<'_, Message> = match pose.is_some_and(|pose| pose.backing.failed) {
            true => text(WRITE_FAILED_NOTICE).size(LABEL_SIZE).style(text::danger).into(),
            false => Space::new().height(Length::Fixed(0.0)).into(),
        };

        let table = container(
            container(responsive(move |size: Size| self.view_fields(size)))
                .padding(theme::CONSOLE_BORDER_WIDTH),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container);

        let footer: Element<'_, Message> = match pose {
            Some(pose) => row![pose.aligning(), add].spacing(GAP).align_y(Vertical::Center).into(),
            None => add,
        };

        column![notice, table, footer].spacing(ROW_GAP).height(Length::Fill).into()
    }

    fn view_fields(&self, size: Size) -> Element<'_, Message> {
        let pose = self.pose.as_ref();
        let rows = pose.map_or(0, |pose| pose.inputs.len());

        if rows == 0 {
            let blank = container(theme::centered_text(self.absence()).size(LABEL_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return column![fields_header(0.0), blank].width(Length::Fill).height(Length::Fill).into();
        }

        let body = (size.height - KEY_HEAD_HEIGHT).max(0.0);
        let tail = if rows as f32 * FIELD_ROW_HEIGHT > body { SCROLLBAR_ALLOWANCE } else { 0.0 };
        let width = (size.width - tail).max(0.0);

        let listed = (0..rows).fold(Column::with_capacity(rows).width(Length::Fixed(width)), |listed, at| {
            match pose {
                Some(pose) => listed.push(pose.field_row(at, width)),
                None => listed,
            }
        });

        let scrolled = smooth_scroll(
            scrollable(listed)
                .id(self.fields_id.clone())
                .direction(scrollable::Direction::Vertical(bar()))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        column![fields_header(tail), scrolled].width(Length::Fill).height(Length::Fill).into()
    }

    fn absence(&self) -> &'static str {
        match self.pose.is_some() {
            true => NO_PART_CHOSEN,
            false => NO_MODEL_NOTICE,
        }
    }

    fn vacancy(&self) -> &'static str {
        let holding = self.draft.as_ref().map(|draft| draft.track.is_some());

        match (holding, self.opened.is_some()) {
            (Some(true), _) => EMPTY_TRACK_NOTICE,
            (Some(false), _) => NO_CURVE_CHOSEN,
            (None, true) => UNREADABLE_NOTICE,
            (None, false) => NO_CLIP_NOTICE,
        }
    }
}


struct TreeRow {
    label: String,
    depth: u16,
    mark: &'static str,
    part: Option<usize>,
    owner: Option<usize>,
    track: Option<usize>,
    warn: bool,
    bucket: bool,
}

impl TreeRow {
    fn span(&self) -> f32 {
        ROW_PADDING * 2.0
            + MARKER_WIDTH
            + INDENT * f32::from(self.depth)
            + CHAR_WIDTH * glyphs::columns(&self.label)
    }

    fn view(
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

fn keys_header<'a>(tail: f32) -> Element<'a, Message> {
    let row = Field::TYPED
        .iter()
        .fold(
            row![theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH))].spacing(ROW_GAP),
            |header, field| header.push(theme::centered_text(field.label()).size(LABEL_SIZE).width(Length::Fill)),
        )
        .push(theme::centered_text("Curve").size(LABEL_SIZE).width(Length::Fixed(EASE_WIDTH)))
        .push(theme::centered_text("Action").size(LABEL_SIZE).width(Length::Fixed(STEP_WIDTH)))
        .push(theme::centered_text(CLOSE_LABEL).size(CELL_SIZE).width(Length::Fixed(DROP_WIDTH)));

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(row.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
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

fn adrift_input(theme: &Theme, status: text_input::Status, adrift: bool) -> text_input::Style {
    let style = theme::rounded_input(theme, status);

    if !adrift {
        return style;
    }

    let palette = theme.palette();
    let iced::Background::Color(base) = style.background else {
        return style;
    };

    let blend = |base: f32, over: f32| base * (1.0 - ADRIFT_TINT) + over * ADRIFT_TINT;
    let tinted = Color {
        r: blend(base.r, palette.danger.r),
        g: blend(base.g, palette.danger.g),
        b: blend(base.b, palette.danger.b),
        a: 1.0,
    };

    text_input::Style { background: tinted.into(), border: style.border.color(palette.danger), ..style }
}

fn tip<'a>(content: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let banner = container(text(label).size(LABEL_SIZE))
        .padding(CELL_PADDING)
        .style(container::bordered_box);

    tooltip(content, banner, tooltip::Position::Top).into()
}

fn console_edge<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| container::Style {
            border: theme::mock_console_container(theme).border,
            ..container::Style::default()
        })
        .into()
}

fn cuts_header<'a>(tail: f32) -> Element<'a, Message> {
    let head = CUT_FIELDS.iter().enumerate().fold(
        row![theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH))].spacing(ROW_GAP),
        |listed, (cell, label)| {
            let span = match cell == CUT_NAME_FIELD {
                true => Length::Fill,
                false => Length::Fixed(CUT_CELL_WIDTH),
            };

            listed.push(theme::centered_text(*label).size(LABEL_SIZE).width(span))
        },
    );

    let head = head
        .push(theme::centered_text("Action").size(LABEL_SIZE).width(Length::Fixed(CUT_STEP_WIDTH * 2.0 + 2.0)))
        .push(theme::centered_text(CLOSE_LABEL).size(CELL_SIZE).width(Length::Fixed(DROP_WIDTH)));

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(head.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
}

fn fields_header<'a>(tail: f32) -> Element<'a, Message> {
    let row = row![
        theme::centered_text("#").size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH)),
        theme::centered_text("Field").size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)),
        theme::centered_text("Value").size(LABEL_SIZE).width(Length::Fill),
    ]
    .spacing(ROW_GAP);

    let inset = Padding { top: 0.0, right: KEY_ROW_INSET + tail, bottom: 0.0, left: KEY_ROW_INSET };

    container(row.width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(inset)
        .style(theme::zebra_table_header)
        .into()
}

fn bar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new()
        .width(SCROLLBAR_WIDTH)
        .scroller_width(SCROLLBAR_WIDTH)
        .margin(SCROLLBAR_MARGIN)
}

fn both_ways() -> scrollable::Direction {
    scrollable::Direction::Both { vertical: bar(), horizontal: bar() }
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

fn listing(
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

fn roots(model: &Model) -> Vec<usize> {
    lineage(model).1
}

fn rows(model: &Model, expanded: &HashSet<usize>) -> Vec<(usize, usize)> {
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

fn hidden(part: &ModelPart) -> Option<&'static str> {
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

fn fact<'a>(label: &'a str, value: Element<'a, Message>, stripe: usize) -> Element<'a, Message> {
    let body = row![text(label).size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)), value]
        .spacing(ROW_GAP)
        .align_y(Vertical::Center);

    container(body)
        .width(Length::Fill)
        .padding([FACT_ROW_PAD, 6.0])
        .style(move |theme: &Theme| theme::zebra_table_row(theme, stripe))
        .into()
}

fn held_curve(doc: &Maanim, at: usize) -> Option<Held> {
    let track = doc.track(at)?;
    let ordinal = doc
        .tracks()
        .iter()
        .take(at)
        .filter(|other| other.part == track.part && other.kind == track.kind)
        .count();

    Some(Held { part: track.part, kind: track.kind, ordinal })
}

fn locate_curve(doc: &Maanim, held: &Held) -> Option<usize> {
    doc.tracks()
        .iter()
        .enumerate()
        .filter(|(_, track)| track.part == held.part && track.kind == held.kind)
        .nth(held.ordinal)
        .map(|(at, _)| at)
}

fn span_of(keys: &[Keyframe], at: usize) -> Option<(i32, i32)> {
    let here = keys.get(at)?.frame;
    let end = keys.get(at + 1).map_or(here, |next| next.frame.saturating_sub(1).max(here));

    Some((here, end))
}

fn bears(model: &Model, part: usize) -> bool {
    let wanted = i32::try_from(part).ok();

    model.parts.iter().enumerate().any(|(at, other)| at != part && Some(other.parent) == wanted)
}

fn model_rows<'a>(index: Option<i32>, part: Option<&ModelPart>) -> Element<'a, Message> {
    let named = match (index, part) {
        (Some(index), Some(part)) => match part.name.trim() {
            "" => index.to_string(),
            name => format!("{} \u{00b7} {}", index, name),
        },
        (Some(index), None) => format!("{} \u{00b7} {}", index, NO_PART_NOTICE),
        (None, _) => String::new(),
    };

    let pair = |left: i32, right: i32| format!("{}, {}", left, right);
    let facts = [
        ("Part", named),
        ("Parent", part.map_or_else(String::new, |part| part.parent.to_string())),
        ("Sprite", part.map_or_else(String::new, |part| part.sprite.to_string())),
        ("Z Order", part.map_or_else(String::new, |part| part.z.to_string())),
        ("Offset", part.map_or_else(String::new, |part| pair(part.x, part.y))),
        ("Pivot", part.map_or_else(String::new, |part| pair(part.pivot_x, part.pivot_y))),
        ("Scale", part.map_or_else(String::new, |part| pair(part.scale_x, part.scale_y))),
        ("Opacity", part.map_or_else(String::new, |part| part.opacity.to_string())),
    ];

    facts
        .into_iter()
        .enumerate()
        .fold(column![].width(Length::Fill), |listed, (stripe, (label, value))| {
            let cell = text(value).size(LABEL_SIZE).width(Length::Fill).into();

            listed.push(fact(label, cell, stripe))
        })
        .into()
}

fn atlas_rows<'a>(index: Option<i32>, part: Option<&ModelPart>, rig: Option<&Rig>) -> Element<'a, Message> {
    let sheet = rig.map(|rig| &rig.sheet);
    let cuts = sheet.map_or(0, |sheet| sheet.cuts.len());
    let at = part.and_then(|part| usize::try_from(part.sprite).ok()).filter(|at| *at < cuts);
    let cut = at.zip(sheet).and_then(|(at, sheet)| sheet.cuts.get(at));
    let opaque = at.zip(sheet).and_then(|(at, sheet)| sheet.opaque.get(at).copied().flatten());

    let named = match (index, part) {
        (Some(index), Some(part)) => match part.name.trim() {
            "" => index.to_string(),
            name => format!("{} \u{00b7} {}", index, name),
        },
        (Some(index), None) => format!("{} \u{00b7} {}", index, LOST_PART_NOTICE),
        (None, _) => String::new(),
    };

    let sprite = match (part, at) {
        (None, _) => String::new(),
        (Some(_), Some(at)) => format!("{} of {}", at, cuts),
        (Some(part), None) if part.sprite < 0 => NO_SPRITE_NOTICE.to_owned(),
        (Some(part), None) => format!("{} \u{00b7} {}", part.sprite, PAST_ATLAS_NOTICE),
    };

    let span = |width: i32, height: i32| format!("{} \u{00d7} {}", width, height);
    let facts = [
        ("Part", named),
        ("Sprite", sprite),
        ("Cut", cut.map_or_else(String::new, |cut| format!("{}, {}", cut.x, cut.y))),
        ("Size", cut.map_or_else(String::new, |cut| span(cut.width, cut.height))),
        ("Drawn", opaque.map_or_else(|| cut.map_or_else(String::new, |_| BLANK_CUT_NOTICE.to_owned()), |seen| span(seen.width, seen.height))),
        ("Margin", margin(cut, opaque)),
        ("Region", cut.map_or_else(String::new, |cut| named_cut(&cut.name))),
        ("File", sheet.map_or_else(String::new, |sheet| sheet.image_name.clone())),
    ];

    facts
        .into_iter()
        .enumerate()
        .fold(column![].width(Length::Fill), |listed, (stripe, (label, value))| {
            let cell = text(value).size(LABEL_SIZE).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph).into();

            listed.push(fact(label, cell, stripe))
        })
        .into()
}

fn named_cut(name: &str) -> String {
    match name.trim() {
        "" => UNNAMED_CUT_NOTICE.to_owned(),
        named => named.to_owned(),
    }
}

fn margin(cut: Option<&SpriteCut>, opaque: Option<Opaque>) -> String {
    let Some(cut) = cut else {
        return String::new();
    };

    let Some(seen) = opaque else {
        return WHOLE_CUT_NOTICE.to_owned();
    };

    let left = seen.x.saturating_sub(cut.x);
    let top = seen.y.saturating_sub(cut.y);
    let right = cut.width.saturating_sub(left).saturating_sub(seen.width);
    let bottom = cut.height.saturating_sub(top).saturating_sub(seen.height);

    format!("{}, {}, {}, {}", left, top, right, bottom)
}

fn fact_header<'a>(focus: Focus) -> Element<'a, Message> {
    let label = match focus {
        Focus::Curve => "Model",
        Focus::Part => "Atlas",
    };

    container(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .style(theme::zebra_table_header)
        .into()
}

fn panel_head<'a>(label: &'a str) -> Element<'a, Message> {
    container(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .style(theme::zebra_table_header)
        .into()
}

fn options(anim: &AnimSettings) -> Element<'_, Message> {
    let header = panel_head("Option");

    Lens::ALL
        .iter()
        .enumerate()
        .fold(column![header], |listed, (stripe, lens)| {
            let on = lens.on(anim);

            let toggle = button(theme::centered_text(lens.label()).size(LABEL_SIZE).width(Length::Fill))
                .width(Length::Fill)
                .padding([1, 4])
                .on_press(Message::Overlay(*lens))
                .style(move |theme: &Theme, status| theme::toggle_button(theme, status, on));

            listed.push(
                container(toggle)
                    .height(Length::Fixed(FACT_ROW_HEIGHT))
                    .align_y(Vertical::Center)
                    .padding([0, 3])
                    .style(move |theme: &Theme| theme::zebra_table_row(theme, stripe)),
            )
        })
        .width(Length::Fixed(DEBUG_WIDTH))
        .into()
}


fn console_card<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container)
        .into()
}

fn centred<'a>(notice: &'a str) -> Element<'a, Message> {
    container(theme::centered_text(notice).size(LABEL_SIZE))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn facts_card<'a>(facts: [(&'a str, String); 4]) -> Element<'a, Message> {
    let listed = facts.into_iter().enumerate().fold(
        column![].width(Length::Fill),
        |listed, (stripe, (label, value))| {
            let cell = text(value)
                .size(LABEL_SIZE)
                .width(Length::Fill)
                .wrapping(text::Wrapping::WordOrGlyph)
                .into();

            listed.push(fact(label, cell, stripe))
        },
    );

    container(container(listed).padding(theme::CONSOLE_BORDER_WIDTH))
        .width(Length::Fill)
        .style(theme::mock_console_container)
        .into()
}

fn footer_button<'a>(label: &'a str, message: Option<Message>) -> Element<'a, Message> {
    button(theme::button_label(label).size(LABEL_SIZE))
        .width(Length::Fill)
        .padding([3, 6])
        .on_press_maybe(message)
        .style(theme::primary_button)
        .into()
}

fn actions<'a>(ready: bool) -> Element<'a, Message> {
    let locate = button(theme::centered_text("Locate").size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .padding([1, 4])
        .on_press_maybe(ready.then_some(Message::Locate))
        .style(move |theme: &Theme, status| theme::toggle_button(theme, status, ready));

    column![
        panel_head("Action"),
        container(locate)
            .height(Length::Fixed(FACT_ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding([0, 3])
            .style(|theme: &Theme| theme::zebra_table_row(theme, 0)),
    ]
    .width(Length::Fixed(DEBUG_WIDTH))
    .into()
}

fn strip<'a>(
    focus: Focus,
    index: Option<i32>,
    part: Option<&ModelPart>,
    rig: Option<&Rig>,
    ready: bool,
    anim: &'a AnimSettings,
) -> Element<'a, Message> {
    let table = match focus {
        Focus::Curve => model_rows(index, part),
        Focus::Part => atlas_rows(index, part, rig),
    };

    let facts: Element<'_, Message> = column![fact_header(focus), table].width(Length::Fill).into();
    let body = row![facts, actions(ready), options(anim)].spacing(GAP);

    container(body).width(Length::Fill).into()
}

fn head_button<'a>(label: &'a str, message: Option<Message>) -> Element<'a, Message> {
    button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(HEAD_HEIGHT))
        .padding([1, 4])
        .on_press_maybe(message)
        .style(theme::primary_button)
        .into()
}

fn panel_head_row<'a>(mode: Mode, live: bool) -> Element<'a, Message> {
    let other = mode.other();

    let row = row![
        head_button(other.label(), Some(Message::Switch(other))),
        head_button("Manage", Some(Message::OpenManage)),
        head_button("Export", live.then_some(Message::Export)),
    ]
    .spacing(ROW_GAP)
    .align_y(Vertical::Center);

    container(row).width(Length::Fill).height(Length::Fixed(HEAD_HEIGHT)).into()
}

fn panel_frame<'a>(mode: Mode, live: bool, body: Element<'a, Message>) -> Element<'a, Message> {
    let framed = column![panel_head_row(mode, live), rule::horizontal(1), body]
        .spacing(ROW_GAP)
        .height(Length::Fill);

    container(framed)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Length::Fill)
        .padding(PANEL_PADDING)
        .into()
}

fn vacant_cuts<'a>() -> Element<'a, Message> {
    facts_card([
        ("Sheet", String::new()),
        ("Cuts", String::new()),
        ("Size", String::new()),
        ("Regions", String::new()),
    ])
}

fn vacant_slices<'a>() -> Element<'a, Message> {
    let table = console_card(column![cuts_header(0.0), centred(NO_SET_NOTICE)].width(Length::Fill));

    column![table, footer_button("Add Cut", None)].spacing(ROW_GAP).height(Length::Fill).into()
}

fn vacant_keys<'a>() -> Element<'a, Message> {
    let table = console_card(column![keys_header(0.0), centred(NO_SET_HINT)].width(Length::Fill));

    column![table, footer_button("Add Keyframe", None)].spacing(ROW_GAP).height(Length::Fill).into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Settled {
    Saved,
    Failed,
}

struct Backing {
    read_from: PathBuf,
    stamp: Stamp,
    dirty: bool,
    writing: bool,
    failed: bool,
    token: u64,
}

impl Backing {
    fn open(source: &Path) -> Option<(Backing, Vec<u8>)> {
        let read_from = source.to_path_buf();

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Studio could not read the file: {}", err))
            .ok()?;

        let Some(stamp) = preview::stamp(&read_from) else {
            warn!(path = %read_from.display(), "Studio could not stamp the file");

            return None;
        };

        let backing = Backing {
            read_from,
            stamp,
            dirty: false,
            writing: false,
            failed: false,
            token: next_token(),
        };

        Some((backing, bytes))
    }

    fn busy(&self) -> bool {
        self.dirty || self.writing
    }

    fn drifted(&self) -> bool {
        !self.dirty && !self.writing && preview::stamp(&self.read_from) != Some(self.stamp)
    }

    fn prepare(&mut self) -> (PathBuf, Stamp, u64) {
        self.dirty = false;
        self.writing = true;

        (self.read_from.clone(), self.stamp, self.token)
    }

    fn settle(&mut self, path: PathBuf, stamp: Option<Stamp>) -> Settled {
        self.writing = false;

        let Some(stamp) = stamp else {
            self.failed = true;

            return Settled::Failed;
        };

        self.read_from = path;
        self.stamp = stamp;
        self.failed = false;

        Settled::Saved
    }
}

struct Draft {
    backing: Backing,
    doc: Maanim,
    track: Option<usize>,
    inputs: Vec<[String; 4]>,
    cursors: Vec<[widget::Id; 3]>,
    neutral: i32,
    hint: String,
    looping: String,
    buffer: Option<(usize, Field)>,
    looped: bool,
}

impl Draft {
    fn load(anim: &Path) -> Option<Draft> {
        let (backing, bytes) = Backing::open(anim)?;

        let doc = Maanim::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the file: {}", err))
            .ok()?;

        let mut draft = Draft {
            backing,
            doc,
            track: None,
            inputs: Vec::new(),
            cursors: Vec::new(),
            neutral: 0,
            hint: CELL_HINT.to_string(),
            looping: String::new(),
            buffer: None,
            looped: false,
        };

        draft.restate();

        Some(draft)
    }

    fn retrack(&mut self, at: usize) {
        if at >= self.doc.tracks().len() {
            return;
        }

        self.track = Some(at);
        self.restate();
    }

    fn retrack_clamped(&mut self) {
        self.track = self.track.filter(|at| *at < self.doc.tracks().len());
        self.restate();
    }

    fn curve(&self) -> Option<&nyanko::graphics::rig::AnimModification> {
        self.doc.track(self.track?)
    }

    fn restate(&mut self) {
        let neutral = self.neutral;
        let looping = self.curve().map_or_else(String::new, |track| shown(track.loop_count, LOOP_DEFAULT));
        let track = self.curve();

        self.inputs = track
            .map(|track| {
                track
                    .keyframes
                    .iter()
                    .map(|key| {
                        let cells = [key.frame, key.value, key.ease, key.ease_power];

                        std::array::from_fn(|at| shown(cells[at], Field::ALL[at].default(neutral)))
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.cursors = self
            .inputs
            .iter()
            .map(|_| std::array::from_fn(|_| widget::Id::unique()))
            .collect();
        self.looping = looping;
    }

    fn set_looping(&mut self, value: &str) {
        if !typable(value) || self.track.is_none() {
            return;
        }

        self.looping = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.looped = true;

            return;
        }

        self.looped = false;

        let Some(parsed) = settled(value, LOOP_DEFAULT) else {
            return;
        };

        let Some(track) = self.track.and_then(|at| self.doc.edit(at)) else {
            return;
        };

        if track.loop_count == parsed {
            return;
        }

        track.loop_count = parsed;
        self.backing.dirty = true;
    }

    fn set_ease(&mut self, at: usize, ease: i32) {
        let Some(track) = self.track else {
            return;
        };

        let Some(key) = self.doc.edit(track).and_then(|track| track.keyframes.get_mut(at)) else {
            return;
        };

        if key.ease == ease {
            return;
        }

        key.ease = ease;
        self.restate();
        self.backing.dirty = true;
    }

    fn add_key(&mut self) {
        let Some(track) = self.track else {
            return;
        };

        if self.doc.add_key(track).is_none() {
            return;
        }

        self.restate();
        self.backing.dirty = true;
    }

    fn drop_key(&mut self, at: usize) {
        let Some(track) = self.track else {
            return;
        };

        if !self.doc.remove_key(track, at) {
            return;
        }

        self.restate();
        self.backing.dirty = true;
    }

    fn cursor(&self, at: usize, column: usize) -> widget::Id {
        self.cursors
            .get(at)
            .and_then(|row| row.get(column))
            .cloned()
            .unwrap_or_else(widget::Id::unique)
    }

    fn key_at(&self, at: usize) -> Option<i32> {
        Some(self.curve()?.keyframes.get(at)?.frame)
    }

    fn key_span(&self, at: usize) -> Option<(i32, i32)> {
        span_of(&self.curve()?.keyframes, at)
    }

    fn part(&self) -> Option<i32> {
        self.curve().map(|track| track.part)
    }

    fn buffering(&self, at: usize, field: Field) -> bool {
        self.buffer == Some((at, field))
    }

    fn resolve_looping(&mut self) {
        if !std::mem::take(&mut self.looped) {
            return;
        }

        let Some(typed) = self.looping.strip_prefix(BUFFER_MARK).map(str::to_owned) else {
            return;
        };

        self.set_looping(&typed);
    }

    fn resolve_buffer(&mut self) {
        let Some((at, field)) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self
            .inputs
            .get(at)
            .and_then(|row| row.get(field.slot()))
            .and_then(|slot| slot.strip_prefix(BUFFER_MARK))
        else {
            return;
        };

        let typed = typed.to_owned();

        self.edit(at, field, &typed);
    }

    fn edit(&mut self, at: usize, field: Field, value: &str) -> Option<usize> {
        if !typable(value) {
            return None;
        }

        let held = value.starts_with(BUFFER_MARK);

        let slot = self.inputs.get_mut(at).and_then(|row| row.get_mut(field.slot()))?;

        *slot = value.to_owned();

        if held {
            self.buffer = Some((at, field));

            return None;
        }

        if self.buffering(at, field) {
            self.buffer = None;
        }

        let parsed = settled(value, field.default(self.neutral))?;

        let track = self.track?;

        let key = self.doc.edit(track).and_then(|track| track.keyframes.get_mut(at))?;

        let cell = match field {
            Field::Frame => &mut key.frame,
            Field::Value => &mut key.value,
            Field::Ease => &mut key.ease,
            Field::Power => &mut key.ease_power,
        };

        if *cell == parsed {
            return None;
        }

        *cell = parsed;
        self.backing.dirty = true;

        if field != Field::Frame {
            return None;
        }

        self.reorder(track, at)
    }

    fn reorder(&mut self, track: usize, moved: usize) -> Option<usize> {
        let keys = &self.doc.track(track)?.keyframes;

        let mut order: Vec<usize> = (0..keys.len()).collect();
        order.sort_by_key(|at| keys[*at].frame);

        if order.iter().enumerate().all(|(to, from)| to == *from) {
            return None;
        }

        self.doc.sort_keys(track);
        self.inputs = order.iter().filter_map(|at| self.inputs.get(*at).cloned()).collect();
        self.cursors = order.iter().filter_map(|at| self.cursors.get(*at).cloned()).collect();

        order.iter().position(|from| *from == moved)
    }

    fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    fn persist_now(&mut self) {
        if !self.backing.busy() {
            return;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written);
    }

    fn looping(&self) -> Element<'_, Message> {
        let count = self.curve().map_or(0, |track| track.loop_count);

        let body = row![
            text("Loop").size(LABEL_SIZE),
            text_input(LOOP_HINT, &self.looping)
                .on_input(Message::LoopChanged)
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .width(Length::Fixed(LOOP_WIDTH))
                .style(theme::rounded_input),
            text(loop_label(count)).size(LABEL_SIZE),
        ]
        .spacing(ROW_GAP)
        .align_y(Vertical::Center);

        container(body).padding([LOOP_CARD_PAD, LOOP_CARD_PAD * 2.0]).style(theme::card_container_primary).into()
    }

    fn key_row(&self, at: usize, active: bool, stripe: usize, armed: bool, width: f32) -> Element<'_, Message> {
        let cells = self.inputs.get(at).map_or(&[][..], |row| row.as_slice());
        let ease = self.curve().and_then(|track| track.keyframes.get(at)).map_or(0, |key| key.ease);

        let mut line = row![theme::centered_text(at.to_string())
            .size(LABEL_SIZE)
            .width(Length::Fixed(INDEX_WIDTH))]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        for (column, field) in Field::TYPED.into_iter().enumerate() {
            let value = cells.get(field.slot()).map_or("", String::as_str);
            let live = field != Field::Power || ease_takes_power(ease);

            let hint = if field == Field::Value { self.hint.as_str() } else { CELL_HINT };
            let mut cell = text_input(hint, value)
                .id(self.cursor(at, column))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .align_x(Horizontal::Center)
                .width(Length::Fill)
                .style(theme::rounded_input);

            if live {
                cell = cell.on_input(move |typed| Message::Changed(at, field, typed));
            }

            line = line.push(cell);
        }

        let curve = pick_list(EASES, Some(ease_label(ease)), move |label| {
            Message::EaseChanged(at, ease_value(label).unwrap_or(0))
        })
        .width(Length::Fixed(EASE_WIDTH))
        .padding([1, 4])
        .text_size(LABEL_SIZE)
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu);

        let act = |label, message| {
            button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
                .width(Length::Fixed(STEP_WIDTH))
                .padding([0, 2])
                .on_press(message)
                .style(theme::neutral_button)
        };

        let actions =
            column![act("View", Message::Seek(at)), act("Bound", Message::Bound(at))].spacing(2.0);

        let mark = if armed { CONFIRM_MARK } else { CLOSE_LABEL };
        let drop = button(theme::centered_text(mark).size(CELL_SIZE))
            .width(Length::Fixed(DROP_WIDTH))
            .padding(0)
            .on_press(Message::DropKey(at))
            .style(theme::danger_button);

        let body = line.push(curve).push(actions).push(drop);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        container(body)
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| match active {
                true => container::Style {
                    background: Some(Color { a: ACTIVE_TINT, ..theme.palette().primary }.into()),
                    ..container::Style::default()
                },
                false => theme::zebra_table_row(theme, stripe),
            })
            .into()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Offset {
    row: usize,
    label: String,
}

impl std::fmt::Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub struct Sheet {
    backing: Backing,
    doc: Imgcut,
    art: RgbaImage,
    source: picture::Source,
    outlines: Vec<picture::Outline>,
    inputs: Vec<[String; 5]>,
    picked: Option<usize>,
    hidden: Option<usize>,
    buffer: Option<(usize, usize)>,
}

impl std::fmt::Debug for Sheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sheet").field("cuts", &self.doc.count()).finish()
    }
}

impl Sheet {
    fn assemble(backing: Backing, bytes: Vec<u8>, art: Vec<u8>) -> Option<Sheet> {
        let doc = Imgcut::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the cut list: {}", err))
            .ok()?;

        let decoded = image::load_from_memory(&art)
            .inspect_err(|err| warn!("Animation editor could not decode the atlas: {}", err))
            .ok()?
            .to_rgba8();

        let (width, height) = (decoded.width(), decoded.height());

        let mut sheet = Sheet {
            backing,
            doc,
            art: decoded,
            source: picture::Source::new(art, width, height),
            outlines: Vec::new(),
            inputs: Vec::new(),
            picked: None,
            hidden: None,
            buffer: None,
        };

        sheet.restate();

        Some(sheet)
    }

    fn span(&self) -> (i32, i32) {
        (self.art.width() as i32, self.art.height() as i32)
    }

    fn outside(&self, at: usize, cell: usize) -> bool {
        let (width, height) = self.span();
        let Some(cut) = self.doc.cut(at) else {
            return false;
        };

        match cell {
            0 => cut.x < 0 || cut.x > width,
            1 => cut.y < 0 || cut.y > height,
            2 => cut.width < 0 || cut.x.saturating_add(cut.width) > width,
            3 => cut.height < 0 || cut.y.saturating_add(cut.height) > height,
            _ => false,
        }
    }

    fn opaque(&self, at: usize) -> Option<[i32; 4]> {
        let cut = self.doc.cut(at)?;
        let (span_x, span_y) = self.span();

        let left = cut.x.clamp(0, span_x);
        let top = cut.y.clamp(0, span_y);
        let right = cut.x.saturating_add(cut.width).clamp(0, span_x);
        let bottom = cut.y.saturating_add(cut.height).clamp(0, span_y);

        let raw = self.art.as_raw();
        let stride = span_x as usize * 4;
        let mut seen: Option<[i32; 4]> = None;

        for y in top..bottom {
            let row = y as usize * stride;

            for x in left..right {
                if raw.get(row + x as usize * 4 + 3).copied().unwrap_or(0) < ALPHA_FLOOR {
                    continue;
                }

                seen = Some(match seen {
                    None => [x, y, x, y],
                    Some([near_x, near_y, far_x, far_y]) => {
                        [near_x.min(x), near_y.min(y), far_x.max(x), far_y.max(y)]
                    }
                });
            }
        }

        let [near_x, near_y, far_x, far_y] = seen?;
        let region = [near_x, near_y, far_x - near_x + 1, far_y - near_y + 1];

        (region != [cut.x, cut.y, cut.width, cut.height]).then_some(region)
    }

    fn find(&self, at: usize) -> Option<Point> {
        let cut = self.doc.cut(at)?;

        Some(Point::new(
            cut.x as f32 + cut.width as f32 / 2.0,
            cut.y as f32 + cut.height as f32 / 2.0,
        ))
    }

    fn restate(&mut self) {
        let (picked, hidden) = (self.picked, self.hidden);

        self.outlines = (0..self.doc.count())
            .filter(|at| hidden != Some(*at))
            .filter_map(|at| Some((at, self.doc.cut(at)?)))
            .map(|(at, cut)| picture::Outline {
                x: cut.x as f32,
                y: cut.y as f32,
                width: cut.width as f32,
                height: cut.height as f32,
                bold: picked == Some(at),
            })
            .collect();

        self.inputs = (0..self.doc.count())
            .map(|at| {
                std::array::from_fn(|cell| match cell {
                    CUT_NAME_FIELD => self.doc.name(at).unwrap_or_default().to_owned(),
                    cell => self.doc.field(at, cell).unwrap_or(0).to_string(),
                })
            })
            .collect();
    }

    fn buffering(&self, at: usize, cell: usize) -> bool {
        self.buffer == Some((at, cell))
    }

    fn resolve_buffer(&mut self) {
        let Some((at, cell)) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self
            .inputs
            .get(at)
            .and_then(|row| row.get(cell))
            .and_then(|slot| slot.strip_prefix(BUFFER_MARK))
            .map(str::to_owned)
        else {
            return;
        };

        self.edit(at, cell, &typed);
    }

    fn edit(&mut self, at: usize, cell: usize, value: &str) {
        let named = cell == CUT_NAME_FIELD;

        if named && !authoring::nameable(value) {
            return;
        }

        if !named && !typable(value) {
            return;
        }

        let Some(slot) = self.inputs.get_mut(at).and_then(|row| row.get_mut(cell)) else {
            return;
        };

        *slot = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.buffer = Some((at, cell));

            return;
        }

        if self.buffering(at, cell) {
            self.buffer = None;
        }

        let changed = match named {
            true => self.doc.set_name(at, value),
            false => settled(value, 0).is_some_and(|parsed| self.doc.set_field(at, cell, parsed.max(0))),
        };

        if changed {
            self.backing.dirty = true;
            self.restate();
        }
    }

    fn place(&mut self, at: usize, region: [i32; 4]) {
        if self.doc.place(at, region) {
            self.backing.dirty = true;
            self.restate();
        }
    }

    fn over(&self, at: Point) -> Option<usize> {
        (0..self.doc.count()).rev().find(|held| {
            self.doc.cut(*held).is_some_and(|cut| {
                let (x, y) = (at.x.floor() as i32, at.y.floor() as i32);

                x >= cut.x && y >= cut.y && x < cut.x + cut.width && y < cut.y + cut.height
            })
        })
    }

    fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    fn persist_now(&mut self) -> Settled {
        if !self.backing.busy() {
            return Settled::Saved;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written)
    }

    fn cut_row(&self, at: usize, framing: bool, armed: bool, picked: bool, width: f32) -> Element<'_, Message> {
        let cells = self.inputs.get(at).map_or(&[][..], |row| row.as_slice());

        let mut line = row![theme::centered_text(at.to_string())
            .size(LABEL_SIZE)
            .width(Length::Fixed(INDEX_WIDTH))]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        for cell in 0..CUT_FIELDS.len() {
            let named = cell == CUT_NAME_FIELD;
            let adrift = !named && self.outside(at, cell);
            let value = match framing && !named {
                true => "",
                false => cells.get(cell).map_or("", String::as_str),
            };

            let entry = text_input(if named { "" } else { CELL_HINT }, value)
                .on_input(move |typed| Message::Cut(at, cell, typed))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .width(match named {
                    true => Length::Fill,
                    false => Length::Fixed(CUT_CELL_WIDTH),
                })
                .style(move |theme: &Theme, status| adrift_input(theme, status, adrift));

            let entry = match named {
                true => entry,
                false => entry.align_x(Horizontal::Center),
            };

            let cell: Element<'_, Message> = match adrift {
                true => tip(entry, OUT_OF_BOUNDS),
                false => entry.into(),
            };

            line = line.push(cell);
        }

        let act = |label: &'static str, message, lit: bool| {
            button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
                .width(Length::Fixed(CUT_STEP_WIDTH))
                .padding([0, 2])
                .on_press(message)
                .style(move |theme: &Theme, status| match lit {
                    true => theme::toggle_button(theme, status, true),
                    false => theme::neutral_button(theme, status),
                })
        };

        let actions = column![
            row![act("Set", Message::Frame(at), framing), act("Trim", Message::Trim(at), false)]
                .spacing(2.0),
            row![act("Find", Message::Find(at), false), act("Select", Message::Slice(at), false)]
                .spacing(2.0),
        ]
        .spacing(2.0)
        .width(Length::Fixed(CUT_STEP_WIDTH * 2.0 + 2.0));

        let mark = if armed { CONFIRM_MARK } else { CLOSE_LABEL };
        let drop = button(theme::centered_text(mark).size(CELL_SIZE))
            .width(Length::Fixed(DROP_WIDTH))
            .padding(0)
            .on_press(Message::DropCut(at))
            .style(theme::danger_button);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        container(line.push(actions).push(drop))
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| match picked {
                true => container::Style {
                    background: Some(Color { a: ACTIVE_TINT, ..theme.palette().primary }.into()),
                    ..container::Style::default()
                },
                false => theme::zebra_table_row(theme, at),
            })
            .into()
    }
}

struct Pose {
    backing: Backing,
    doc: Mamodel,
    part: Option<usize>,
    row: usize,
    inputs: Vec<String>,
    cursors: Vec<widget::Id>,
    hints: Vec<String>,
    cuts: usize,
    align: [String; 2],
    buffer: Option<Slotted>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slotted {
    Cell(usize),
    Axis(usize),
}

impl Pose {
    fn load(model: &Path) -> Option<Pose> {
        let (backing, bytes) = Backing::open(model)?;

        let doc = Mamodel::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the model: {}", err))
            .ok()?;

        let mut pose = Pose {
            backing,
            doc,
            part: None,
            row: 0,
            inputs: Vec::new(),
            cursors: Vec::new(),
            hints: Vec::new(),
            cuts: 0,
            align: [String::new(), String::new()],
            buffer: None,
        };

        pose.reseat();
        pose.restate();

        Some(pose)
    }

    fn reseat(&mut self) {
        let cells = authoring::defaults(self.doc.model());

        self.hints = FIELDS
            .iter()
            .enumerate()
            .map(|(at, _)| cells.get(at).map_or_else(String::new, i32::to_string))
            .collect();
    }

    fn aim(&mut self, row: usize) {
        if row >= self.doc.offsets() {
            return;
        }

        self.row = row;
        self.restate();
    }

    fn offset(&self, row: usize) -> Offset {
        let label = match self.doc.offset_name(row).unwrap_or_default().trim() {
            "" => format!("Root {}", row),
            named => named.to_owned(),
        };

        Offset { row, label }
    }

    fn hint(&self, at: usize) -> &str {
        self.hints.get(at).map_or("", String::as_str)
    }

    fn fallback(&self, at: usize) -> i32 {
        authoring::defaults(self.doc.model()).get(at).copied().unwrap_or(0)
    }

    fn pick(&mut self, at: usize) {
        if at >= self.doc.count() {
            return;
        }

        self.part = Some(at);
        self.restate();
    }

    fn restate(&mut self) {
        let (x, y) = self.doc.offset(self.row).unwrap_or_default();
        self.align = [x.to_string(), y.to_string()];

        let Some(part) = self.part else {
            self.inputs.clear();
            self.cursors.clear();

            return;
        };

        self.inputs = (0..FIELDS.len())
            .map(|at| match at {
                NAME_FIELD => self.doc.name(part).unwrap_or_default().to_owned(),
                at => self.doc.field(part, at).unwrap_or(0).to_string(),
            })
            .collect();

        self.cursors = self.inputs.iter().map(|_| widget::Id::unique()).collect();
    }

    fn buffering(&self, held: Slotted) -> bool {
        self.buffer == Some(held)
    }

    fn resolve_buffer(&mut self) {
        let Some(held) = self.buffer.take() else {
            return;
        };

        let typed = match held {
            Slotted::Cell(at) => self.inputs.get(at),
            Slotted::Axis(axis) => self.align.get(axis),
        };

        let Some(typed) = typed.and_then(|slot| slot.strip_prefix(BUFFER_MARK)).map(str::to_owned) else {
            return;
        };

        match held {
            Slotted::Cell(at) => self.edit(at, &typed),
            Slotted::Axis(axis) => self.shift(axis, &typed),
        }
    }

    fn edit(&mut self, at: usize, value: &str) {
        let Some(part) = self.part else {
            return;
        };

        let named = at == NAME_FIELD;

        if named && !nameable(value) {
            return;
        }

        if !named && !typable(value) {
            return;
        }

        let Some(slot) = self.inputs.get_mut(at) else {
            return;
        };

        *slot = value.to_owned();

        if !named && value.starts_with(BUFFER_MARK) {
            self.buffer = Some(Slotted::Cell(at));

            return;
        }

        if self.buffering(Slotted::Cell(at)) {
            self.buffer = None;
        }

        let changed = match named {
            true => self.doc.set_name(part, value),
            false => settled(value, self.fallback(at))
                .map(|parsed| bound(self.doc.model(), self.cuts, at, parsed))
                .is_some_and(|parsed| self.doc.set_field(part, at, parsed)),
        };

        self.backing.dirty |= changed;
    }

    fn shift(&mut self, axis: usize, value: &str) {
        if !typable(value) {
            return;
        }

        let Some(slot) = self.align.get_mut(axis) else {
            return;
        };

        *slot = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.buffer = Some(Slotted::Axis(axis));

            return;
        }

        if self.buffering(Slotted::Axis(axis)) {
            self.buffer = None;
        }

        let Some(parsed) = settled(value, 0) else {
            return;
        };

        self.backing.dirty |= self.doc.set_offset(self.row, axis, parsed);
    }

    fn grow(&mut self, parent: Option<usize>) {
        let added = self.doc.add_part(parent);

        self.backing.dirty = true;
        self.pick(added);
    }

    fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    fn persist_now(&mut self) -> Settled {
        if !self.backing.busy() {
            return Settled::Saved;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written)
    }

    fn aligning(&self) -> Element<'_, Message> {
        let live = self.doc.offset(self.row).is_some();

        let axis = |at: usize| {
            let cell = text_input(CELL_HINT, self.align.get(at).map_or("", String::as_str))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .align_x(Horizontal::Center)
                .width(Length::Fixed(ALIGN_WIDTH))
                .style(theme::rounded_input);

            match live {
                true => cell.on_input(move |typed| Message::OffsetChanged(at, typed)),
                false => cell,
            }
        };

        let rows: Vec<Offset> = (0..self.doc.offsets()).map(|row| self.offset(row)).collect();
        let picker = pick_list(rows, Some(self.offset(self.row)), |offset| Message::Offset(offset.row))
            .width(Length::Fixed(OFFSET_WIDTH))
            .padding([1, 4])
            .text_size(LABEL_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        let body = row![picker, axis(0), axis(1)].spacing(ROW_GAP).align_y(Vertical::Center);

        container(body)
            .padding([LOOP_CARD_PAD, LOOP_CARD_PAD * 2.0])
            .style(theme::card_container_primary)
            .into()
    }

    fn field_row(&self, at: usize, width: f32) -> Element<'_, Message> {
        let named = at == NAME_FIELD;
        let index = if named { String::new() } else { at.to_string() };
        let hint = if named { "" } else { self.hint(at) };

        let cell = text_input(hint, self.inputs.get(at).map_or("", String::as_str))
            .id(self.cursors.get(at).cloned().unwrap_or_else(widget::Id::unique))
            .on_input(move |typed| Message::Field(at, typed))
            .size(CELL_SIZE)
            .padding(CELL_PADDING)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let body = row![
            theme::centered_text(index).size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH)),
            text(FIELDS.get(at).copied().unwrap_or_default()).size(LABEL_SIZE).width(Length::Fixed(FACT_LABEL)),
            cell,
        ]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(FIELD_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        container(body)
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| theme::zebra_table_row(theme, at))
            .into()
    }
}

fn shown(value: i32, default: i32) -> String {
    if value == default { String::new() } else { value.to_string() }
}

fn settled(value: &str, default: i32) -> Option<i32> {
    if value.is_empty() {
        return Some(default);
    }

    value.parse().ok()
}

fn typable(value: &str) -> bool {
    let mut chars = value.strip_prefix(BUFFER_MARK).unwrap_or(value).chars();

    match chars.next() {
        None => true,
        Some('-') => chars.all(|digit| digit.is_ascii_digit()),
        Some(first) if first.is_ascii_digit() => chars.all(|digit| digit.is_ascii_digit()),
        Some(_) => false,
    }
}

fn named(path: &Path) -> Option<String> {
    path.file_name().and_then(|name| name.to_str()).map(str::to_owned)
}

fn revalue_file(anim: &Path, moved: &[Option<usize>]) {
    let Some((mut backing, bytes)) = Backing::open(anim) else {
        return;
    };

    let Ok(mut doc) = Maanim::parse(&bytes) else {
        return;
    };

    if !doc.revalue(SPRITE_KIND, moved) {
        return;
    }

    backing.dirty = true;

    let (path, stamp, _) = backing.prepare();

    if write_now(&path, &doc.write(), stamp).is_none() {
        warn!(path = %path.display(), "Studio could not save a recut animation");
    }
}

fn retarget_file(anim: &Path, moved: &[Option<usize>]) {
    let Some((mut backing, bytes)) = Backing::open(anim) else {
        return;
    };

    let parsed = Maanim::parse(&bytes)
        .inspect_err(|err| warn!(path = %anim.display(), "Studio could not reindex the file: {}", err));

    let Ok(mut doc) = parsed else {
        return;
    };

    if !doc.retarget(moved) {
        return;
    }

    backing.dirty = true;

    let (path, stamp, _) = backing.prepare();

    if write_now(&path, &doc.write(), stamp).is_none() {
        warn!(path = %path.display(), "Studio could not save a reindexed animation");
    }
}

fn write_now(path: &Path, body: &[u8], stamp: Stamp) -> Option<Stamp> {
    preview::save(path, body, stamp)
        .inspect_err(|err| warn!(path = %path.display(), "Studio could not write the file: {}", err))
        .ok()
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    fn model(parents: &[i32]) -> Model {
        Model {
            parts: parents.iter().map(|parent| ModelPart { parent: *parent, ..ModelPart::default() }).collect(),
            ..Model::default()
        }
    }


    const SAMPLE: &str = "[modelanim:animation]\n1\n2\n0,11,-1,0,0,\n1\n0,0,0,0\n1,4,-1,0,0,\n1\n0,0,0,0\n";

    fn doc() -> Maanim {
        Maanim::parse(SAMPLE.as_bytes()).expect("the sample parses")
    }


    fn keys(frames: &[i32]) -> Vec<Keyframe> {
        frames.iter().map(|frame| Keyframe { frame: *frame, ..Keyframe::default() }).collect()
    }


    const DOUBLED: &str = "[modelanim:animation]\n1\n3\n5,11,-1,0,0,\n1\n0,0,0,0\n2,4,-1,0,0,\n1\n0,0,0,0\n5,11,-1,0,0,\n1\n0,0,0,0\n";

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
    fn a_bound_stops_one_frame_short_of_the_next_key() {
        // Ending on the next key would play the first frame of the following segment.
        let keys = keys(&[0, 10, 25]);

        assert_eq!(span_of(&keys, 0), Some((0, 9)));
        assert_eq!(span_of(&keys, 1), Some((10, 24)));
    }

    #[test]
    fn the_last_key_bounds_itself_rather_than_repeating_its_neighbour() {
        // It used to reach backwards and hand back the previous key's span verbatim.
        let keys = keys(&[0, 10, 25]);

        assert_eq!(span_of(&keys, 2), Some((25, 25)));
        assert_ne!(span_of(&keys, 2), span_of(&keys, 1));
    }

    #[test]
    fn keys_sharing_a_frame_do_not_invert_the_bound() {
        let keys = keys(&[4, 4]);

        assert_eq!(span_of(&keys, 0), Some((4, 4)));
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
