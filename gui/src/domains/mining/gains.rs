use super::*;
use iced::widget::{button, column, container, row, rule, text_input, Column, Space};

#[derive(Clone)]
struct Level {
    value: i32,
    fallback: i32,
    typed: String,
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

pub(super) fn kind_badge(find: &talents::Find) -> &'static str {
    match talent_kinds(find) {
        (true, true) => "TALENT+ULTRA",
        (false, true) => "ULTRA",
        _ => "TALENT",
    }
}

impl State {
    pub(super) fn view_find<'a>(
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
            inner = inner.push(value_row(&relabel(line)));
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

        let name: Element<'_, Message> = match skill_name::load(&self.plates, &gain.group, vfs, true) {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    // Ordering no longer separates ultra cards, so the badge is the only thing
    // telling a reader which kind of talent landed.
    #[test]
    fn the_badge_names_which_kinds_of_talent_landed() {
        assert_eq!(kind_badge(&find(&[false, false])), "TALENT");
        assert_eq!(kind_badge(&find(&[true])), "ULTRA");
        assert_eq!(kind_badge(&find(&[false, true])), "TALENT+ULTRA");
    }
}
