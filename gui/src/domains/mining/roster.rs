use super::*;
use iced::widget::{button, column, container, row, rule, Column, Row, Space};

pub(super) struct Icons<'a> {
    pub(super) cache: &'a ability_icon::Cache,
    pub(super) sheets: &'a [SpriteSheet],
    pub(super) assets: &'a CustomAssets,
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

pub(super) struct UnitContext<'a> {
    pub(super) base: Option<&'a Entity>,
    pub(super) curve: Option<&'a LevelCurve>,
    pub(super) costs: Option<&'a HashMap<u8, TalentCost>>,
    pub(super) level: i32,
}

pub(super) fn changed_diff<'a>(
    changed: &'a changes::Changed,
    cat: Option<&'a CatEntry>,
    global: GlobalContext<'a>,
    settings: &Settings,
) -> forms::Diff {
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

pub(super) fn form_diff<'a>(cat: &'a CatEntry, form: usize, global: GlobalContext<'a>, settings: &Settings) -> forms::Diff {
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

pub(super) fn panels<'a>(diff: &forms::Diff, icons: &Icons<'_>) -> Vec<Element<'a, Message>> {
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

fn change_row<'a>(change: &forms::Change) -> Element<'a, Message> {
    let mut line = Row::new().spacing(VALUE_LABEL_GAP).align_y(Vertical::Center);

    line = line.push(plain(change.label, VALUE_TEXT_SIZE).color(CHIP_TEXT));
    line = line.push(strong(&change.before, VALUE_TEXT_SIZE).color(CHIP_TEXT));

    if let Some(shift) = &change.shift {
        line = line.push(strong(shift, VALUE_TEXT_SIZE).color(CHIP_TEXT));
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
    hinted(icons.render(ability.icon, ability.fallback), &ability.text)
}

fn explained_ability<'a>(ability: &forms::Ability, icons: &Icons<'_>) -> Element<'a, Message> {
    let heading = row![
        icons.render(ability.icon, ability.fallback),
        strong(ability.name, VALUE_TEXT_SIZE).color(CHIP_TEXT),
    ]
    .spacing(4)
    .align_y(Vertical::Center);

    if ability.detail.is_empty() && !ability.text.is_empty() {
        return column![heading, tinted_superscript(&ability.text, VALUE_TEXT_SIZE, Some(CHIP_TEXT))]
            .spacing(2)
            .width(Length::Fill)
            .into();
    }

    let mut block = Column::new().spacing(2).width(Length::Fill).push(hinted(heading, &ability.text));

    for change in &ability.detail {
        block = block.push(change_row(change));
    }

    block.into()
}

pub(super) fn strict_config(settings: &Settings) -> ScannerConfig {
    let mut config = settings.scanner_config(None);
    config.show_invalid_cats = false;
    config.show_invalid_enemies = false;
    config.pristine = true;

    config
}

pub(super) fn trait_row<'a>(stats: &Entity, icons: &Icons<'_>) -> Element<'a, Message> {
    let mut carried = Row::new().spacing(ABILITY_ICON_GAP).align_y(Vertical::Center);
    let mut found = false;

    for pure in REGISTRY {
        let display = get_display_def(pure.identity);

        if !is_trait(pure.identity) || (pure.attributes)(stats).is_empty() {
            continue;
        }

        carried = carried.push(icons.render(display.icon, display.fallback));
        found = true;
    }

    if !found {
        return strong("None", UNIT_NAME_SIZE).into();
    }

    carried.into()
}

pub(super) fn conjured(cats: &[CatEntry]) -> HashSet<u32> {
    cats.iter()
        .flat_map(|entry| entry.stats.iter().flatten())
        .filter_map(|stats| u32::try_from(stats.conjure_unit_id).ok())
        .collect()
}

pub(super) fn top_form(cat: &CatEntry) -> usize {
    (0..cat.forms.len()).rev().find(|slot| cat.forms[*slot]).unwrap_or(FIRST_FORM)
}

pub(super) fn portrait_form(cat: &CatEntry, ultra: bool) -> usize {
    let ceiling = if ultra { 3 } else { 2 };

    (0..=ceiling)
        .rev()
        .find(|&form| cat.forms.get(form).copied().unwrap_or(false))
        .or_else(|| (0..cat.forms.len()).rev().find(|&form| cat.forms[form]))
        .unwrap_or(0)
}

impl State {
    pub(super) fn view_cats<'a>(
        &'a self,
        cats: &'a [CatEntry],
        vfs: &'a Vfs,
        settings: &'a Settings,
        width: f32,
    ) -> Element<'a, Message> {
        let mut body = Column::new().spacing(SECTION_SPACING).width(Length::Fill);
        let mut listed = false;

        if !self.ready.fresh.is_empty() {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "New", self.ready.fresh.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .fresh
                    .iter()
                    .filter_map(|id| self.entry(cats, *id))
                    .map(|cat| self.view_fresh(cat))
                    .collect();

                uniform_grid(cards, CARD_SPACING).into()
            }));
        }

        if !self.ready.spoken.is_empty() {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "Localized", self.ready.spoken.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .spoken
                    .iter()
                    .filter_map(|slot| self.spoken_cats.get(*slot))
                    .map(|held| self.view_spoken(held, cats))
                    .collect();

                uniform_grid(cards, CARD_SPACING).into()
            }));
        }

        if !self.ready.changed.is_empty() {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "Changes", self.ready.changed.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .changed
                    .iter()
                    .filter_map(|(slot, diff)| self.changes.get(*slot).map(|changed| (changed, diff)))
                    .map(|(changed, diff)| self.view_changed(changed, cats, diff))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, UNIT_MIN_WIDTH)).into()
            }));
        }

        if !self.ready.unlocked.is_empty() {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "Forms", self.ready.unlocked.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .unlocked
                    .iter()
                    .filter_map(|(slot, diff)| self.forms.get(*slot).map(|held| (held, diff)))
                    .map(|(held, diff)| self.view_unlocked(held, cats, diff))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, UNIT_MIN_WIDTH)).into()
            }));
        }

        if let Some(report) = &self.report
            && !self.ready.talents.is_empty()
        {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "Talents", self.ready.talents.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .talents
                    .iter()
                    .filter_map(|slot| report.finds.get(*slot))
                    .map(|find| self.view_find(find, cats, vfs, settings))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, UNIT_MIN_WIDTH)).into()
            }));
        }

        if !self.ready.raised.is_empty() {
            listed = true;

            body = body.push(self.view_fold(Tab::Cats, "Levels", self.ready.raised.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .raised
                    .iter()
                    .filter_map(|slot| self.levels_raised.get(*slot))
                    .map(|entry| self.view_raised(entry, cats))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, LEVEL_MIN_WIDTH)).into()
            }));
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

    pub(super) fn spoken_form(&self, cats: &[CatEntry], held: &localized::Localized) -> usize {
        held.form.unwrap_or_else(|| self.entry(cats, held.id).map_or(FIRST_FORM, top_form))
    }

    fn view_spoken<'a>(&'a self, held: &'a localized::Localized, cats: &'a [CatEntry]) -> Element<'a, Message> {
        let cat = self.entry(cats, held.id);
        let form = self.spoken_form(cats, held);

        let identity = button(self.view_identity(cat, form, held.id))
            .padding(0)
            .style(button::text)
            .on_press_maybe(cat.map(|_| Message::OpenUnit(held.id)));

        self.view_tongues(identity.into(), held)
    }

    pub(super) fn view_tongues<'a>(
        &'a self,
        identity: Element<'a, Message>,
        held: &'a localized::Localized,
    ) -> Element<'a, Message> {
        let mut card = row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(format!("ID: {}", held.id), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center);

        if let Some(form) = held.form {
            let badge = format!("{} FORM", cat_files::form_name(form).to_uppercase());

            card = card.push(light_box(strong(badge, UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)));
        }

        let card = card.push(light_box(
            strong(tongues(&held.languages), UNIT_NAME_SIZE),
            Length::Fixed(UNIT_BOX_HEIGHT),
        ));

        container(card).padding(CARD_PADDING).width(Length::Shrink).style(theme::card_container).into()
    }

    fn view_raised<'a>(&'a self, raised: &'a levels::Raised, cats: &'a [CatEntry]) -> Element<'a, Message> {
        let cat = cats.iter().find(|entry| entry.id == raised.cat_id);
        let form = cat.map_or(FIRST_FORM, top_form);
        let rarity = cat.map_or_else(|| "??".to_string(), |entry| (STAT_RARITY.formatter)(entry.unitbuy.rarity));

        let identity = button(self.view_identity(cat, form, raised.cat_id))
            .padding(0)
            .style(button::text)
            .on_press_maybe(cat.map(|_| Message::OpenUnit(raised.cat_id)));

        let header = row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(format!("ID: {}", raised.cat_id), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(rarity, UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center);

        let reading = row![
            strong(raised.before.label(), UNIT_NAME_SIZE),
            strong("->", UNIT_NAME_SIZE),
            strong(raised.after.label(), UNIT_NAME_SIZE),
        ]
        .spacing(VALUE_LABEL_GAP)
        .align_y(Vertical::Center);

        let card = column![header, rule::horizontal(1), reading].spacing(10).width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }

    pub(super) fn view_identity<'a>(&'a self, cat: Option<&'a CatEntry>, form: usize, id: u32) -> Element<'a, Message> {
        let icon = self.ready.art.get(&(id, form)).and_then(|path| self.portrait(path));

        let name = cat.map_or_else(|| format!("Unknown unit {:03}", id), |entry| entry.display_name(form));

        let body: Element<'a, Message> = match icon {
            Some(handle) => iced_image(handle)
                .width(Length::Fixed(PORTRAIT_SIZE))
                .height(Length::Fixed(PORTRAIT_SIZE))
                .into(),
            None => Space::new().width(Length::Fixed(PORTRAIT_SIZE)).height(Length::Fixed(PORTRAIT_SIZE)).into(),
        };

        let frame = container(body)
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
        diff: &'a forms::Diff,
    ) -> Element<'a, Message> {
        let cat = self.entry(cats, changed.cat_id);
        let badge = format!("{} FORM", cat_files::form_name(changed.form).to_uppercase());

        self.view_diff_card(cat, changed.cat_id, changed.form, badge, diff)
    }

    fn view_unlocked<'a>(
        &'a self,
        unlocked: &'a forms::Unlocked,
        cats: &'a [CatEntry],
        diff: &'a forms::Diff,
    ) -> Element<'a, Message> {
        let cat = self.entry(cats, unlocked.cat_id);
        let badge = if unlocked.form == forms::ULTRA_FORM { "ULTRA FORM" } else { "TRUE FORM" };

        self.view_diff_card(cat, unlocked.cat_id, unlocked.form, badge.to_string(), diff)
    }

    fn view_diff_card<'a>(
        &'a self,
        cat: Option<&'a CatEntry>,
        cat_id: u32,
        form: usize,
        badge: String,
        diff: &'a forms::Diff,
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

        body = body.extend(panels(diff, &icons));

        if let Some(spirit) = &diff.spirit
            && !spirit.is_empty()
        {
            body = body.push(strong("Conjure", LEDGER_TITLE_SIZE));
            body = body.extend(panels(spirit, &icons));
        }

        let card = column![header, rule::horizontal(1), body].spacing(10).width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }
}
