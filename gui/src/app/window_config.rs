use iced::Size;

use core::common::io::json;
use core::modules::settings::WindowSettings;

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct WindowConfig {
    settings: SettingsWindowField,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct SettingsWindowField {
    window: WindowSettings,
}

pub fn saved_size() -> Size {
    let config: WindowConfig = json::load("settings.json").unwrap_or_default();
    Size::new(config.settings.window.width.max(800.0), config.settings.window.height.max(600.0))
}
