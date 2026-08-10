use std::ops::Range;

pub(crate) const ROW_HEIGHT: f32 = 50.0;
pub(crate) const ROW_SPACING: f32 = 4.0;
pub(crate) const ROW_PITCH: f32 = ROW_HEIGHT + ROW_SPACING;

const BUFFER_ROWS: usize = 4;

pub(crate) struct RowWindow {
    pub(crate) range: Range<usize>,
    pub(crate) pad_before: f32,
    pub(crate) pad_after: f32,
}

pub(crate) fn compute(total: usize, viewport_h: f32, offset: f32) -> RowWindow {
    if total == 0 {
        return RowWindow { range: 0..0, pad_before: 0.0, pad_after: 0.0 };
    }

    let content_h = ROW_PITCH * total as f32 - ROW_SPACING;
    let max_offset = (content_h - viewport_h).max(0.0);
    let offset = offset.clamp(0.0, max_offset).round();

    let anchor = (offset / ROW_PITCH).floor() as usize;
    let span = ((viewport_h / ROW_PITCH).ceil() as usize).saturating_add(1);

    let first = anchor.saturating_sub(BUFFER_ROWS);
    let end = anchor.saturating_add(span).saturating_add(BUFFER_ROWS).min(total);

    RowWindow {
        range: first..end,
        pad_before: if first > 0 { ROW_PITCH * first as f32 - ROW_SPACING } else { 0.0 },
        pad_after: if end < total { ROW_PITCH * (total - end) as f32 - ROW_SPACING } else { 0.0 },
    }
}

