use iced::widget::{container, operation, scrollable};
use iced::{widget, Element, Font, Length, Task};

use crate::app::theme;
use crate::widget::smooth_scroll;

const STICK_THRESHOLD: f32 = 0.995;
const SCROLLBAR_GAP: f32 = 2.0;

pub(crate) struct ConsoleState {
    id: widget::Id,
    stuck_to_bottom: bool,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self { id: widget::Id::unique(), stuck_to_bottom: true }
    }
}

impl ConsoleState {
    pub(crate) fn on_scroll(&mut self, viewport: scrollable::Viewport) {
        let overflow = viewport.content_bounds().height - viewport.bounds().height;

        self.on_scroll_offset(viewport.relative_offset().y, overflow);
    }

    fn on_scroll_offset(&mut self, offset: f32, overflow: f32) {
        self.stuck_to_bottom = sticks(offset, overflow);
    }

    pub(crate) fn restick<Message: 'static>(&mut self) -> Task<Message> {
        self.stuck_to_bottom = true;

        operation::snap_to_end(self.id.clone())
    }

    pub(crate) fn snap_to_bottom<Message: 'static>(&self) -> Task<Message> {
        if self.stuck_to_bottom {
            operation::snap_to_end(self.id.clone())
        } else {
            Task::none()
        }
    }

    pub(crate) fn view<'a, Message: 'a>(
        &self,
        log: &'a str,
        on_scroll: impl Fn(scrollable::Viewport) -> Message + 'a,
    ) -> Element<'a, Message> {
        mock_console(self.id.clone(), log, on_scroll)
    }
}

fn sticks(offset: f32, overflow: f32) -> bool {
    overflow <= 0.0 || offset >= STICK_THRESHOLD
}

pub(crate) fn mock_console<'a, Message: 'a>(
    id: widget::Id,
    log: &'a str,
    on_scroll: impl Fn(scrollable::Viewport) -> Message + 'a,
) -> Element<'a, Message> {
    let content = scrollable(
        container(iced::widget::text(log.trim_end()).size(12).font(Font::MONOSPACE))
            .width(Length::Fill)
            .padding(8),
    )
        .id(id)
        .on_scroll(on_scroll)
        .spacing(SCROLLBAR_GAP)
        .height(Length::Fill);

    container(container(smooth_scroll(content)).padding(theme::CONSOLE_BORDER_WIDTH))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::mock_console_container)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A cleared console is shorter than its viewport, so iced reports 0 / -overflow.
    // That is -0.0, which is not >= the threshold, and it used to unstick the console
    // for the rest of the session.
    #[test]
    fn a_console_with_nothing_to_scroll_stays_stuck() {
        assert!(sticks(0.0 / -200.0, -200.0), "a cleared console must stay stuck");
        assert!(sticks(f32::NAN, 0.0), "content exactly filling the viewport must stay stuck");
        assert!(sticks(0.0, 0.0));
    }

    // Navigating away drops the scrollable; the rebuilt one reports offset 0, which
    // unsticks the console for the rest of the session even though nobody scrolled.
    #[test]
    fn re_entering_the_page_re_arms_a_console_the_remount_unstuck() {
        let mut console = ConsoleState::default();

        console.on_scroll_offset(0.0, 400.0);
        assert!(!console.stuck_to_bottom);

        let _task: Task<()> = console.restick();
        assert!(console.stuck_to_bottom);
    }

    #[test]
    fn scrolling_away_from_the_bottom_still_unsticks() {
        assert!(!sticks(0.0, 400.0));
        assert!(!sticks(0.9, 400.0));
        assert!(sticks(1.0, 400.0));
        assert!(sticks(STICK_THRESHOLD, 400.0));
    }
}
