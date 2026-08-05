use iced::widget::{button, column, scrollable, text, Space};
use iced::{Alignment, Element, Length, Size};
use sha2::{Digest, Sha256};

use crate::widget::popup;

use super::Message;

const POPUP_SIZE: Size = Size::new(500.0, 400.0);
const HASH_BYTES: usize = 8;

pub(super) const NOTICE_TITLE: &str = "TEST2";
pub(super) const NOTICE_CONTENT: &str = r#"
This is a test pop-up, if you are seeing this, tell @omochikeari15 she accidentally shipped a test in an stable release.
"#;

fn digest(title: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update([0u8]);
    hasher.update(content.as_bytes());

    hasher
        .finalize()
        .iter()
        .take(HASH_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn hash() -> String {
    digest(NOTICE_TITLE, NOTICE_CONTENT)
}

pub(super) fn should_show(acknowledged: &[String]) -> bool {
    !NOTICE_CONTENT.trim().is_empty() && !acknowledged.contains(&hash())
}

pub(super) fn update(state: &mut popup::State, message: popup::Message) -> bool {
    state.update(message, POPUP_SIZE)
}

pub(super) fn view(state: &popup::State, window: Size) -> Element<'_, Message> {
    state.view(NOTICE_TITLE, POPUP_SIZE, window, Message::Notice, || {
        column![
            scrollable(text(NOTICE_CONTENT).size(14.0)).height(Length::Fill),
            Space::new().height(20.0),
            button(text("Acknowledge").size(16.0))
                .style(button::primary)
                .on_press(Message::AcknowledgeNotice),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20.0)
        .into()
    }, None)
}

