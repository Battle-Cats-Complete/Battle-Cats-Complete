use std::path::Path;

use iced::advanced::text::{self, Paragraph as _};
use iced::alignment;
use iced::widget::{button, text as text_widget, Button};
use iced::{Font, Length, Size, Theme};

use crate::app::theme;

pub(super) const BUTTON_WIDTH: f32 = 160.0;
pub(super) const BUTTON_HEIGHT: f32 = 30.0;
pub(super) const TEXT_SIZE: f32 = 13.0;
pub(super) const GLYPH_ADVANCE: f32 = 0.52;
pub(super) const COMBO_PADDING: [f32; 2] = [6.5, 8.0];
const COMBO_CLEARANCE: f32 = 25.0;

const INSET: f32 = 10.0;
const ELLIPSIS: char = '\u{2026}';

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

pub(super) fn slot<'a, M: Clone + 'a>(idle: &'a str, chosen: Option<&Path>, on_press: M) -> Button<'a, M> {
    let filled = chosen.is_some();
    let label = chosen.map_or_else(|| idle.to_string(), |path| fitted(&name_of(path), BUTTON_WIDTH));

    action(label, on_press).style(move |t: &Theme, status| {
        if filled { theme::success_button(t, status) } else { theme::neutral_button(t, status) }
    })
}

pub(super) fn action<'a, M: Clone + 'a>(label: impl text_widget::IntoFragment<'a>, on_press: M) -> Button<'a, M> {
    button(text_widget(label).size(TEXT_SIZE).width(Length::Fill).height(Length::Fill).center())
        .width(Length::Fixed(BUTTON_WIDTH))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .padding(0)
        .on_press(on_press)
}

pub(super) fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().to_string())
}

fn measure(label: &str) -> f32 {
    let line_height = text::LineHeight::default();

    Paragraph::with_text(text::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, line_height.to_absolute(TEXT_SIZE.into()).into()),
        size: TEXT_SIZE.into(),
        line_height,
        font: Font::DEFAULT,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Center,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::default(),
    })
    .min_width()
}

pub(super) fn combo_width<L: AsRef<str>>(labels: impl Iterator<Item = L>) -> f32 {
    let widest = labels.map(|label| measure(label.as_ref())).fold(0.0, f32::max);

    (widest + COMBO_CLEARANCE).max(BUTTON_WIDTH)
}

pub(super) fn fitted(name: &str, width: f32) -> String {
    let capacity = ((width - INSET * 2.0) / (TEXT_SIZE * GLYPH_ADVANCE)).floor().max(1.0) as usize;

    if name.chars().count() <= capacity {
        return name.to_string();
    }

    let (stem, suffix) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{}", extension)),
        None => (name, String::new()),
    };

    let reserved = suffix.chars().count() + 1;
    let keep = capacity.saturating_sub(reserved);

    if keep == 0 {
        return name.chars().take(capacity).collect();
    }

    let head: String = stem.chars().take(keep).collect();

    format!("{}{}{}", head, ELLIPSIS, suffix)
}

#[cfg(test)]
mod tests {
    use super::{fitted, BUTTON_WIDTH};

    #[test]
    fn a_short_name_is_left_alone() {
        assert_eq!(fitted("205_c.png", BUTTON_WIDTH), "205_c.png");
    }

    #[test]
    fn a_long_name_loses_stem_but_keeps_its_extension() {
        let long = "a_really_very_long_spritesheet_name.imgcut";
        let cut = fitted(long, BUTTON_WIDTH);

        assert!(cut.ends_with(".imgcut"));
        assert!(cut.contains('\u{2026}'));
        assert!(cut.chars().count() < long.chars().count());
    }
}
