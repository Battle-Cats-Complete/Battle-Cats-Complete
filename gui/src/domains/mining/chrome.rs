use super::*;
use iced::widget::{button, column, container, row, rule, scrollable, stack, text, Column, Row, Space};

fn version_tables<'a>(ore: &Diff) -> Vec<Element<'a, Message>> {
    let mut stack = Vec::new();

    for (label, suffix) in REGIONS {
        let Some(build) = build_in(ore, suffix) else {
            continue;
        };

        let header = container(theme::table_cell_text(*label, Length::Fixed(TABLE_CELL_WIDTH)).size(META_TEXT_SIZE))
            .center_y(Length::Fixed(TABLE_ROW_HEIGHT))
            .style(theme::zebra_table_header);

        stack.push(
            column![
                header,
                zebra_cell(&build.label, 0),
                zebra_cell(build.code, 1),
                zebra_cell(format!("v{}", build.name), 2),
            ]
            .align_x(Horizontal::Center)
            .width(Length::Shrink)
            .into(),
        );
    }

    stack
}

pub(super) fn build_in<'a>(ore: &'a Diff, suffix: &str) -> Option<&'a Build> {
    ore.after.iter().chain(ore.before.iter()).find(|build| build.label.ends_with(suffix))
}

pub(super) fn ago(age: Duration) -> String {
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

impl State {
    pub(super) fn enabled(&self, tab: Tab) -> bool {
        match tab {
            Tab::Meta => true,
            Tab::Cats => self.has_finds(),
            Tab::Enemies => {
                !self.ready.foes_new.is_empty()
                    || !self.ready.foes_changed.is_empty()
                    || !self.ready.foes_spoken.is_empty()
            }
            Tab::Stages => {
                !self.terrain.fresh.is_empty()
                    || !self.terrain.spoken.is_empty()
                    || !self.terrain.moved.is_empty()
                    || !self.terrain.crowned.is_empty()
            }
            Tab::Files => !self.files.is_empty(),
        }
    }

    pub(super) fn folded(&self, tab: Tab, title: &'static str, count: usize) -> bool {
        self.folds.get(&(tab, title)).copied().unwrap_or(count > FOLD_LIMIT)
    }

    pub(super) fn view_fold<'a>(
        &'a self,
        tab: Tab,
        title: &'static str,
        count: usize,
        align: Horizontal,
        content: impl FnOnce() -> Element<'a, Message>,
    ) -> Element<'a, Message> {
        let folded = self.folded(tab, title, count);

        let mut heading = row![strong(title, FOLD_TITLE_SIZE)].spacing(CELL_SPACING).align_y(Vertical::Bottom);

        if folded {
            heading = heading.push(
                plain("(collapsed)", FOLD_NOTE_SIZE)
                    .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) }),
            );
        }

        let mut body = Column::new().spacing(SECTION_HEAD_GAP).width(Length::Fill).align_x(align);

        body = body.push(
            button(heading).padding(0).style(button::text).on_press(Message::Fold(tab, title, !folded)),
        );

        body = body.push(rule::horizontal(1));

        if !folded {
            body = body.push(content());
        }

        body.into()
    }

    pub fn view<'a>(
        &'a self,
        cats: &'a [CatEntry],
        foes: &'a [EnemyEntry],
        registry: &'a StageRegistry,
        global: GlobalContext<'a>,
        settings: &'a Settings,
        window: Size,
    ) -> Element<'a, Message> {
        let body = match self.tab {
            Tab::Meta => self.view_meta(window),
            Tab::Cats => self.view_cats(cats, &global.vault.vfs, settings, window.width - SIDEBAR_WIDTH),
            Tab::Enemies => self.view_enemies(foes, window.width - SIDEBAR_WIDTH),
            Tab::Stages => self.view_stages(registry, window.width - SIDEBAR_WIDTH),
            Tab::Files => self.view_files(window),
        };

        let page = smooth_scroll(
            scrollable(container(body).padding(PAGE_PADDING).width(Length::Fill))
                .id(Id::new(SCROLL_ID))
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .spacing(SCROLLBAR_GAP)
                .width(Length::Fill)
                .height(Length::Fill),
        );

        let mut layers = stack![page];

        if self.tab == Tab::Meta && self.actionable() {
            layers = layers.push(self.view_actions());
        }

        row![self.view_sidebar(), layers.width(Length::Fill).height(Length::Fill)]
            .height(Length::Fill)
            .into()
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

    fn view_meta<'a>(&'a self, window: Size) -> Element<'a, Message> {
        let room = window.height - PAGE_PADDING * 2.0;
        let width = window.width - SIDEBAR_WIDTH;

        let Some(held) = self.diff.as_ref().filter(|_| self.has_diff()) else {
            let (headline, hint, action) = if self.snapped {
                (
                    "Nothing has changed since your snapshot",
                    "Import an update, or click \"Diff Snapshot\" to check again",
                    (Chore::Diff, "Diff Snapshot", Message::CreateDiff),
                )
            } else {
                (
                    "No changes to report yet",
                    "Import an update, or take a snapshot to track your own changes",
                    (Chore::Snapshot, "Create Snapshot", Message::CreateBase),
                )
            };

            return self.view_empty(headline, hint, self.diggable.then_some(action), room);
        };

        let panes = 2;

        let usable = (width - PAGE_PADDING * 2.0 - SCROLLBAR_RESERVE).max(META_MIN_WIDTH);
        let columns = fits(usable, META_MIN_WIDTH, SECTION_SPACING).min(panes);
        let room = (usable - SECTION_SPACING * (columns - 1) as f32) / columns as f32;

        let known = column![
            stamp_table(),
            wrapped(version_tables(held), room, TABLE_CELL_WIDTH, CARD_SPACING),
        ]
        .spacing(CARD_SPACING)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

        let summary = packed(self.view_tally(), room, TALLY_TABLE_WIDTH, CARD_SPACING);

        let shown: Vec<Element<'a, Message>> =
            vec![placard("Information", known, room), placard("Summary", summary, room)];

        let body = wrapped(shown, usable, room, SECTION_SPACING);

        if !self.actionable() {
            return body;
        }

        column![body, Space::new().height(ACTION_CLEARANCE)].width(Length::Fill).into()
    }

    fn actionable(&self) -> bool {
        self.diff.is_some() && self.has_diff()
    }

    fn view_actions(&self) -> Element<'_, Message> {
        let wipe = if self.wipe.is_set() { CONFIRM_LABEL } else { "Clear Diff" };

        let mut buttons = Row::new().spacing(CELL_SPACING);

        if self.diggable {
            buttons = buttons.push(if self.snapped {
                self.view_action(Chore::Snapshot, "Update Snapshot", Message::CreateBase)
            } else {
                self.view_action(Chore::Snapshot, "Create Snapshot", Message::CreateBase)
            });

            buttons = buttons.push(if self.snapped {
                self.view_action(Chore::Diff, "Diff Snapshot", Message::CreateDiff)
            } else {
                self.view_inert("No Snapshot")
            });
        }

        let buttons = buttons.push(self.view_chore(wipe, theme::danger_button, Message::ClearDiff));

        container(buttons)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Bottom)
            .padding(ACTION_PADDING)
            .into()
    }

    fn view_action<'a>(&'a self, chore: Chore, label: &'a str, message: Message) -> Element<'a, Message> {
        let busy = self.chore == Some(chore);
        let done = self.outcome.get().copied().filter(|(held, _)| *held == chore).map(|(_, kept)| kept);

        let (label, style): (&str, theme::ButtonStyleFn) = match (busy, done) {
            (true, _) => (self.busy_label(chore), theme::warning_button),
            (false, Some(true)) => (self.struck_label(chore), theme::success_button),
            (false, Some(false)) => (self.barren_label(chore), theme::danger_button),
            (false, None) => (label, theme::primary_button),
        };

        self.view_chore(label, style, message)
    }

    fn busy_label(&self, chore: Chore) -> &'static str {
        match chore {
            Chore::Snapshot if self.creating => "Creating Snapshot...",
            Chore::Snapshot => "Updating Snapshot...",
            Chore::Diff => "Diffing Snapshot...",
        }
    }

    fn struck_label(&self, chore: Chore) -> &'static str {
        match chore {
            Chore::Snapshot if self.creating => FOUNDED_LABEL,
            Chore::Snapshot => RAISED_LABEL,
            Chore::Diff => STRUCK_LABEL,
        }
    }

    fn barren_label(&self, chore: Chore) -> &'static str {
        match chore {
            Chore::Snapshot => STILL_LABEL,
            Chore::Diff => BARREN_LABEL,
        }
    }

    fn view_inert<'a>(&'a self, label: &'a str) -> Element<'a, Message> {
        button(theme::centered_text(label).size(TAB_TEXT_SIZE))
            .padding([6, 16])
            .width(Length::Fixed(theme::ACTION_BUTTON_WIDTH))
            .style(theme::neutral_button)
            .into()
    }

    fn view_chore<'a>(&'a self, label: &'a str, style: theme::ButtonStyleFn, message: Message) -> Element<'a, Message> {
        button(theme::centered_text(label).size(TAB_TEXT_SIZE))
            .padding([6, 16])
            .width(Length::Fixed(theme::ACTION_BUTTON_WIDTH))
            .style(style)
            .on_press_maybe(self.chore.is_none().then_some(message))
            .into()
    }

    fn view_empty<'a>(
        &'a self,
        headline: &'a str,
        hint: &'a str,
        action: Option<(Chore, &'a str, Message)>,
        room: f32,
    ) -> Element<'a, Message> {
        let mut body = Column::new().spacing(10).align_x(Horizontal::Center);

        body = body.push(plain(headline, EMPTY_TEXT_SIZE).align_x(Horizontal::Center));

        if let Some((chore, label, message)) = action {
            body = body.push(plain(hint, EMPTY_TEXT_SIZE).align_x(Horizontal::Center));
            body = body.push(self.view_action(chore, label, message));
        }

        container(body).center_x(Length::Fill).center_y(Length::Fixed(room.max(PORTRAIT_SIZE))).into()
    }

    fn view_tally<'a>(&'a self) -> Vec<Element<'a, Message>> {
        let forms = |slot: usize| {
            self.ready
                .unlocked
                .iter()
                .filter(|(held, _)| self.forms.get(*held).is_some_and(|found| found.form == slot))
                .count()
        };

        let talents = |ultra: bool| {
            self.report.as_ref().map_or(0, |report| {
                self.ready
                    .talents
                    .iter()
                    .filter_map(|slot| report.finds.get(*slot))
                    .filter(|find| find.gained.iter().any(|gain| gain.ultra == ultra))
                    .count()
            })
        };

        let cats = vec![
            ("New Count", self.ready.fresh.len()),
            ("Changed Count", self.ready.changed.len()),
            ("Localized Count", self.ready.spoken.len()),
            ("True Form Count", forms(forms::TRUE_FORM)),
            ("Ultra Form Count", forms(forms::ULTRA_FORM)),
            ("Talent Count", talents(false)),
            ("Ultra Talent Count", talents(true)),
        ];

        let foes = vec![
            ("New Count", self.ready.foes_new.len()),
            ("Changed Count", self.ready.foes_changed.len()),
            ("Localized Count", self.ready.foes_spoken.len()),
        ];

        let lands = vec![
            ("New Sub Count", subchapters(&self.terrain.opened)),
            ("Localized Sub Count", subchapters(&self.terrain.spoken)),
            ("Changed Sub Count", subchapters(&self.terrain.moved)),
            ("New Crowns Count", subchapters(&self.terrain.crowned)),
            ("New Stage Count", stage_count(&self.terrain.added)),
            ("Changed Stage Count", stage_count(&self.terrain.moved)),
        ];

        let files = vec![
            ("New Data Count", self.files.fresh_data.len()),
            ("Changed Data Count", self.files.moved_data.len()),
            ("New PNG Count", self.files.fresh_art.len()),
            ("Changed PNG Count", self.files.moved_art.len()),
        ];

        vec![
            tally_table("Cats", cats),
            tally_table("Enemies", foes),
            tally_table("Stages", lands),
            tally_table("Files", files),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(label: &str, name: &str, code: u32) -> Build {
        Build { code, name: name.to_string(), label: label.to_string() }
    }

    fn ore(before: Vec<Build>, after: Vec<Build>) -> Diff {
        Diff { schema: 2, stamp: 0, before, after, files: Vec::new(), touched: Vec::new() }
    }

    #[test]
    fn a_column_finds_only_its_own_regions_build() {
        let held = ore(
            Vec::new(),
            vec![build("jp.co.ponos.battlecatsen", "15.5.0", 1_505_000), build("jp.co.ponos.battlecats", "15.6.0", 1_506_000)],
        );

        assert_eq!(build_in(&held, "battlecatsen").map(|b| b.code), Some(1_505_000));
        assert_eq!(build_in(&held, "battlecats").map(|b| b.code), Some(1_506_000));
        assert!(build_in(&held, "battlecatskr").is_none());
    }

    #[test]
    fn the_age_line_reads_as_a_human_would_say_it() {
        assert_eq!(ago(Duration::from_secs(20)), "moments ago");
        assert_eq!(ago(Duration::from_secs(60)), "1 minute ago");
        assert_eq!(ago(Duration::from_secs(7200)), "2 hours ago");
    }
}
