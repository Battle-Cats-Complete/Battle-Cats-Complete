use std::path::Path;

use iced::advanced::text::{self, Paragraph as _};
use iced::alignment;
use iced::widget::{button, text as text_widget, Button};
use iced::{Font, Length, Size, Theme};

use crate::app::theme;

pub(crate) const BUTTON_WIDTH: f32 = 160.0;
pub(crate) const BUTTON_HEIGHT: f32 = 30.0;
pub(crate) const TEXT_SIZE: f32 = 13.0;
pub(crate) const GLYPH_ADVANCE: f32 = 0.52;
pub(crate) const COMBO_PADDING: [f32; 2] = [6.5, 8.0];
const COMBO_CLEARANCE: f32 = 25.0;

const INSET: f32 = 10.0;
const ELLIPSIS: char = '\u{2026}';

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

pub(crate) fn slot<'a, M: Clone + 'a>(idle: &'a str, chosen: Option<&Path>, on_press: M) -> Button<'a, M> {
    let filled = chosen.is_some();
    let label = chosen.map_or_else(|| idle.to_string(), |path| fitted(&name_of(path), BUTTON_WIDTH));

    action(label, on_press).style(move |t: &Theme, status| {
        if filled { theme::success_button(t, status) } else { theme::neutral_button(t, status) }
    })
}

pub(crate) fn action<'a, M: Clone + 'a>(label: impl text_widget::IntoFragment<'a>, on_press: M) -> Button<'a, M> {
    button(text_widget(label).size(TEXT_SIZE).width(Length::Fill).height(Length::Fill).center())
        .width(Length::Fixed(BUTTON_WIDTH))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .padding(0)
        .on_press(on_press)
}

pub(crate) fn name_of(path: &Path) -> String {
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

pub(crate) fn combo_width<L: AsRef<str>>(labels: impl Iterator<Item = L>) -> f32 {
    let widest = labels.map(|label| measure(label.as_ref())).fold(0.0, f32::max);

    (widest + COMBO_CLEARANCE).max(BUTTON_WIDTH)
}

pub(crate) fn fitted(name: &str, width: f32) -> String {
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
        let tail: String = name.chars().rev().take(capacity).collect();

        return tail.chars().rev().collect();
    }

    let glyphs: Vec<char> = stem.chars().collect();
    let lead = keep / 2;
    let trail = keep - lead;

    let head: String = glyphs.iter().take(lead).collect();
    let rear: String = glyphs.iter().skip(glyphs.len().saturating_sub(trail)).collect();

    format!("{}{}{}{}", head, ELLIPSIS, rear, suffix)
}

#[cfg(test)]
mod tests {
    use super::{fitted, BUTTON_WIDTH};

    #[test]
    fn a_short_name_is_left_alone() {
        assert_eq!(fitted("205_c.png", BUTTON_WIDTH), "205_c.png");
    }

    #[test]
    fn a_long_name_loses_its_centre_but_keeps_both_ends_and_the_extension() {
        let long = "a_really_very_long_spritesheet_name.imgcut";
        let cut = fitted(long, BUTTON_WIDTH);

        assert!(cut.ends_with("name.imgcut"), "the tail of the stem survives: {cut}");
        assert!(cut.starts_with("a_re"), "so does the head: {cut}");
        assert!(cut.contains('\u{2026}'));
        assert!(cut.chars().count() < long.chars().count());
    }

    #[test]
    fn a_name_with_no_room_for_a_head_is_cut_from_the_start() {
        // Once the budget past the extension is a single glyph, the centre elision
        // degenerates into a start elision on its own.
        let cut = fitted("an_extremely_long_name.mamodel", 90.0);

        assert!(cut.ends_with("e.mamodel"), "the extension stays visible: {cut}");
        assert!(cut.starts_with('\u{2026}'), "the start is what goes: {cut}");
    }
}
