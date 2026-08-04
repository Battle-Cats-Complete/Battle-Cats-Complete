pub mod app;
pub mod common;
pub mod modules;
pub mod widget;

use std::fs;
use std::panic;

use iced::window;
use iced::Size;

use core::common::assets;
use core::common::dirs::APP_DIR;

pub fn main() -> iced::Result {
    panic::set_hook(Box::new(|panic_info| {
        let msg = format!("Battle Cats Complete crashed!\n{}\n", panic_info);
        let _ = fs::write("crash.txt", msg);
    }));

    iced::application(
        app::BattleCatsApp::new,
        app::BattleCatsApp::update,
        app::BattleCatsApp::view,
    )
        .title("Battle Cats Complete")
        .theme(app::BattleCatsApp::theme)
        .font(assets::FONT_JP)
        .font(assets::FONT_KR)
        .font(assets::FONT_TC)
        .font(assets::FONT_TH)
        .window(window::Settings {
            size: Size::new(800.0, 600.0),
            min_size: Some(Size::new(800.0, 600.0)),
            icon: load_icon(),
            platform_specific: platform_specific_settings(),
            ..Default::default()
        })
        .subscription(app::BattleCatsApp::subscription)
        .run()
}

#[cfg(target_os = "linux")]
fn platform_specific_settings() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific { application_id: APP_DIR.to_string(), ..Default::default() }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific_settings() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific::default()
}

fn load_icon() -> Option<window::icon::Icon> {
    image::load_from_memory(assets::ICON).ok().and_then(|image| {
        let rgba = image.into_rgba8();
        let (width, height) = rgba.dimensions();
        let raw_pixels = rgba.into_raw();
        window::icon::from_rgba(raw_pixels, width, height).ok()
    })
}