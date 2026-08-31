use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, image as iced_image, row, rule, scrollable, text, text_input, tooltip, Column, Row, Space, Text};
use iced::{Color, Element, Length, Size, Task, Theme};
use nyanko::cat::unit::{LevelCurve, TalentCost};
use nyanko::combat::Entity;

use kore::common::formats::SpriteSheet as CoreSpriteSheet;
use kore::common::region::Region;
use kore::domains::cat::files as cat_files;
use kore::domains::cat::game::stats as cat_stats;
use kore::domains::cat::game::talents as talent_logic;
use kore::domains::cat::scanner::{self as cat_scanner, CatEntry};
use kore::domains::mining::{self, changes, forms, talents, units, Build, Ore, Status};
use kore::domains::settings::Settings;
use kore::systems::combat::registry::{AbilityIcon, STAT_RARITY};
use kore::common::context::GlobalContext;
use kore::Vfs;

use crate::app::theme;
use crate::common::header_icon::{self, HeaderIcon};
use crate::common::{ability_icon, img015, skill_name, CustomAssets, SpriteSheet};
use crate::widget::{fallback_icon, list_row, section, smooth_scroll, tinted_superscript, uniform_grid};

const SIDEBAR_WIDTH: f32 = 110.0;
const SIDEBAR_PADDING: f32 = 8.0;
const SIDEBAR_SPACING: f32 = 4.0;
const TAB_TEXT_SIZE: f32 = 14.0;
const TAB_PADDING: [u16; 2] = [8, 12];

const PAGE_PADDING: f32 = 20.0;
const SCROLLBAR_GAP: f32 = 8.0;
const SCROLLBAR_RESERVE: f32 = 24.0;
const CARD_PADDING: f32 = 12.0;
const UNIT_MIN_WIDTH: f32 = 440.0;
const SECTION_SPACING: f32 = 18.0;
const CARD_SPACING: f32 = 8.0;
const ULTRA_LEVEL: i32 = 60;
const LEVEL_INPUT_WIDTH: f32 = 44.0;
const LEVEL_INPUT_PADDING: f32 = 2.0;
const IGNORED_LABELS: &[&str] = &["Width"];

const FIRST_FORM: usize = 0;

const PORTRAIT_SIZE: f32 = 56.0;
const TALENT_ICON_SIZE: f32 = 30.0;
const NAME_PLATE_HEIGHT: f32 = 26.0;
const UNIT_NAME_SIZE: f32 = 16.0;

const CHIP_RADIUS: f32 = 5.0;
const CHIP_PADDING: f32 = 6.0;
const CELL_SPACING: f32 = 8.0;
const HEADER_TEXT_SIZE: f32 = 13.0;
const LEDGER_TITLE_SIZE: f32 = 17.0;
const ABILITY_ICON_SIZE: f32 = 32.0;
const ABILITY_ICON_GAP: f32 = CELL_SPACING / 2.0;
const LEDGER_RULE_ALPHA: f32 = 0.35;

const VALUE_TEXT_SIZE: f32 = 13.0;
const BOX_PADDING: f32 = 4.0;
const BOX_RADIUS: f32 = 4.0;
const UNIT_BOX_HEIGHT: f32 = PORTRAIT_SIZE + BOX_PADDING * 2.0;
const HEADER_BOX_HEIGHT: f32 = TALENT_ICON_SIZE + BOX_PADDING * 2.0;
const VALUE_LABEL_GAP: f32 = 8.0;
const META_LABEL_WIDTH: f32 = 110.0;
const META_TEXT_SIZE: f32 = 14.0;
const NOTICE_TEXT_SIZE: f32 = 15.0;

const ULTRA_TINT: Color = Color { r: 0.47, g: 0.08, b: 0.08, a: 1.0 };
const NORMAL_TINT: Color = Color { r: 0.71, g: 0.55, b: 0.08, a: 1.0 };
const RETUNE_TINT: Color = Color { r: 0.16, g: 0.36, b: 0.55, a: 1.0 };
const ADDITION_TINT: Color = Color { r: 0.13, g: 0.42, b: 0.20, a: 1.0 };
const REMOVAL_TINT: Color = Color { r: 0.47, g: 0.13, b: 0.13, a: 1.0 };
const DARK_BOX_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 };
const CHIP_TEXT: Color = Color::WHITE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Meta,
    Cats,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Meta => "Meta",
            Self::Cats => "Cats",
        }
    }
}

const TABS: &[Tab] = &[Tab::Meta, Tab::Cats];

#[derive(Clone)]
pub enum Message {
    Select(Tab),
    LevelChanged(u32, String),
    OpenTalents(u32, usize),
    OpenForm(u32, usize),
    OpenUnit(u32),
    Img015Loaded(u64, usize, Option<CoreSpriteSheet>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Select(tab) => write!(f, "Select({:?})", tab),
            Self::LevelChanged(cat, value) => write!(f, "LevelChanged({}, {})", cat, value),
            Self::OpenTalents(cat, form) => write!(f, "OpenTalents({}, {})", cat, form),
            Self::OpenForm(cat, form) => write!(f, "OpenForm({}, {})", cat, form),
            Self::OpenUnit(cat) => write!(f, "OpenUnit({})", cat),
            Self::Img015Loaded(generation, index, _) => write!(f, "Img015Loaded({}, {})", generation, index),
        }
    }
}

pub struct State {
    tab: Tab,
    ore: Option<Ore>,
    report: Option<talents::Report>,
    units: Option<units::Report>,
    forms: Vec<forms::Unlocked>,
    changes: Vec<changes::Changed>,
    levels: HashMap<u32, String>,
    img015_sheets: Vec<SpriteSheet>,
    sheet_generation: u64,
    icons: ability_icon::Cache,
    plates: skill_name::Cache,
    portraits: header_icon::Cache,
    portrait_dummy: HeaderIcon,
    assets: CustomAssets,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tab: Tab::Meta,
            ore: None,
            report: None,
            units: None,
            forms: Vec::new(),
            changes: Vec::new(),
            levels: HashMap::new(),
            img015_sheets: Vec::new(),
            sheet_generation: 0,
            icons: ability_icon::Cache::default(),
            plates: skill_name::Cache::default(),
            portraits: header_icon::Cache::default(),
            portrait_dummy: HeaderIcon::dummy(),
            assets: CustomAssets::new(),
        }
    }
}

impl State {
    pub(crate) fn refresh(&mut self) {
        self.ore = mining::load();
        self.report = self
            .ore
            .as_ref()
            .and_then(|ore| ore.file(cat_files::SKILL_ACQUISITION))
            .map(talents::read);

        self.units = self
            .ore
            .as_ref()
            .and_then(|ore| ore.file(cat_files::UNIT_BUY))
            .map(units::read);

        self.forms = self
            .ore
            .as_ref()
            .and_then(|ore| ore.file(cat_files::UNIT_BUY))
            .map_or_else(Vec::new, forms::read);

        self.changes = self
            .ore
            .as_ref()
            .map_or_else(Vec::new, |ore| ore.files.iter().flat_map(changes::read).collect());

        if !self.enabled(self.tab) {
            self.tab = Tab::Meta;
        }
    }

    pub(crate) fn enter(&mut self, vfs: &Vfs) -> Task<Message> {
        self.refresh();

        self.check_sheets(vfs)
    }

    pub(crate) fn relocalize(&mut self, vfs: &Vfs) -> Task<Message> {
        self.clear_caches();

        self.check_sheets(vfs)
    }

    pub(crate) fn clear_caches(&mut self) {
        self.sheet_generation = self.sheet_generation.wrapping_add(1);
        self.icons.clear();
        self.plates.borrow_mut().clear();
        self.portraits.borrow_mut().clear();

        for sheet in &mut self.img015_sheets {
            sheet.mark_stale();
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(tab) => {
                if self.enabled(tab) {
                    self.tab = tab;
                }
            }
            Message::LevelChanged(cat_id, value) => {
                if value.chars().all(|glyph| glyph.is_ascii_digit()) {
                    self.levels.insert(cat_id, value);
                }
            }
            Message::OpenTalents(..) | Message::OpenForm(..) | Message::OpenUnit(..) => {}
            Message::Img015Loaded(generation, index, sheet) => {
                if generation == self.sheet_generation
                    && let Some(slot) = self.img015_sheets.get_mut(index)
                {
                    slot.apply(sheet);
                    self.icons.clear();
                }
            }
        }

        Task::none()
    }

    fn check_sheets(&mut self, vfs: &Vfs) -> Task<Message> {
        let generation = self.sheet_generation;

        img015::ensure_loaded(&mut self.img015_sheets, vfs)
            .map(move |(index, sheet)| Message::Img015Loaded(generation, index, sheet))
    }

    fn enabled(&self, tab: Tab) -> bool {
        match tab {
            Tab::Meta => true,
            Tab::Cats => self.has_finds(),
        }
    }

    pub(crate) fn enabled_levels(&self, cat_id: u32) -> Option<HashMap<u8, u8>> {
        self.report
            .as_ref()?
            .finds
            .iter()
            .find(|find| find.cat_id == cat_id)
            .map(talents::Find::enabled_levels)
    }

    fn unlocks(&self, cat_id: u32, form: usize) -> bool {
        self.forms.iter().any(|unlocked| unlocked.cat_id == cat_id && unlocked.form == form)
    }

    fn has_finds(&self) -> bool {
        self.has_talents() || self.has_fresh() || !self.forms.is_empty() || !self.changes.is_empty()
    }

    fn has_talents(&self) -> bool {
        self.report.as_ref().is_some_and(|report| report.status == Status::Changed && !report.finds.is_empty())
    }

    fn has_fresh(&self) -> bool {
        self.units.as_ref().is_some_and(|units| units.status == Status::Changed && !units.fresh.is_empty())
    }

    fn fresh_units<'a>(&'a self, cats: &'a [CatEntry], vfs: &Vfs, settings: &Settings) -> Vec<&'a CatEntry> {
        let Some(units) = &self.units else {
            return Vec::new();
        };

        let mut strict = settings.scanner_config(None);
        strict.show_invalid_cats = false;

        let spirits = conjured(cats);

        units
            .fresh
            .iter()
            .filter(|id| !spirits.contains(*id))
            .filter_map(|id| cats.iter().find(|entry| entry.id == *id))
            .filter(|entry| cat_scanner::listable(vfs, entry, &strict))
            .collect()
    }

    pub fn view<'a>(
        &'a self,
        cats: &'a [CatEntry],
        global: GlobalContext<'a>,
        settings: &'a Settings,
        window: Size,
    ) -> Element<'a, Message> {
        let body = match self.tab {
            Tab::Meta => self.view_meta(),
            Tab::Cats => self.view_cats(cats, global, settings, window.width - SIDEBAR_WIDTH),
        };

        let page = smooth_scroll(
            scrollable(container(body).padding(PAGE_PADDING).width(Length::Fill))
                .spacing(SCROLLBAR_GAP)
                .width(Length::Fill)
                .height(Length::Fill),
        );

        row![self.view_sidebar(), page].height(Length::Fill).into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut tabs = Column::new().spacing(SIDEBAR_SPACING);

        for tab in TABS {
            let cell = container(theme::button_label(tab.label()).size(TAB_TEXT_SIZE).wrapping(Wrapping::None))
                .padding(TAB_PADDING)
                .width(Length::Fill);

            tabs = tabs.push(if self.enabled(*tab) {
                list_row(cell, self.tab == *tab, true, Length::Fill, Message::Select(*tab))
            } else {
                cell.style(|theme: &Theme| container::Style {
                    text_color: Some(theme::weak_text_color(theme)),
                    ..theme::card_container_muted(theme)
                })
                .into()
            });
        }

        container(smooth_scroll(scrollable(tabs).width(Length::Fill).height(Length::Fill)))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(SIDEBAR_PADDING)
            .style(theme::list_panel_container)
            .into()
    }

    fn view_meta(&self) -> Element<'_, Message> {
        let Some(ore) = &self.ore else {
            return notice("No base available, import game data to create a base");
        };

        let baseline = self.report.as_ref().is_some_and(|report| report.status == Status::Baseline);

        if baseline || ore.files.is_empty() {
            return notice("No ore available, import a game data to see its diff");
        }

        let mut rows = vec![("Captured", ore.age().map_or_else(|| "just now".to_string(), ago))];

        rows.extend(version_rows(ore));

        if let Some(report) = &self.report {
            rows.push(("Source", source_line(&report.region, &ore.after)));

            if !report.dropped.is_empty() {
                rows.push(("Units dropped", report.dropped.len().to_string()));
            }

            if report.unreadable > 0 {
                rows.push(("Rows unread", report.unreadable.to_string()));
            }
        }

        let mut table = Column::new().spacing(6);

        for (label, value) in rows {
            table = table.push(
                row![
                    strong(label, META_TEXT_SIZE).width(Length::Fixed(META_LABEL_WIDTH)),
                    plain(value, META_TEXT_SIZE),
                ]
                .spacing(VALUE_LABEL_GAP)
                .align_y(Vertical::Center),
            );
        }

        table.width(Length::Shrink).into()
    }

    fn view_cats<'a>(
        &'a self,
        cats: &'a [CatEntry],
        global: GlobalContext<'a>,
        settings: &'a Settings,
        width: f32,
    ) -> Element<'a, Message> {
        let vfs = &global.vault.vfs;

        let mut body = Column::new().spacing(SECTION_SPACING).width(Length::Fill);
        let mut listed = false;

        let fresh = self.fresh_units(cats, vfs, settings);

        if !fresh.is_empty() {
            listed = true;
            let cards: Vec<Element<'a, Message>> = fresh.into_iter().map(|cat| self.view_fresh(cat)).collect();

            body = body.push(section("New", Length::Fill, uniform_grid(cards, CARD_SPACING)));
        }

        let altered: Vec<&changes::Changed> = self
            .changes
            .iter()
            .filter(|changed| cats.iter().any(|entry| entry.id == changed.cat_id))
            .filter(|changed| !self.unlocks(changed.cat_id, changed.form))
            .collect();

        let read: Vec<(&changes::Changed, forms::Diff)> = altered
            .into_iter()
            .map(|changed| (changed, changed_diff(changed, cats, global, settings)))
            .filter(|(_, diff)| !diff.is_empty())
            .collect();

        if !read.is_empty() {
            listed = true;

            let cards: Vec<Element<'a, Message>> = read
                .into_iter()
                .map(|(changed, diff)| self.view_changed(changed, cats, diff))
                .collect();

            body = body.push(section("Changes", Length::Fill, wrap(cards, cards_per_row(width, UNIT_MIN_WIDTH))));
        }

        let unlocked: Vec<&forms::Unlocked> = self
            .forms
            .iter()
            .filter(|unlocked| cats.iter().any(|entry| entry.id == unlocked.cat_id))
            .collect();

        if !unlocked.is_empty() {
            listed = true;

            let cards: Vec<Element<'a, Message>> = unlocked
                .into_iter()
                .map(|entry| self.view_unlocked(entry, cats, global, settings))
                .collect();

            body = body.push(section("Forms", Length::Fill, wrap(cards, cards_per_row(width, UNIT_MIN_WIDTH))));
        }

        if let Some(report) = &self.report
            && self.has_talents()
        {
            listed = true;

            let mut ranked: Vec<&talents::Find> = report.finds.iter().collect();
            ranked.sort_by_key(|find| talent_rank(find));

            let cards: Vec<Element<'a, Message>> =
                ranked.into_iter().map(|find| self.view_find(find, cats, vfs, settings)).collect();

            body = body.push(section("Talents", Length::Fill, wrap(cards, cards_per_row(width, UNIT_MIN_WIDTH))));
        }

        if !listed {
            return notice("No ore available, import a game data to see its diff");
        }

        body.into()
    }

    fn view_fresh<'a>(&'a self, cat: &'a CatEntry) -> Element<'a, Message> {
        let identity = button(self.view_identity(Some(cat), FIRST_FORM, cat.id))
            .padding(0)
            .style(button::text)
            .on_press(Message::OpenUnit(cat.id));

        let card = row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(format!("ID: {}", cat.id), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong((STAT_RARITY.formatter)(cat.unitbuy.rarity), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center);

        container(card).padding(CARD_PADDING).width(Length::Shrink).style(theme::card_container).into()
    }

    fn view_identity<'a>(&'a self, cat: Option<&'a CatEntry>, form: usize, id: u32) -> Element<'a, Message> {
        let icon = cat
            .and_then(|entry| entry.deploy_icon_paths.get(form).and_then(Option::as_ref))
            .and_then(|path| header_icon::load(&self.portraits, path))
            .unwrap_or_else(|| self.portrait_dummy.clone());

        let name = cat.map_or_else(|| format!("Unknown unit {:03}", id), |entry| entry.display_name(form));

        let frame = container(iced_image(icon.handle).height(Length::Fixed(PORTRAIT_SIZE)))
            .width(Length::Fixed(PORTRAIT_SIZE))
            .height(Length::Fixed(PORTRAIT_SIZE))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        row![frame, strong(name, UNIT_NAME_SIZE)]
            .spacing(CELL_SPACING)
            .align_y(Vertical::Center)
            .into()
    }

    fn view_changed<'a>(
        &'a self,
        changed: &'a changes::Changed,
        cats: &'a [CatEntry],
        diff: forms::Diff,
    ) -> Element<'a, Message> {
        let cat = cats.iter().find(|entry| entry.id == changed.cat_id);
        let badge = format!("{} FORM", cat_files::form_name(changed.form).to_uppercase());

        self.view_diff_card(cat, changed.cat_id, changed.form, badge, diff)
    }

    fn view_unlocked<'a>(
        &'a self,
        unlocked: &'a forms::Unlocked,
        cats: &'a [CatEntry],
        global: GlobalContext<'a>,
        settings: &Settings,
    ) -> Element<'a, Message> {
        let cat = cats.iter().find(|entry| entry.id == unlocked.cat_id);
        let form = unlocked.form;
        let badge = if form == forms::ULTRA_FORM { "ULTRA FORM" } else { "TRUE FORM" };

        let diff = cat.map_or_else(forms::Diff::default, |entry| form_diff(entry, form, global, settings));

        self.view_diff_card(cat, unlocked.cat_id, form, badge.to_string(), diff)
    }

    fn view_diff_card<'a>(
        &'a self,
        cat: Option<&'a CatEntry>,
        cat_id: u32,
        form: usize,
        badge: String,
        diff: forms::Diff,
    ) -> Element<'a, Message> {
        let identity = button(self.view_identity(cat, form, cat_id))
            .padding(0)
            .style(button::text)
            .on_press_maybe(cat.map(|_| Message::OpenForm(cat_id, form)));

        let header = row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(badge, UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center);

        let icons = Icons { cache: &self.icons, sheets: &self.img015_sheets, assets: &self.assets };
        let mut body = Column::new().spacing(CARD_SPACING).width(Length::Fill);

        body = body.extend(panels(&diff, &icons));

        if let Some(spirit) = &diff.spirit
            && !spirit.is_empty()
        {
            body = body.push(strong("Conjure", LEDGER_TITLE_SIZE));
            body = body.extend(panels(spirit, &icons));
        }

        let card = column![header, rule::horizontal(1), body].spacing(10).width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }

    fn view_find<'a>(
        &'a self,
        find: &'a talents::Find,
        cats: &'a [CatEntry],
        vfs: &'a Vfs,
        settings: &'a Settings,
    ) -> Element<'a, Message> {
        let cat = cats.iter().find(|entry| entry.id == find.cat_id);
        let form = cat.map_or(0, |entry| portrait_form(entry, find.has_ultra()));
        let seeded = cat.map_or(1, |entry| cat_stats::seeded_level(entry, settings).0);

        let level = self.level_for(find, seeded);

        let unit = UnitContext {
            base: cat.and_then(|entry| entry.stats.get(form).and_then(Option::as_ref)),
            curve: cat.and_then(|entry| entry.curve.as_ref()),
            costs: cat.map(|entry| entry.talent_costs.as_ref()),
            level: level.value,
        };

        let mut ordered: Vec<(&talents::Gain, bool)> = find
            .gained
            .iter()
            .map(|gain| (gain, false))
            .chain(find.retuned.iter().map(|retune| (&retune.gain, true)))
            .collect();

        ordered.sort_by_key(|(gain, _)| gain.ultra);

        let mut talents = Column::new().spacing(CARD_SPACING).width(Length::Fill);

        for (gain, retuned) in ordered {
            talents = talents.push(self.view_gain(gain, retuned, &unit, vfs));
        }

        let card = column![self.view_unit_header(find, cat, form, level), rule::horizontal(1), talents]
            .spacing(10)
            .width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }

    fn level_for(&self, find: &talents::Find, seeded: i32) -> Level {
        let fallback = if find.has_ultra() { ULTRA_LEVEL } else { seeded };
        let typed = self.levels.get(&find.cat_id);

        let value = typed
            .and_then(|input| input.parse::<i32>().ok())
            .filter(|level| *level > 0)
            .unwrap_or(fallback);

        Level { value, fallback, typed: typed.cloned().unwrap_or_default() }
    }

    fn view_unit_header<'a>(
        &'a self,
        find: &'a talents::Find,
        cat: Option<&'a CatEntry>,
        form: usize,
        level: Level,
    ) -> Element<'a, Message> {
        let cat_id = find.cat_id;

        let identity = button(self.view_identity(cat, form, cat_id))
            .padding(0)
            .style(button::text)
            .on_press_maybe(cat.map(|_| Message::OpenTalents(cat_id, form)));

        let field = text_input(&level.fallback.to_string(), &level.typed)
            .on_input(move |value| Message::LevelChanged(cat_id, value))
            .width(Length::Fixed(LEVEL_INPUT_WIDTH))
            .size(HEADER_TEXT_SIZE)
            .padding(LEVEL_INPUT_PADDING)
            .style(theme::rounded_input);

        let level_cell = row![strong("LEVEL", UNIT_NAME_SIZE), field]
            .spacing(CELL_SPACING)
            .align_y(Vertical::Center);

        row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(kind_badge(find), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(level_cell, Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center)
        .into()
    }

    fn view_gain<'a>(
        &'a self,
        gain: &'a talents::Gain,
        retuned: bool,
        unit: &UnitContext<'_>,
        vfs: &'a Vfs,
    ) -> Element<'a, Message> {
        let tint = match (retuned, gain.ultra) {
            (true, _) => RETUNE_TINT,
            (false, true) => ULTRA_TINT,
            (false, false) => NORMAL_TINT,
        };

        let cap = gain.group.max_level.max(1);

        let reading = unit
            .base
            .and_then(|stats| talent_logic::calculate_talent_display(&gain.group, stats, cap, unit.curve, unit.level))
            .unwrap_or_default();

        let mut inner = Column::new()
            .spacing(4)
            .push(self.view_gain_header(gain, cap, unit, vfs))
            .width(Length::Fill);

        for line in reading.lines().filter(|line| !ignored(line)) {
            inner = inner.push(value_row(line));
        }

        container(inner)
            .padding(CHIP_PADDING)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(tint.into()),
                border: iced::border::rounded(CHIP_RADIUS),
                ..Default::default()
            })
            .into()
    }

    fn view_gain_header<'a>(
        &'a self,
        gain: &'a talents::Gain,
        cap: u8,
        unit: &UnitContext<'_>,
        vfs: &'a Vfs,
    ) -> Element<'a, Message> {
        let np = unit.costs.map_or(0, |costs| talent_logic::get_talent_np_cost(gain.group.cost_id, cap, costs));

        let name: Element<'_, Message> = match skill_name::load(&self.plates, &gain.group, vfs) {
            Some(handle) => iced_image(handle).height(Length::Fixed(NAME_PLATE_HEIGHT)).into(),
            None => header_text(gain.name).into(),
        };

        row![
            self.view_talent_icon(gain),
            name,
            Space::new().width(Length::Fill),
            dark_box(header_text(format!("MAX LV: {}", cap)), Length::Fixed(HEADER_BOX_HEIGHT)),
            dark_box(header_text(format!("NP COST: {}", np)), Length::Fixed(HEADER_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center)
        .width(Length::Fill)
        .into()
    }

    fn view_talent_icon<'a>(&'a self, gain: &'a talents::Gain) -> Element<'a, Message> {
        self.view_icon(gain.icon, gain.fallback, TALENT_ICON_SIZE)
    }

    fn view_icon<'a>(&'a self, icon: AbilityIcon, fallback: &'a str, size: f32) -> Element<'a, Message> {
        match icon {
            AbilityIcon::Custom(custom) => {
                if let Some(handle) = self.assets.get_icon_texture(custom) {
                    return iced_image(handle).width(Length::Fixed(size)).height(Length::Fixed(size)).into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.icons.handle(icon_id, &self.img015_sheets) {
                    return iced_image(handle).width(Length::Fixed(size)).height(Length::Fixed(size)).into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon(fallback)
    }
}

struct Icons<'a> {
    cache: &'a ability_icon::Cache,
    sheets: &'a [SpriteSheet],
    assets: &'a CustomAssets,
}

impl Icons<'_> {
    fn render<'b>(&self, icon: AbilityIcon, fallback: &'static str) -> Element<'b, Message> {
        match icon {
            AbilityIcon::Custom(custom) => {
                if let Some(handle) = self.assets.get_icon_texture(custom) {
                    return iced_image(handle)
                        .width(Length::Fixed(ABILITY_ICON_SIZE))
                        .height(Length::Fixed(ABILITY_ICON_SIZE))
                        .into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.cache.handle(icon_id, self.sheets) {
                    return iced_image(handle)
                        .width(Length::Fixed(ABILITY_ICON_SIZE))
                        .height(Length::Fixed(ABILITY_ICON_SIZE))
                        .into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon(fallback)
    }
}

struct UnitContext<'a> {
    base: Option<&'a Entity>,
    curve: Option<&'a LevelCurve>,
    costs: Option<&'a HashMap<u8, TalentCost>>,
    level: i32,
}

#[derive(Clone)]
struct Level {
    value: i32,
    fallback: i32,
    typed: String,
}

fn ignored(line: &str) -> bool {
    line.split_once(": ").is_some_and(|(label, _)| IGNORED_LABELS.contains(&label))
}

fn header_text<'a>(content: impl ToString) -> Text<'a> {
    strong(content, HEADER_TEXT_SIZE).color(CHIP_TEXT)
}

fn strong<'a>(content: impl ToString, size: f32) -> Text<'a> {
    theme::bold_text(content).size(size).wrapping(Wrapping::None)
}

fn plain<'a>(content: impl ToString, size: f32) -> Text<'a> {
    text(content.to_string()).size(size).wrapping(Wrapping::None)
}

fn value_row<'a>(line: &str) -> Element<'a, Message> {
    let content: Element<'a, Message> = match line.split_once(": ") {
        Some((label, reading)) => row![
            plain(label, VALUE_TEXT_SIZE).color(CHIP_TEXT),
            strong(reading, VALUE_TEXT_SIZE).color(CHIP_TEXT),
        ]
        .spacing(VALUE_LABEL_GAP)
        .align_y(Vertical::Center)
        .into(),
        None => strong(line, VALUE_TEXT_SIZE).color(CHIP_TEXT).into(),
    };

    dark_box(content, Length::Shrink)
}

fn dark_box<'a>(content: impl Into<Element<'a, Message>>, height: Length) -> Element<'a, Message> {
    container(content)
        .padding(BOX_PADDING)
        .height(height)
        .align_y(Vertical::Center)
        .style(|_theme: &Theme| container::Style {
            background: Some(DARK_BOX_BG.into()),
            border: iced::border::rounded(BOX_RADIUS),
            ..Default::default()
        })
        .into()
}

fn light_box<'a>(content: impl Into<Element<'a, Message>>, height: Length) -> Element<'a, Message> {
    container(content)
        .padding(BOX_PADDING)
        .height(height)
        .align_y(Vertical::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            border: iced::border::rounded(BOX_RADIUS),
            ..Default::default()
        })
        .into()
}

fn notice<'a>(message: &'a str) -> Element<'a, Message> {
    plain(message, NOTICE_TEXT_SIZE)
        .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) })
        .into()
}

fn wrap<'a>(cards: Vec<Element<'a, Message>>, per_row: usize) -> Element<'a, Message> {
    let mut grid = Column::new().spacing(CARD_SPACING).width(Length::Fill);
    let mut line = Row::new().spacing(CARD_SPACING).width(Length::Fill);
    let mut filled = 0;

    for card in cards {
        line = line.push(card);
        filled += 1;

        if filled == per_row {
            grid = grid.push(line);
            line = Row::new().spacing(CARD_SPACING).width(Length::Fill);
            filled = 0;
        }
    }

    if filled > 0 {
        for _ in filled..per_row {
            line = line.push(Space::new().width(Length::Fill));
        }

        grid = grid.push(line);
    }

    grid.into()
}

fn cards_per_row(available_width: f32, min_width: f32) -> usize {
    let usable = (available_width - PAGE_PADDING * 2.0 - SCROLLBAR_RESERVE).max(min_width);
    let slot = min_width + CARD_SPACING;

    (((usable + CARD_SPACING) / slot).floor() as usize).max(1)
}

fn changed_diff<'a>(
    changed: &'a changes::Changed,
    cats: &'a [CatEntry],
    global: GlobalContext<'a>,
    settings: &Settings,
) -> forms::Diff {
    let cat = cats.iter().find(|entry| entry.id == changed.cat_id);

    forms::compare(&forms::Subject {
        global,
        previous: &changed.previous,
        current: &changed.current,
        curve: cat.and_then(|entry| entry.curve.as_ref()),
        level: reading_level(cat, changed.form, settings),
        frames: frames_for(cat, changed.form, changed.form),
    })
}

fn reading_level(cat: Option<&CatEntry>, form: usize, settings: &Settings) -> i32 {
    if form == forms::ULTRA_FORM {
        return ULTRA_LEVEL;
    }

    cat.map_or(1, |entry| cat_stats::seeded_level(entry, settings).0)
}

fn frames_for(cat: Option<&CatEntry>, earlier: usize, form: usize) -> (i32, i32) {
    let frames = |slot: usize| {
        cat.and_then(|entry| entry.atk_anim_frames.get(slot).copied()).unwrap_or_default()
    };

    (frames(earlier), frames(form))
}

fn form_diff<'a>(cat: &'a CatEntry, form: usize, global: GlobalContext<'a>, settings: &Settings) -> forms::Diff {
    let Some(earlier) = (0..form).rev().find(|slot| cat.forms.get(*slot).copied().unwrap_or(false)) else {
        return forms::Diff::default();
    };

    let (Some(previous), Some(current)) = (
        cat.stats.get(earlier).and_then(Option::as_ref),
        cat.stats.get(form).and_then(Option::as_ref),
    ) else {
        return forms::Diff::default();
    };

    forms::compare(&forms::Subject {
        global,
        previous,
        current,
        curve: cat.curve.as_ref(),
        level: reading_level(Some(cat), form, settings),
        frames: frames_for(Some(cat), earlier, form),
    })
}

fn panels<'a>(diff: &forms::Diff, icons: &Icons<'_>) -> Vec<Element<'a, Message>> {
    let sides = [
        ("Additions", ADDITION_TINT, &diff.gains, &diff.learned),
        ("Removals", REMOVAL_TINT, &diff.losses, &diff.forgotten),
    ];

    sides
        .into_iter()
        .filter_map(|(title, tint, stats, abilities)| ledger(title, tint, stats, abilities, icons))
        .collect()
}

fn ledger<'a>(
    title: &'static str,
    tint: Color,
    stats: &[forms::Change],
    abilities: &[forms::Ability],
    icons: &Icons<'_>,
) -> Option<Element<'a, Message>> {
    if stats.is_empty() && abilities.is_empty() {
        return None;
    }

    let mut panel = Column::new().spacing(8).width(Length::Fill);

    panel = panel.push(strong(title, LEDGER_TITLE_SIZE).color(CHIP_TEXT));
    panel = panel.push(ledger_rule());

    if !stats.is_empty() {
        let mut lines = Column::new().spacing(4).width(Length::Fill);

        for change in stats {
            lines = lines.push(dark_box(change_row(change), Length::Shrink));
        }

        panel = panel.push(labelled("Statistics", lines));
    }

    if !abilities.is_empty() {
        panel = panel.push(labelled("Abilities", ability_groups(abilities, icons)));
    }

    Some(
        container(panel)
            .padding(CHIP_PADDING)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(tint.into()),
                border: iced::border::rounded(CHIP_RADIUS),
                ..Default::default()
            })
            .into(),
    )
}

fn ledger_rule<'a>() -> Element<'a, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color { a: LEDGER_RULE_ALPHA, ..CHIP_TEXT },
            ..rule::default(theme)
        })
        .into()
}

fn labelled<'a>(title: &'static str, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![strong(title, HEADER_TEXT_SIZE).color(CHIP_TEXT), body.into()].spacing(4).width(Length::Fill).into()
}

fn change_row<'a>(change: &forms::Change) -> Element<'a, Message> {
    let mut line = Row::new().spacing(VALUE_LABEL_GAP).align_y(Vertical::Center);

    line = line.push(plain(change.label, VALUE_TEXT_SIZE).color(CHIP_TEXT));
    line = line.push(strong(&change.before, VALUE_TEXT_SIZE).color(CHIP_TEXT));

    if let Some(shift) = &change.shift {
        line = line.push(plain(shift, VALUE_TEXT_SIZE).color(CHIP_TEXT));
    }

    line = line.push(strong("->", VALUE_TEXT_SIZE).color(CHIP_TEXT));
    line = line.push(strong(&change.after, VALUE_TEXT_SIZE).color(CHIP_TEXT));

    line.into()
}

fn ability_groups<'a>(abilities: &[forms::Ability], icons: &Icons<'_>) -> Element<'a, Message> {
    let mut body = Column::new().spacing(4).width(Length::Fill);
    let mut folded = Row::new().spacing(ABILITY_ICON_GAP).align_y(Vertical::Center);
    let mut plain_count = 0;

    for ability in abilities.iter().filter(|ability| !ability.explained()) {
        folded = folded.push(hinted_icon(ability, icons));
        plain_count += 1;
    }

    if plain_count > 0 {
        body = body.push(dark_box(folded, Length::Shrink));
    }

    for ability in abilities.iter().filter(|ability| ability.explained()) {
        body = body.push(dark_box(explained_ability(ability, icons), Length::Shrink));
    }

    body.into()
}

fn hinted_icon<'a>(ability: &forms::Ability, icons: &Icons<'_>) -> Element<'a, Message> {
    let icon = icons.render(ability.icon, ability.fallback);

    if ability.text.is_empty() {
        return icon;
    }

    tooltip(
        icon,
        container(tinted_superscript(&ability.text, VALUE_TEXT_SIZE, None))
            .padding(6)
            .style(container::bordered_box),
        tooltip::Position::Top,
    )
    .into()
}

fn explained_ability<'a>(ability: &forms::Ability, icons: &Icons<'_>) -> Element<'a, Message> {
    let mut block = Column::new().spacing(2).width(Length::Fill);

    block = block.push(
        row![
            icons.render(ability.icon, ability.fallback),
            strong(ability.name, VALUE_TEXT_SIZE).color(CHIP_TEXT),
        ]
        .spacing(4)
        .align_y(Vertical::Center),
    );

    for change in &ability.detail {
        block = block.push(change_row(change));
    }

    block.into()
}

fn conjured(cats: &[CatEntry]) -> HashSet<u32> {
    cats.iter()
        .flat_map(|entry| entry.stats.iter().flatten())
        .filter_map(|stats| u32::try_from(stats.conjure_unit_id).ok())
        .collect()
}

fn talent_kinds(find: &talents::Find) -> (bool, bool) {
    let mut normal = false;
    let mut ultra = false;

    for gain in find.gained.iter().chain(find.retuned.iter().map(|retune| &retune.gain)) {
        if gain.ultra {
            ultra = true;
        } else {
            normal = true;
        }
    }

    (normal, ultra)
}

fn kind_badge(find: &talents::Find) -> &'static str {
    match talent_kinds(find) {
        (true, true) => "TALENT+ULTRA",
        (false, true) => "ULTRA",
        _ => "TALENT",
    }
}

fn talent_rank(find: &talents::Find) -> u8 {
    match talent_kinds(find) {
        (true, true) => 2,
        (false, true) => 1,
        _ => 0,
    }
}

fn version_rows(ore: &Ore) -> Vec<(&'static str, String)> {
    if ore.after.is_empty() {
        return ore
            .before
            .iter()
            .map(|build| ("Base build", format!("{}  {} ({})", build.label, build.name, build.code)))
            .collect();
    }

    ore.after
        .iter()
        .map(|build| {
            let carried = ore.before.iter().find(|held| held.label == build.label);

            let reading = match carried {
                Some(carried) if carried.name != build.name => {
                    format!("{}  {} -> {} ({})", build.label, carried.name, build.name, build.code)
                }
                _ => format!("{}  {} ({})", build.label, build.name, build.code),
            };

            ("Version", reading)
        })
        .collect()
}

fn source_line(region: &str, builds: &[Build]) -> String {
    let name = region_name(region);

    build_for(region, builds).map_or_else(|| name.to_string(), |build| format!("{} v{}", name, build.name))
}

fn build_for<'a>(region: &str, builds: &'a [Build]) -> Option<&'a Build> {
    if builds.len() == 1 {
        return builds.first();
    }

    let suffix = region.parse::<Region>().ok()?.metadata().package_suffix;
    let needle = format!("battlecats{}", suffix);

    builds.iter().find(|build| build.label.ends_with(&needle))
}

fn portrait_form(cat: &CatEntry, ultra: bool) -> usize {
    let ceiling = if ultra { 3 } else { 2 };

    (0..=ceiling)
        .rev()
        .find(|&form| cat.forms.get(form).copied().unwrap_or(false))
        .or_else(|| (0..cat.forms.len()).rev().find(|&form| cat.forms[form]))
        .unwrap_or(0)
}

fn region_name(code: &str) -> &'static str {
    code.parse::<Region>().map_or("loose files", |region| region.metadata().display_name)
}

fn ago(age: Duration) -> String {
    let seconds = age.as_secs();

    if seconds < 60 {
        return "moments ago".to_string();
    }

    let (count, unit) = match seconds {
        60..3600 => (seconds / 60, "minute"),
        3600..86400 => (seconds / 3600, "hour"),
        _ => (seconds / 86400, "day"),
    };

    format!("{} {}{} ago", count, unit, if count == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(label: &str, name: &str, code: u32) -> Build {
        Build { code, name: name.to_string(), label: label.to_string() }
    }

    fn ore(before: Vec<Build>, after: Vec<Build>) -> Ore {
        Ore { schema: 2, stamp: 0, before, after, files: Vec::new() }
    }

    #[test]
    fn the_age_line_reads_as_a_human_would_say_it() {
        assert_eq!(ago(Duration::from_secs(20)), "moments ago");
        assert_eq!(ago(Duration::from_secs(60)), "1 minute ago");
        assert_eq!(ago(Duration::from_secs(7200)), "2 hours ago");
    }

    #[test]
    fn a_version_row_pairs_the_old_build_with_the_new_one_by_package() {
        let rows = version_rows(&ore(
            vec![build("jp.co.ponos.battlecatsen", "15.5.0", 155000)],
            vec![build("jp.co.ponos.battlecatsen", "15.6.0", 156000)],
        ));

        assert_eq!(rows[0].1, "jp.co.ponos.battlecatsen  15.5.0 -> 15.6.0 (156000)");
    }

    // A first import has nothing to pair against, and an unrelated package must not pair either.
    #[test]
    fn an_unpaired_build_reads_as_a_single_version() {
        let rows = version_rows(&ore(Vec::new(), vec![build("battlecats", "15.6.0", 156000)]));
        assert_eq!(rows[0].1, "battlecats  15.6.0 (156000)");

        let rows = version_rows(&ore(
            vec![build("jp.co.ponos.battlecats", "15.5.0", 155000)],
            vec![build("jp.co.ponos.battlecatsen", "15.6.0", 156000)],
        ));
        assert_eq!(rows[0].1, "jp.co.ponos.battlecatsen  15.6.0 (156000)");
    }

    // A multi-region import must name the build that actually served the winning file.
    #[test]
    fn the_source_names_the_build_matching_its_own_region() {
        let builds = vec![
            build("jp.co.ponos.battlecatsen", "15.5.0", 155000),
            build("jp.co.ponos.battlecats", "15.6.0", 156000),
        ];

        assert_eq!(source_line("ja", &builds), "Japan v15.6.0");
        assert_eq!(source_line("en", &builds), "Global v15.5.0");
        assert_eq!(source_line("--", &builds), "loose files");
    }

    // Surge and friends report a spawn width that reads as noise on a diff card.
    #[test]
    fn the_spawn_width_line_is_dropped_but_its_neighbours_survive() {
        assert!(ignored("Width: 400"));
        assert!(!ignored("Range: 400~800"));
        assert!(!ignored("Chance: 0% (+30%) -> 30%"));
        assert!(!ignored("Widthless"));
    }

    fn find(gained: &[bool]) -> talents::Find {
        talents::Find {
            cat_id: 1,
            fresh: false,
            gained: gained
                .iter()
                .map(|ultra| talents::Gain {
                    index: 0,
                    group: nyanko::cat::unit::TalentGroup { limit: u8::from(*ultra), ..Default::default() },
                    name: "",
                    fallback: "",
                    icon: AbilityIcon::None,
                    ultra: *ultra,
                })
                .collect(),
            retuned: Vec::new(),
        }
    }

    // Plain talents read first, ultra-only next, and the mixed units close the section.
    #[test]
    fn talent_cards_rank_plain_then_ultra_then_both() {
        assert_eq!(talent_rank(&find(&[false, false])), 0);
        assert_eq!(talent_rank(&find(&[true])), 1);
        assert_eq!(talent_rank(&find(&[false, true])), 2);
    }

    #[test]
    fn a_narrow_window_still_fits_one_card_per_row() {
        assert_eq!(cards_per_row(200.0, UNIT_MIN_WIDTH), 1);
        assert_eq!(cards_per_row(0.0, UNIT_MIN_WIDTH), 1);
        assert!(cards_per_row(1600.0, UNIT_MIN_WIDTH) > 1, "a wide window must fit more than one unit card");

    }

    #[test]
    fn a_raw_import_names_its_source_instead_of_a_region() {
        assert_eq!(region_name("en"), "Global");
        assert_eq!(region_name("--"), "loose files");
    }
}
