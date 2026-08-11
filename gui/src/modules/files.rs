use iced::widget::{container, Space};
use iced::{Element, Length};

use core::modules::settings::nightly;

pub(crate) fn register_nightly() {
    nightly::register_nightly_usage();
}

pub(crate) fn view<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
