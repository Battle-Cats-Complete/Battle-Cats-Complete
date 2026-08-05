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
        self.stuck_to_bottom = viewport.relative_offset().y >= STICK_THRESHOLD;
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
