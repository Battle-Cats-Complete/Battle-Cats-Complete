use super::*;
use iced::widget::{column, container, row, rule, text, text_input};

use crate::widget::picker;

pub(super) const TITLE: &str = "Onionskin";
pub(super) const SPEC: popup::Spec = popup::Spec::new(popup::Kind::StudioOnion, Size::new(316.0, 248.0));

const PADDING: f32 = 12.0;
const STEP: f32 = 6.0;
const LABEL: f32 = 13.0;
const NAME_WIDTH: f32 = 54.0;
const HASH_WIDTH: f32 = 9.0;
const HINT_COUNT: &str = "None";
const HINT_COLOR: &str = "RRGGBB";
const HINT_FRAMES: &str = "Frames";
const HINT_PERCENT: &str = "Percent";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Knob {
    BeforeCount,
    BeforeColor,
    AfterCount,
    AfterColor,
    Delay,
    Duration,
    Opacity,
}

impl Knob {
    pub(super) fn digits(self) -> bool {
        !matches!(self, Knob::BeforeColor | Knob::AfterColor)
    }

    pub(super) fn held(self, anim: &StudioSettings) -> &String {
        match self {
            Knob::BeforeCount => &anim.onion_before,
            Knob::BeforeColor => &anim.onion_before_color,
            Knob::AfterCount => &anim.onion_after,
            Knob::AfterColor => &anim.onion_after_color,
            Knob::Delay => &anim.onion_gap,
            Knob::Duration => &anim.onion_life,
            Knob::Opacity => &anim.onion_alpha,
        }
    }

    pub(super) fn set(self, anim: &mut StudioSettings, typed: String) {
        let slot = match self {
            Knob::BeforeCount => &mut anim.onion_before,
            Knob::BeforeColor => &mut anim.onion_before_color,
            Knob::AfterCount => &mut anim.onion_after,
            Knob::AfterColor => &mut anim.onion_after_color,
            Knob::Delay => &mut anim.onion_gap,
            Knob::Duration => &mut anim.onion_life,
            Knob::Opacity => &mut anim.onion_alpha,
        };

        *slot = typed;
    }
}

fn field<'a>(knob: Knob, hint: &'a str, anim: &'a StudioSettings) -> Element<'a, Message> {
    text_input(hint, knob.held(anim))
        .size(LABEL)
        .padding(picker::COMBO_PADDING)
        .width(Length::Fill)
        .on_input(move |typed| Message::Onioned(knob, typed))
        .style(theme::rounded_input)
        .into()
}

fn seat<'a>(name: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    row![text(name).size(LABEL).width(Length::Fixed(NAME_WIDTH)), control]
        .spacing(STEP)
        .align_y(Vertical::Center)
        .into()
}

fn stacked<'a>(name: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![theme::centered_text(name).size(LABEL).width(Length::Fill), control]
        .spacing(3)
        .width(Length::Fill)
        .into()
}

fn heading<'a>(name: &'a str) -> Element<'a, Message> {
    column![theme::centered_text(name).size(LABEL).width(Length::Fill), rule::horizontal(1)]
        .spacing(2)
        .into()
}

fn side<'a>(name: &'a str, anim: &'a StudioSettings, count: Knob, color: Knob) -> Element<'a, Message> {
    let hashed = row![
        text("#").size(LABEL).width(Length::Fixed(HASH_WIDTH)),
        field(color, HINT_COLOR, anim),
    ]
    .align_y(Vertical::Center);

    column![
        heading(name),
        seat("Count", field(count, HINT_COUNT, anim)),
        seat("Color", hashed.into()),
    ]
    .spacing(STEP)
    .width(Length::Fill)
    .into()
}

pub(super) fn view(anim: &StudioSettings) -> Element<'_, Message> {
    let sides = row![
        side("Before", anim, Knob::BeforeCount, Knob::BeforeColor),
        side("After", anim, Knob::AfterCount, Knob::AfterColor),
    ]
    .spacing(PADDING);

    let shared = row![
        stacked("Delay", field(Knob::Delay, HINT_FRAMES, anim)),
        stacked("Duration", field(Knob::Duration, HINT_FRAMES, anim)),
        stacked("Opacity", field(Knob::Opacity, HINT_PERCENT, anim)),
    ]
    .spacing(STEP);

    let body = column![sides, heading("Ghost"), shared].spacing(STEP + 2.0);

    container(body).padding(PADDING).width(Length::Fill).into()
}
