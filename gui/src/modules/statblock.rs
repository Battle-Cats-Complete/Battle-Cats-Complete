pub(crate) mod builder;
mod draw;

use iced::{Color, Theme};
use image::RgbaImage;

/// Result of a background statblock export job.
///
/// Copy only builds the image on the background thread — the clipboard itself must be set from
/// the caller's persistent, main-thread `arboard::Clipboard` (see `builder::copy_to_clipboard`),
/// since dropping a short-lived `Clipboard` hands the X11/Wayland selection ownership back and
/// the copied content becomes unreadable by other applications.
#[derive(Clone)]
pub enum JobResult {
    Copy(Result<RgbaImage, String>),
    Save(Result<(), String>),
}

pub(crate) fn feedback_label(
    busy: bool,
    feedback: Option<bool>,
    idle_label: &str,
    busy_label: &str,
    success_label: &str,
    fail_label: &str,
) -> String {
    if busy {
        return busy_label.to_string();
    }

    if let Some(success) = feedback {
        return if success { success_label.to_string() } else { fail_label.to_string() };
    }

    idle_label.to_string()
}

pub(crate) fn feedback_color(theme: &Theme, busy: bool, feedback: Option<bool>) -> Color {
    let palette = theme.palette();

    if busy {
        return palette.warning;
    }

    if let Some(success) = feedback {
        return if success { palette.success } else { palette.danger };
    }

    palette.primary
}
