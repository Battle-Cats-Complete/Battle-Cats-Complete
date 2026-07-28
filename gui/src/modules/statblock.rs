pub(crate) mod builder;
mod draw;

use std::time::{Duration, Instant};

use iced::{Color, Theme};
use image::RgbaImage;

pub(crate) const FEEDBACK_COOLDOWN: Duration = Duration::from_secs(2);

/// Result of a background statblock export job.
///
/// Copy only builds the image on the background thread — the clipboard itself must be set from
/// the caller's persistent, main-thread `arboard::Clipboard` (see `builder::copy_to_clipboard`),
/// since dropping a short-lived `Clipboard` hands the X11/Wayland selection ownership back and
/// the copied content becomes unreadable by other applications.
pub(crate) enum JobResult {
    Copy(Result<RgbaImage, String>),
    Save(Result<(), String>),
}

pub(crate) fn feedback_label(
    busy: bool,
    feedback: Option<(bool, Instant)>,
    idle_label: &str,
    busy_label: &str,
    success_label: &str,
    fail_label: &str,
) -> String {
    if busy {
        return busy_label.to_string();
    }

    if let Some((success, at)) = feedback
        && at.elapsed() < FEEDBACK_COOLDOWN {
        return if success { success_label.to_string() } else { fail_label.to_string() };
    }

    idle_label.to_string()
}

pub(crate) fn feedback_color(theme: &Theme, busy: bool, feedback: Option<(bool, Instant)>) -> Color {
    let palette = theme.palette();

    if busy {
        return palette.warning;
    }

    if let Some((success, at)) = feedback
        && at.elapsed() < FEEDBACK_COOLDOWN {
        return if success { palette.success } else { palette.danger };
    }

    palette.primary
}
