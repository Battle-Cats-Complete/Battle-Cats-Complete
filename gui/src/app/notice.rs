use iced::widget::{button, column, markdown, scrollable, text, Space};
use iced::{Alignment, Element, Length, Size, Theme};
use sha2::{Digest, Sha256};

use crate::widget::{popup, smooth_scroll};

use super::Message;

const POPUP_SIZE: Size = Size::new(500.0, 400.0);
const HASH_BYTES: usize = 8;
const SCROLLBAR_GAP: f32 = 8.0;

pub(super) const NOTICE_TITLE: &str = "TEST3";
pub(super) const NOTICE_CONTENT: &str = r#"
This is a test pop-up, if you are seeing this, tell @omochikeari15 she accidentally shipped a test in an stable release.

Markdown Test:
# Hashtag1
## Hastag2
### Hashtag3
`Tilde1`
```
Tilde3
```
*Asterisk1*
**Asterisk2**
***Asterisk3***
_Underscore1_
__Underscore2__
~Squiggly1~
~~Squiggly2~~
"#;

pub(super) fn parse_content() -> Vec<markdown::Item> {
    markdown::parse(NOTICE_CONTENT).collect()
}

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

pub(super) fn view<'a>(state: &'a popup::State, window: Size, theme: Theme, items: &'a [markdown::Item]) -> Element<'a, Message> {
    state.view(NOTICE_TITLE, POPUP_SIZE, window, Message::Notice, move || {
        column![
            smooth_scroll(
                scrollable(
                    markdown::view(items, markdown::Settings::with_text_size(14.0, &theme))
                        .map(Message::OpenUrl)
                )
                    .height(Length::Fill)
                    .spacing(SCROLLBAR_GAP)
            ),
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

