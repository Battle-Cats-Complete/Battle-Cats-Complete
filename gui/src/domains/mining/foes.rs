use super::*;
use iced::widget::{button, column, container, row, rule, Column, Space};

impl State {
    pub(super) fn view_enemies<'a>(&'a self, roster: &'a [EnemyEntry], width: f32) -> Element<'a, Message> {
        let mut body = Column::new().spacing(SECTION_SPACING).width(Length::Fill);

        if !self.ready.foes_new.is_empty() {
            body = body.push(self.view_fold(Tab::Enemies, "New", self.ready.foes_new.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> =
                    self.ready.foes_new.iter().map(|id| self.view_foe(*id, None, roster)).collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, LEVEL_MIN_WIDTH)).into()
            }));
        }

        if !self.ready.foes_spoken.is_empty() {
            body = body.push(self.view_fold(Tab::Enemies, "Localized", self.ready.foes_spoken.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .foes_spoken
                    .iter()
                    .filter_map(|slot| self.spoken_foes.get(*slot))
                    .map(|held| self.view_spoken_foe(held, roster))
                    .collect();

                uniform_grid(cards, CARD_SPACING).into()
            }));
        }

        if !self.ready.foes_changed.is_empty() {
            body = body.push(self.view_fold(Tab::Enemies, "Changed", self.ready.foes_changed.len(), Horizontal::Left, || {
                let cards: Vec<Element<'a, Message>> = self
                    .ready
                    .foes_changed
                    .iter()
                    .filter_map(|(slot, diff)| self.foes.get(*slot).map(|foe| (foe, diff)))
                    .map(|(foe, diff)| self.view_foe(foe.enemy_id, Some((foe, diff)), roster))
                    .collect();

                uniform_grid(cards, CARD_SPACING).columns(cards_per_row(width, UNIT_MIN_WIDTH)).into()
            }));
        }

        body.into()
    }

    fn view_foe_identity<'a>(&'a self, enemy_id: u32, entry: Option<&'a EnemyEntry>) -> Element<'a, Message> {
        let icon = self.ready.foe_art.get(&enemy_id).and_then(|path| self.thumbnail(path));
        let name = entry.map_or_else(|| format!("Enemy {:03}", enemy_id), EnemyEntry::display_name);

        let portrait: Element<'a, Message> = match icon {
            Some(handle) => iced_image(handle)
                .width(Length::Fixed(PORTRAIT_SIZE))
                .height(Length::Fixed(PORTRAIT_SIZE))
                .into(),
            None => Space::new().width(Length::Fixed(PORTRAIT_SIZE)).height(Length::Fixed(PORTRAIT_SIZE)).into(),
        };

        row![
            container(portrait)
                .width(Length::Fixed(PORTRAIT_SIZE))
                .height(Length::Fixed(PORTRAIT_SIZE))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            theme::bold_text(name).size(UNIT_NAME_SIZE),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center)
        .into()
    }

    fn view_spoken_foe<'a>(
        &'a self,
        held: &'a localized::Localized,
        roster: &'a [EnemyEntry],
    ) -> Element<'a, Message> {
        let entry = roster.iter().find(|known| known.id == held.id);

        let identity = button(self.view_foe_identity(held.id, entry))
            .padding(0)
            .style(button::text)
            .on_press_maybe(entry.map(|_| Message::OpenEnemy(held.id)));

        self.view_tongues(identity.into(), held)
    }

    fn view_foe<'a>(
        &'a self,
        enemy_id: u32,
        changed: Option<(&'a enemies::Changed, &'a forms::Diff)>,
        roster: &'a [EnemyEntry],
    ) -> Element<'a, Message> {
        let entry = self.foe(roster, enemy_id);
        let stats = changed.map(|(foe, _)| &foe.current).or(entry.map(|held| &held.stats));

        let identity = button(self.view_foe_identity(enemy_id, entry))
            .padding(0)
            .style(button::text)
            .on_press_maybe(entry.map(|_| Message::OpenEnemy(enemy_id)));

        let icons = Icons { cache: &self.icons, sheets: &self.img015_sheets, assets: &self.assets };

        let mut header = row![
            light_box(identity, Length::Fixed(UNIT_BOX_HEIGHT)),
            light_box(strong(format!("ID: {}", enemy_id), UNIT_NAME_SIZE), Length::Fixed(UNIT_BOX_HEIGHT)),
        ]
        .spacing(CELL_SPACING)
        .align_y(Vertical::Center);

        if let Some(stats) = stats {
            header = header.push(light_box(trait_row(stats, &icons), Length::Fixed(UNIT_BOX_HEIGHT)));
        }

        let Some((_, diff)) = changed else {
            return container(header).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into();
        };

        let mut body = Column::new().spacing(CARD_SPACING).width(Length::Fill);
        body = body.extend(panels(diff, &icons));

        let card = column![header, rule::horizontal(1), body].spacing(10).width(Length::Fill);

        container(card).padding(CARD_PADDING).width(Length::Fill).style(theme::card_container).into()
    }
}
