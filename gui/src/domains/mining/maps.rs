use super::*;
use iced::widget::{button, column, container, row, rule, text, Column};

struct Lands<'a, 'b> {
    moves: &'b HashMap<GlobalMapId, (u8, u8)>,
    tongues: &'b HashMap<GlobalMapId, Vec<String>>,
    banners: &'b HashMap<GlobalMapId, PathBuf>,
    plates: &'b HashMap<GlobalStageId, PathBuf>,
    registry: &'a StageRegistry,
}

pub(super) type Grouped = Vec<(Category, BTreeMap<u32, Vec<u32>>)>;

pub(super) fn grouped<'a>(found: impl Iterator<Item = &'a stages::Located>, registry: &StageRegistry) -> Grouped {
    let placed = found.map(|entry| {
        let key = GlobalMapId { category: Category::from_prefix(&entry.prefix), map: entry.map };

        (key, entry.stage)
    });

    gather(placed, registry)
}

pub(super) fn gather(found: impl Iterator<Item = (GlobalMapId, Option<u32>)>, registry: &StageRegistry) -> Grouped {
    let mut raw: HashMap<Category, BTreeMap<u32, (bool, Vec<u32>)>> = HashMap::new();

    for (key, stage) in found {
        if !registry.maps.contains_key(&key) {
            continue;
        }

        let slot = raw.entry(key.category).or_default().entry(key.map).or_insert((false, Vec::new()));

        match stage {
            None => slot.0 = true,
            Some(id) => slot.1.push(id),
        }
    }

    settle(raw, registry)
}

pub(super) fn settle(raw: HashMap<Category, BTreeMap<u32, (bool, Vec<u32>)>>, registry: &StageRegistry) -> Grouped {
    let mut all: Grouped = Vec::new();

    for (category, maps) in raw {
        let mut kept: BTreeMap<u32, Vec<u32>> = BTreeMap::new();

        for (map, (whole, mut listed)) in maps {
            let key = GlobalMapId { category: category.clone(), map };
            let held = registry.maps.get(&key);

            if whole {
                listed.extend(held.map(|found| found.stages.clone()).unwrap_or_default());
            }

            if let Some(found) = held {
                listed.retain(|stage| found.stages.contains(stage));
            }

            listed.sort_unstable();
            listed.dedup();

            if !listed.is_empty() {
                kept.insert(map, listed);
            }
        }

        if !kept.is_empty() {
            all.push((category, kept));
        }
    }

    all.sort_by_key(|(category, _)| category.sort_order());

    all
}

fn crown_shift<'a>(before: u8, after: u8) -> Element<'a, Message> {
    let crown = || {
        text(CROWN_GLYPH)
            .font(fonts::MISC_SYMBOLS)
            .size(VALUE_TEXT_SIZE)
            .line_height(fonts::MISC_SYMBOLS_LINE_HEIGHT)
            .color(CHIP_TEXT)
    };

    row![
        strong(before, VALUE_TEXT_SIZE).color(CHIP_TEXT),
        crown(),
        strong("->", VALUE_TEXT_SIZE).color(CHIP_TEXT),
        strong(after, VALUE_TEXT_SIZE).color(CHIP_TEXT),
        crown(),
    ]
    .spacing(2)
    .align_y(Vertical::Center)
    .into()
}

pub(super) fn map_art(map: u32, prefix: &str) -> String {
    if prefix.is_empty() {
        return format!("mapname{:03}.png", map);
    }

    format!("mapname{:03}_{}.png", map, prefix)
}

pub(super) fn stage_art(map: u32, stage: u32, prefix: &str) -> String {
    if prefix.is_empty() {
        return format!("mapsn{:03}_{:02}.png", map, stage);
    }

    format!("mapsn{:03}_{:02}_{}.png", map, stage, prefix)
}

pub(super) fn subchapters(grouped: &Grouped) -> usize {
    grouped.iter().map(|(_, maps)| maps.len()).sum()
}

pub(super) fn stage_count(grouped: &Grouped) -> usize {
    grouped.iter().flat_map(|(_, maps)| maps.values()).map(Vec::len).sum()
}

impl State {
    pub(super) fn view_stages<'a>(&'a self, registry: &'a StageRegistry, width: f32) -> Element<'a, Message> {
        let mut body = Column::new().spacing(SECTION_SPACING).width(Length::Fill);

        let lands = Lands {
            moves: &self.terrain.moves,
            tongues: &self.terrain.tongues,
            banners: &self.terrain.banners,
            plates: &self.terrain.plates,
            registry,
        };

        for (title, grouped, seam) in [
            ("New", &self.terrain.fresh, Seam::Fresh),
            ("Localized", &self.terrain.spoken, Seam::Spoken),
            ("Crowns", &self.terrain.crowned, Seam::Crowned),
            ("Changed", &self.terrain.moved, Seam::Moved),
        ] {
            if grouped.is_empty() {
                continue;
            }

            body = body.push(self.view_fold(Tab::Stages, title, stage_count(grouped), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = grouped
                    .iter()
                    .map(|(category, maps)| self.view_land(category, maps, seam, &lands))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, STAGE_MIN_WIDTH)).into()
            }));
        }

        body.into()
    }

    fn view_land<'a>(
        &'a self,
        category: &'a Category,
        maps: &'a BTreeMap<u32, Vec<u32>>,
        seam: Seam,
        lands: &Lands<'a, '_>,
    ) -> Element<'a, Message> {
        let heading = button(strong(category.display_name(), UNIT_NAME_SIZE))
            .padding(0)
            .style(button::text)
            .on_press(Message::OpenCategory(category.clone()));

        let mut crown = row![light_box(heading, Length::Fixed(UNIT_BOX_HEIGHT))]
            .spacing(CELL_SPACING)
            .align_y(Vertical::Center);

        let letters = category.map_prefix();

        if letters != category.display_name() {
            crown = crown.push(light_box(strong(letters, UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)));
        }

        let mut body = Column::new().spacing(CARD_SPACING).width(Length::Fill);

        for (map, listed) in maps {
            body = body.push(self.view_subchapter(category, *map, listed, seam, lands));
        }

        let card = column![crown, rule::horizontal(1), body].spacing(10).width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }

    fn view_subchapter<'a>(
        &'a self,
        category: &Category,
        map: u32,
        listed: &[u32],
        seam: Seam,
        lands: &Lands<'a, '_>,
    ) -> Element<'a, Message> {
        let key = GlobalMapId { category: category.clone(), map };
        let held = lands.registry.maps.get(&key);

        let banner = lands
            .banners
            .get(&key)
            .and_then(|path| self.plate(path))
            .map(|handle| iced_image(handle).height(Length::Fixed(NAME_PLATE_HEIGHT)).into());

        let label: Element<'a, Message> = banner.unwrap_or_else(|| {
            let name = held.map_or_else(|| format!("Map {:03}", map), |found| found.name.clone());

            strong(name, VALUE_TEXT_SIZE).into()
        });

        let heading = button(label).padding(0).style(button::text).on_press(Message::OpenMap(key.clone()));

        let mut crest = row![heading, dark_box(strong(format!("ID: {}", map), VALUE_TEXT_SIZE).color(CHIP_TEXT), Length::Shrink)]
            .spacing(CELL_SPACING)
            .align_y(Vertical::Center);

        let mut crown = None;

        if seam == Seam::Crowned
            && let Some((before, after)) = lands.moves.get(&key)
        {
            crest = crest.push(dark_box(crown_shift(*before, *after), Length::Shrink));
            crown = Some(after.saturating_sub(1));
        }

        if seam == Seam::Spoken
            && let Some(languages) = lands.tongues.get(&key)
        {
            crest = crest.push(dark_box(
                theme::bold_text(tongues(languages)).size(VALUE_TEXT_SIZE).color(CHIP_TEXT),
                Length::Shrink,
            ));
        }

        let chips: Vec<Element<'a, Message>> = listed
            .iter()
            .map(|stage| {
                let id = GlobalStageId { category: category.clone(), map, stage: *stage };

                self.view_stage_chip(id, crown, lands)
            })
            .collect();

        let tint = if seam.tinted() { ADDITION_TINT } else { NORMAL_TINT };

        let block = column![crest, ledger_rule(), uniform_grid(chips, ABILITY_ICON_GAP)]
            .spacing(6)
            .width(Length::Fill);

        container(block)
            .padding(CHIP_PADDING)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(tint.into()),
                border: iced::border::rounded(CHIP_RADIUS),
                ..Default::default()
            })
            .into()
    }

    fn view_stage_chip<'a>(
        &'a self,
        key: GlobalStageId,
        crown: Option<u8>,
        lands: &Lands<'a, '_>,
    ) -> Element<'a, Message> {
        let stage = key.stage;

        let label: Element<'a, Message> = lands
            .plates
            .get(&key)
            .and_then(|path| self.plate(path))
            .map_or_else(
                || {
                    let name = lands
                        .registry
                        .stages
                        .get(&key)
                        .map_or_else(|| format!("Stage {:02}", stage), |found| found.name.clone());

                    container(strong(name, VALUE_TEXT_SIZE).color(CHIP_TEXT))
                        .width(Length::Fill)
                        .center_y(Length::Fixed(NAME_PLATE_HEIGHT))
                        .align_x(Horizontal::Left)
                        .into()
                },
                |handle| {
                    container(
                        iced_image(handle)
                            .height(Length::Fixed(NAME_PLATE_HEIGHT))
                            .content_fit(ContentFit::Contain),
                    )
                    .width(Length::Fill)
                    .align_x(Horizontal::Left)
                    .into()
                },
            );

        button(label)
            .padding(0)
            .width(Length::Fill)
            .style(button::text)
            .on_press(Message::OpenStage(key, crown))
            .into()
    }
}
