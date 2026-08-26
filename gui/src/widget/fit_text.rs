use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{Paragraph as _, Text};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{column, responsive, text};
use iced::{Element, Font, Length, Pixels, Size};

const STEP: f32 = 1.0;

pub(crate) fn fit_column<'a, Message: 'a>(
    lines: Vec<(String, f32)>,
    spacing: f32,
    floor: f32,
) -> Element<'a, Message> {
    responsive(move |available| {
        let shrink = shrink_by(&lines, available.width, floor);

        let mut stacked = column![]
            .spacing(spacing)
            .width(Length::Fill)
            .align_x(Horizontal::Center);

        for (content, size) in &lines {
            stacked = stacked.push(text(content.clone()).size((size - shrink).max(floor)));
        }

        stacked.into()
    })
        .height(Length::Shrink)
        .into()
}

fn shrink_by(lines: &[(String, f32)], available: f32, floor: f32) -> f32 {
    let deepest = lines.iter().map(|(_, size)| size - floor).fold(0.0, f32::max);

    if available <= 0.0 || deepest <= 0.0 {
        return 0.0;
    }

    let mut shrink: f32 = 0.0;

    for (content, size) in lines {
        let width = measure(content, *size);

        if width <= available {
            continue;
        }

        shrink = shrink.max(size - size * available / width);
    }

    shrink = shrink.ceil().min(deepest);

    while shrink < deepest && overflows(lines, shrink, available, floor) {
        shrink += STEP;
    }

    shrink.min(deepest)
}

fn overflows(lines: &[(String, f32)], shrink: f32, available: f32, floor: f32) -> bool {
    lines.iter().any(|(content, size)| measure(content, (size - shrink).max(floor)) > available)
}

fn measure(content: &str, size: f32) -> f32 {
    Paragraph::with_text(Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height: LineHeight::default(),
        font: Font::DEFAULT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: Vertical::Top,
        shaping: Shaping::default(),
        wrapping: Wrapping::default(),
    })
        .min_bounds()
        .width
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: &str = "Attack Power of All Units Up Considerably While Only Nyanko Rangers";

    #[test]
    fn a_line_that_already_fits_is_left_alone() {
        let lines = vec![("Cat Army".to_string(), 18.0), ("Worker Start Level Up (Sm)".to_string(), 14.0)];

        assert_eq!(shrink_by(&lines, 4000.0, 8.0), 0.0);
    }

    #[test]
    fn every_line_shrinks_together_until_the_widest_one_fits() {
        let lines = vec![(LONG.to_string(), 18.0), ("Cat Army".to_string(), 14.0)];
        let available = 400.0;

        let shrink = shrink_by(&lines, available, 8.0);

        assert!(shrink > 0.0, "an overflowing line must pull the whole block down a size");
        assert!(!overflows(&lines, shrink, available, 8.0), "the chosen size still overflows");
        assert!(
            overflows(&lines, shrink - STEP, available, 8.0),
            "the block shrank further than it had to"
        );
    }

    #[test]
    fn the_floor_bounds_how_far_a_block_will_shrink() {
        let lines = vec![(LONG.to_string(), 18.0)];

        assert_eq!(shrink_by(&lines, 1.0, 8.0), 10.0);
    }
}
