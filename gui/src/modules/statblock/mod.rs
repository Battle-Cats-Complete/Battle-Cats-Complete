pub(crate) mod builder;
mod draw;

use std::time::{Duration, Instant};

use iced::Color;

pub(crate) const FEEDBACK_COOLDOWN: Duration = Duration::from_secs(2);

const DEFAULT_COLOR: Color = Color::from_rgb8(31, 106, 165);
const SUCCESS_COLOR: Color = Color::from_rgb8(40, 160, 60);
const FAIL_COLOR: Color = Color::from_rgb8(200, 40, 40);
const PROCESSING_COLOR: Color = Color::from_rgb8(200, 160, 0);

pub(crate) fn button_feedback(
    busy: bool,
    feedback: Option<(bool, Instant)>,
    idle_label: &str,
    busy_label: &str,
    success_label: &str,
    fail_label: &str,
) -> (String, Color) {
    if busy {
        return (busy_label.to_string(), PROCESSING_COLOR);
    }

    if let Some((success, at)) = feedback
        && at.elapsed() < FEEDBACK_COOLDOWN {
        return if success {
            (success_label.to_string(), SUCCESS_COLOR)
        } else {
            (fail_label.to_string(), FAIL_COLOR)
        };
    }

    (idle_label.to_string(), DEFAULT_COLOR)
}
