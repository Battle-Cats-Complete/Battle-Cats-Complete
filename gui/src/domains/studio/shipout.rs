use super::*;
use iced::widget::{button, column, container, row, text, text_input};

use crate::widget::picker;

use kore::domains::cat::scanner::CatEntry;
use kore::domains::enemy::scanner::EnemyEntry;

pub(super) const TITLE: &str = "Export Set";
pub(super) const SPEC: popup::Spec = popup::Spec::new(popup::Kind::StudioShipout, Size::new(304.0, 186.0));

const PADDING: f32 = 12.0;
const STEP: f32 = 9.0;
const LABEL: f32 = 13.0;
const SEAT_WIDTH: f32 = 216.0;
const HINT: &str = "File or Entity...";
const JOINT: &str = "::";

pub(crate) struct Muster<'a> {
    pub(crate) cats: &'a [CatEntry],
    pub(crate) enemies: &'a [EnemyEntry],
}

impl sets::Roster for Muster<'_> {
    fn cat(&self, id: u32, form: usize) -> Option<String> {
        let held = self.cats.iter().find(|cat| cat.id == id)?;

        held.names.get(form)?.clone().filter(|name| !name.trim().is_empty())
    }

    fn cat_named(&self, name: &str) -> Option<(u32, usize)> {
        self.cats.iter().find_map(|cat| {
            let form = cat.names.iter().position(|held| held.as_deref() == Some(name))?;

            Some((cat.id, form))
        })
    }

    fn enemy(&self, id: u32) -> Option<String> {
        self.enemies
            .iter()
            .find(|enemy| enemy.id == id)
            .map(|enemy| enemy.name.clone())
            .filter(|name| !name.trim().is_empty())
    }

    fn enemy_named(&self, name: &str) -> Option<u32> {
        self.enemies.iter().find(|enemy| enemy.name == name).map(|enemy| enemy.id)
    }
}

fn verb(aim: &sets::Aim) -> &'static str {
    match aim {
        sets::Aim::Cat { .. } => "Export to Cat",
        sets::Aim::Enemy { .. } => "Export to Enemy",
        sets::Aim::Blank | sets::Aim::Zip(_) => "Export to ZIP",
    }
}

fn landing<'a>(set: &'a str, aim: &'a sets::Aim) -> Element<'a, Message> {
    let faded = |held: String| {
        text(held)
            .size(LABEL)
            .style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) })
    };

    let Some((name, id)) = aim.parted() else {
        return faded(aim.caption(set)).into();
    };

    row![
        faded(name),
        text(JOINT).size(LABEL).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        faded(id),
    ]
    .spacing(5)
    .align_y(Vertical::Center)
    .into()
}

pub(super) fn view<'a>(
    set: &'a str,
    typed: &'a str,
    aim: &'a sets::Aim,
    armed: bool,
    shipping: Shipping,
) -> Element<'a, Message> {
    let asking = text(format!("Export \"{}\" to?", set)).size(LABEL);

    let field = text_input(HINT, typed)
        .size(LABEL)
        .padding(picker::COMBO_PADDING)
        .width(Length::Fixed(SEAT_WIDTH))
        .on_input(Message::Aimed)
        .on_submit(Message::Ship)
        .style(theme::rounded_input);

    let (label, style, press) = shipping.shipout(verb(aim), armed);

    let go = button(theme::centered_text(label).size(LABEL).width(Length::Fill))
        .width(Length::Fixed(SEAT_WIDTH))
        .padding([3, 6])
        .on_press_maybe(press)
        .style(style);

    let body = column![asking, field, landing(set, aim), go]
        .spacing(STEP)
        .align_x(Horizontal::Center);

    container(body).padding(PADDING).width(Length::Fill).center_x(Length::Fill).into()
}
