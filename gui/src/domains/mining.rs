use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, image as iced_image, row, rule, scrollable, text, text_input, Column, Row, Space, Text};
use iced::{Color, Element, Length, Size, Task, Theme};
use nyanko::cat::unit::{LevelCurve, TalentCost};
use nyanko::combat::Entity;

use kore::common::formats::SpriteSheet as CoreSpriteSheet;
use kore::common::region::Region;
use kore::domains::cat::files as cat_files;
use kore::domains::cat::game::stats as cat_stats;
use kore::domains::cat::game::talents as talent_logic;
use kore::domains::cat::scanner::{self as cat_scanner, CatEntry};
use kore::domains::mining::{self, talents, units, Build, Ore, Status};
use kore::domains::settings::Settings;
use kore::systems::combat::registry::{AbilityIcon, STAT_RARITY};
use kore::Vfs;

use crate::app::theme;
use crate::common::header_icon::{self, HeaderIcon};
use crate::common::{ability_icon, img015, skill_name, CustomAssets, SpriteSheet};
use crate::widget::{fallback_icon, list_row, section, smooth_scroll, uniform_grid};

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
    OpenUnit(u32),
    Img015Loaded(u64, usize, Option<CoreSpriteSheet>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Select(tab) => write!(f, "Select({:?})", tab),
            Self::LevelChanged(cat, value) => write!(f, "LevelChanged({}, {})", cat, value),
            Self::OpenTalents(cat, form) => write!(f, "OpenTalents({}, {})", cat, form),
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
            Message::OpenTalents(..) | Message::OpenUnit(..) => {}
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

    fn has_finds(&self) -> bool {
        self.has_talents() || self.has_fresh()
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
        vfs: &'a Vfs,
        settings: &'a Settings,
        window: Size,
    ) -> Element<'a, Message> {
        let body = match self.tab {
            Tab::Meta => self.view_meta(),
            Tab::Cats => self.view_cats(cats, vfs, settings, window.width - SIDEBAR_WIDTH),
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
        vfs: &'a Vfs,
        settings: &'a Settings,
        width: f32,
    ) -> Element<'a, Message> {
        let mut body = Column::new().spacing(SECTION_SPACING).width(Length::Fill);
        let mut listed = false;

        let fresh = self.fresh_units(cats, vfs, settings);

        if !fresh.is_empty() {
            listed = true;
            let cards: Vec<Element<'a, Message>> = fresh.into_iter().map(|cat| self.view_fresh(cat)).collect();

            body = body.push(section("New", Length::Fill, uniform_grid(cards, CARD_SPACING)));
        }

        if let Some(report) = &self.report
            && self.has_talents()
        {
            listed = true;

            let cards: Vec<Element<'a, Message>> =
                report.finds.iter().map(|find| self.view_find(find, cats, vfs, settings)).collect();

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

        let mut talents = Column::new().spacing(CARD_SPACING).width(Length::Fill);

        for gain in &find.gained {
            talents = talents.push(self.view_gain(gain, false, &unit, vfs));
        }

        for retune in &find.retuned {
            talents = talents.push(self.view_gain(&retune.gain, true, &unit, vfs));
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
        match gain.icon {
            AbilityIcon::Custom(custom) => {
                if let Some(handle) = self.assets.get_icon_texture(custom) {
                    return iced_image(handle)
                        .width(Length::Fixed(TALENT_ICON_SIZE))
                        .height(Length::Fixed(TALENT_ICON_SIZE))
                        .into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.icons.handle(icon_id, &self.img015_sheets) {
                    return iced_image(handle)
                        .width(Length::Fixed(TALENT_ICON_SIZE))
                        .height(Length::Fixed(TALENT_ICON_SIZE))
                        .into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon(gain.fallback)
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

fn conjured(cats: &[CatEntry]) -> HashSet<u32> {
    cats.iter()
        .flat_map(|entry| entry.stats.iter().flatten())
        .filter_map(|stats| u32::try_from(stats.conjure_unit_id).ok())
        .collect()
}

fn kind_badge(find: &talents::Find) -> &'static str {
    let mut normal = false;
    let mut ultra = false;

    for gain in find.gained.iter().chain(find.retuned.iter().map(|retune| &retune.gain)) {
        if gain.ultra {
            ultra = true;
        } else {
            normal = true;
        }
    }

    match (normal, ultra) {
        (true, true) => "TALENT+ULTRA",
        (false, true) => "ULTRA",
        _ => "TALENT",
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
