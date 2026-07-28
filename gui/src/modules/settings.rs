mod addons;
mod disk;
mod exceptions;
mod keys;
mod pem;

use std::time::Duration;
#[cfg(target_os = "linux")]
use std::time::Instant;

use iced::widget::{
    button, column, container, opaque, pick_list, row, scrollable, stack, text, text_input, toggler,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use tracing::info;

use core::modules::settings::lang;
use core::modules::settings::{
    ExportBehavior, Settings as CoreSettings, SidebarBehavior, UpdateMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Cats,
    Enemies,
    Stages,
    Mods,
    Data,
    Animation,
    AddOns,
    About,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopFeedback {
    Created,
    Deleted,
    Failed,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TabSelected(Tab),

    // General Tab
    ToggleLogging(bool),
    ToggleNightly(bool),
    UpdateModeSelected(UpdateMode),
    LanguageMoveUp(usize),
    LanguageMoveDown(usize),
    RestoreDefaultLanguages,
    #[cfg(target_os = "linux")]
    ToggleDesktopData,
    ManualUpdateCheck,

    // Cats Tab
    PreferredBannerSelected(usize),
    ToggleSmoothBanner(bool),
    ToggleInvalidCats(bool),
    ToggleExpandSpirit(bool),
    DefaultLevelChanged(String),
    ToggleAutoLevel(bool),
    ToggleBumpUltra(bool),

    // Enemies Tab
    ToggleInvalidEnemies(bool),

    // Stages Tab
    SidebarBehaviorSelected(SidebarBehavior),

    // Mods Tab
    ExportBehaviorSelected(ExportBehavior),

    // Data Tab
    ToggleKeyValidation(bool),
    ToggleUltraCompression(bool),
    ManualIpChanged(String),
    ToggleAppPersistence(bool),
    RevealIpField(bool),
    Keys(keys::Message),
    Exceptions(exceptions::Message),
    Disk(disk::Message),

    // Mods Tab (PEM)
    Pem(pem::Message),

    // Addons Tab
    Addons(addons::Message),

    // Animation Tab
    CenteringBehaviorSelected(usize),
    ToggleDebugView(bool),
    ToggleTightBounds(bool),
    ToggleAutoCamera(bool),
    ShowcaseWalkChanged(String),
    ShowcaseIdleChanged(String),
    ShowcaseKbChanged(String),
}

pub struct State {
    pub active_tab: Tab,
    pub ip_field_revealed: bool,
    pub manual_ip_buffer: String,

    pub default_cat_level_buffer: String,
    pub showcase_walk_buffer: String,
    pub showcase_idle_buffer: String,
    pub showcase_kb_buffer: String,

    keys: keys::State,
    exceptions: exceptions::State,
    pem: pem::State,
    addons: addons::State,
    disk: disk::State,

    #[cfg(target_os = "linux")]
    desktop_feedback: Option<(DesktopFeedback, Instant)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: Tab::General,
            ip_field_revealed: false,
            manual_ip_buffer: String::new(),
            default_cat_level_buffer: "1".to_string(),
            showcase_walk_buffer: "0".to_string(),
            showcase_idle_buffer: "0".to_string(),
            showcase_kb_buffer: "0".to_string(),
            keys: keys::State::default(),
            exceptions: exceptions::State::default(),
            pem: pem::State::default(),
            addons: addons::State::default(),
            disk: disk::State::default(),
            #[cfg(target_os = "linux")]
            desktop_feedback: None,
        }
    }
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        // TODO: Rewrite `core` to handle `iced` without ticking
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, core_settings: &mut CoreSettings) -> Task<Message> {
        match message {
            Message::Tick => {
                self.keys.update(keys::Message::Tick);
                self.exceptions.update(exceptions::Message::Tick);
                self.addons.update(addons::Message::Tick);
                self.disk.update(disk::Message::Tick);

                #[cfg(target_os = "linux")]
                if self.desktop_feedback.is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(2)) {
                    self.desktop_feedback = None;
                }

                self.pem.update(pem::Message::Tick).map(Message::Pem)
            }
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                match tab {
                    Tab::General => lang::ensure_complete_list(&mut core_settings.general.language_priority),
                    Tab::Cats => self.default_cat_level_buffer = core_settings.cat_data.default_level.to_string(),
                    Tab::Data => self.manual_ip_buffer = core_settings.game_data.manual_ip.clone(),
                    Tab::Animation => {
                        self.showcase_walk_buffer = core_settings.animation.default_showcase_walk.to_string();
                        self.showcase_idle_buffer = core_settings.animation.default_showcase_idle.to_string();
                        self.showcase_kb_buffer = core_settings.animation.default_showcase_kb.to_string();
                    }
                    _ => {}
                }
                Task::none()
            }

            // General Tab
            Message::ToggleLogging(enabled) => {
                core_settings.general.enable_logging = enabled;
                Task::none()
            }
            Message::ToggleNightly(enabled) => {
                core_settings.general.enable_nightly = enabled;
                Task::none()
            }
            Message::UpdateModeSelected(mode) => {
                core_settings.general.update_mode = mode;
                Task::none()
            }
            Message::LanguageMoveUp(idx) => {
                if idx > 0 && idx < core_settings.general.language_priority.len() {
                    core_settings.general.language_priority.swap(idx, idx - 1);
                }
                Task::none()
            }
            Message::LanguageMoveDown(idx) => {
                if idx < core_settings.general.language_priority.len().saturating_sub(1) {
                    core_settings.general.language_priority.swap(idx, idx + 1);
                }
                Task::none()
            }
            Message::RestoreDefaultLanguages => {
                core_settings.general.language_priority = lang::default_priority();
                Task::none()
            }
            #[cfg(target_os = "linux")]
            Message::ToggleDesktopData => {
                let is_installed = core::modules::settings::desktop::is_desktop_data_present();
                let (feedback, success) = if is_installed {
                    (DesktopFeedback::Deleted, core::modules::settings::desktop::delete_desktop_data().is_ok())
                } else {
                    (DesktopFeedback::Created, core::modules::settings::desktop::create_desktop_data().is_ok())
                };
                self.desktop_feedback = Some((if success { feedback } else { DesktopFeedback::Failed }, Instant::now()));
                Task::none()
            }
            Message::ManualUpdateCheck => {
                info!("Manual update check requested from Settings");
                core_settings.runtime.manual_check_requested = true;
                Task::none()
            }

            // Cats Tab
            Message::PreferredBannerSelected(val) => {
                core_settings.cat_data.preferred_banner_form = val;
                Task::none()
            }
            Message::ToggleSmoothBanner(val) => {
                core_settings.cat_data.high_banner_quality = val;
                Task::none()
            }
            Message::ToggleInvalidCats(val) => {
                core_settings.cat_data.show_invalid_cats = val;
                Task::none()
            }
            Message::ToggleExpandSpirit(val) => {
                core_settings.cat_data.expand_spirit_details = val;
                Task::none()
            }
            Message::DefaultLevelChanged(val) => {
                self.default_cat_level_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.cat_data.default_level = parsed;
                }
                Task::none()
            }
            Message::ToggleAutoLevel(val) => {
                core_settings.cat_data.auto_level_calculations = val;
                Task::none()
            }
            Message::ToggleBumpUltra(val) => {
                core_settings.cat_data.bump_ultra_60 = val;
                Task::none()
            }

            // Enemies Tab
            Message::ToggleInvalidEnemies(val) => {
                core_settings.enemy_data.show_invalid_enemies = val;
                Task::none()
            }

            // Stages Tab
            Message::SidebarBehaviorSelected(val) => {
                core_settings.stages.sidebar_behavior = val;
                Task::none()
            }

            // Mods Tab
            Message::ExportBehaviorSelected(val) => {
                core_settings.mods.export_behavior = val;
                Task::none()
            }
            Message::Pem(msg) => self.pem.update(msg).map(Message::Pem),

            // Data Tab
            Message::ToggleKeyValidation(val) => {
                core_settings.game_data.enforce_key_validation = val;
                Task::none()
            }
            Message::ToggleUltraCompression(val) => {
                core_settings.game_data.enable_ultra_compression = val;
                if !val && core_settings.game_data.last_compression_level > 15 {
                    core_settings.game_data.last_compression_level = 15;
                }
                Task::none()
            }
            Message::ManualIpChanged(val) => {
                self.manual_ip_buffer = val.clone();
                core_settings.game_data.manual_ip = val;
                Task::none()
            }
            Message::ToggleAppPersistence(val) => {
                core_settings.game_data.app_folder_persistence = val;
                Task::none()
            }
            Message::RevealIpField(val) => {
                self.ip_field_revealed = val;
                Task::none()
            }
            Message::Keys(msg) => {
                self.keys.update(msg);
                Task::none()
            }
            Message::Exceptions(msg) => {
                self.exceptions.update(msg);
                Task::none()
            }
            Message::Disk(msg) => {
                self.disk.update(msg);
                Task::none()
            }

            // Addons Tab
            Message::Addons(msg) => {
                self.addons.update(msg);
                Task::none()
            }

            // Animation Tab
            Message::CenteringBehaviorSelected(val) => {
                core_settings.animation.centering_behavior = val;
                Task::none()
            }
            Message::ToggleDebugView(val) => {
                core_settings.animation.debug_view = val;
                Task::none()
            }
            Message::ToggleTightBounds(val) => {
                core_settings.animation.use_tight_bounds = val;
                Task::none()
            }
            Message::ToggleAutoCamera(val) => {
                core_settings.animation.auto_set_camera_region = val;
                Task::none()
            }
            Message::ShowcaseWalkChanged(val) => {
                self.showcase_walk_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_walk = parsed;
                }
                Task::none()
            }
            Message::ShowcaseIdleChanged(val) => {
                self.showcase_idle_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_idle = parsed;
                }
                Task::none()
            }
            Message::ShowcaseKbChanged(val) => {
                self.showcase_kb_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_kb = parsed;
                }
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let main_content = column![
            self.view_tabs(),
            scrollable(container(self.view_tab_content(core_settings)).padding(15))
                .width(Length::Fill)
                .height(Length::Fill),
        ];

        let modal: Option<Element<'a, Message>> = if self.keys.is_open {
            Some(self.keys.view().map(Message::Keys))
        } else if self.exceptions.is_open {
            Some(self.exceptions.view().map(Message::Exceptions))
        } else if self.pem.is_open {
            Some(self.pem.view().map(Message::Pem))
        } else if self.addons.is_modal_open() {
            Some(self.addons.view_modal().map(Message::Addons))
        } else if self.disk.is_modal_open() {
            Some(self.disk.view_modal().map(Message::Disk))
        } else {
            None
        };

        if let Some(modal_content) = modal {
            let overlay = opaque(
                container(modal_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_theme: &Theme| {
                        container::background(
                            iced::Color::from_rgba8(0, 0, 0, 0.6)
                        )
                    })
            );
            stack![main_content, overlay].into()
        } else {
            main_content.into()
        }
    }

    fn view_tabs<'a>(&'a self) -> Element<'a, Message> {
        let tabs = [
            (Tab::General, "General"),
            (Tab::Cats, "Cats"),
            (Tab::Enemies, "Enemies"),
            (Tab::Stages, "Stages"),
            (Tab::Mods, "Mods"),
            (Tab::Data, "Data"),
            (Tab::Animation, "Animation"),
            (Tab::AddOns, "Add-Ons"),
            (Tab::About, "About"),
        ];

        let mut row_tabs = row![].spacing(5).padding(10);

        for (tab_enum, label) in tabs {
            let is_active = self.active_tab == tab_enum;

            let btn = button(text(label).size(14))
                .padding([6, 12])
                .style(move |theme: &Theme, status| {
                    if is_active {
                        button::primary(theme, status)
                    } else {
                        button::secondary(theme, status)
                    }
                })
                .on_press(Message::TabSelected(tab_enum));

            row_tabs = row_tabs.push(btn);
        }

        container(row_tabs).width(Length::Fill).into()
    }

    fn view_tab_content<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        match self.active_tab {
            Tab::General => self.view_general(core_settings),
            Tab::Cats => self.view_cats(core_settings),
            Tab::Enemies => self.view_enemies(core_settings),
            Tab::Stages => self.view_stages(core_settings),
            Tab::Mods => self.view_mods(core_settings),
            Tab::Data => self.view_data(core_settings),
            Tab::Animation => self.view_animation(core_settings),
            Tab::AddOns => self.addons.view().map(Message::Addons),
            Tab::About => self.view_about(),
        }
    }

    fn view_general<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let update_modes = vec!["Auto-Reset", "Auto-Load", "Prompt", "Ignore"];
        let current_update_mode = match core_settings.general.update_mode {
            UpdateMode::AutoReset => "Auto-Reset",
            UpdateMode::AutoLoad => "Auto-Load",
            UpdateMode::Prompt => "Prompt",
            UpdateMode::Ignore => "Ignore",
        };

        let mut lang_col = column![].spacing(2);
        let langs_len = core_settings.general.language_priority.len();
        for (i, lang_code) in core_settings.general.language_priority.iter().enumerate() {
            let row_lang = row![
                text(lang::get_label_for_code(lang_code)).width(Length::Fixed(100.0)),
                button("↑").on_press_maybe(if i > 0 { Some(Message::LanguageMoveUp(i)) } else { None }),
                button("↓").on_press_maybe(if i < langs_len.saturating_sub(1) { Some(Message::LanguageMoveDown(i)) } else { None }),
            ].spacing(5).align_y(Alignment::Center);
            lang_col = lang_col.push(row_lang);
        }

        let mut system_col = column![text("System").size(24)].spacing(10);

        #[cfg(target_os = "linux")]
        {
            let is_installed = core::modules::settings::desktop::is_desktop_data_present();
            let (label, color) = match &self.desktop_feedback {
                Some((DesktopFeedback::Created, _)) => ("Desktop Data Created!", [40, 160, 40]),
                Some((DesktopFeedback::Deleted, _)) => ("Desktop Data Deleted!", [40, 160, 40]),
                Some((DesktopFeedback::Failed, _)) => ("Failed!", [180, 50, 50]),
                None if is_installed => ("Delete Desktop Data", [180, 50, 50]),
                None => ("Create Desktop Data", [40, 90, 160]),
            };
            system_col = system_col.push(
                button(text(label))
                    .padding([8, 16])
                    .style(move |_theme: &Theme, _status| button::Style {
                        background: Some(iced::Color::from_rgb8(color[0], color[1], color[2]).into()),
                        text_color: iced::Color::WHITE,
                        ..Default::default()
                    })
                    .on_press(Message::ToggleDesktopData)
            );
        }

        system_col = system_col.push(button("Check for Update Now").on_press(Message::ManualUpdateCheck));

        column![
            system_col,

            text("Behavior").size(24),
            row![
                toggler(core_settings.general.enable_logging).on_toggle(Message::ToggleLogging),
                text("Enable Logging"),
            ].spacing(10),
            row![
                toggler(core_settings.general.enable_nightly).on_toggle(Message::ToggleNightly),
                text("Enable Nightly Features 🌙"),
            ].spacing(10),
            row![
                text("Update Handling:"),
                pick_list(
                    update_modes,
                    Some(current_update_mode),
                    |val| {
                        let mode = match val {
                            "Auto-Reset" => UpdateMode::AutoReset,
                            "Auto-Load" => UpdateMode::AutoLoad,
                            "Prompt" => UpdateMode::Prompt,
                            _ => UpdateMode::Ignore,
                        };
                        Message::UpdateModeSelected(mode)
                    }
                ),
            ].spacing(10).align_y(Alignment::Center),

            text("Language Priority").size(24),
            lang_col,
            button("Restore Defaults").on_press(Message::RestoreDefaultLanguages),
        ].spacing(20).into()
    }

    fn view_cats<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let banner_options: Vec<usize> = vec![0, 1, 2, 3];

        column![
            text("Cat List").size(24),
            row![
                text("Preferred Banner Form:"),
                pick_list(
                    banner_options,
                    Some(core_settings.cat_data.preferred_banner_form),
                    Message::PreferredBannerSelected,
                ),
            ].spacing(10).align_y(Alignment::Center),

            row![
                toggler(core_settings.cat_data.high_banner_quality).on_toggle(Message::ToggleSmoothBanner),
                text("Smooth Banner Scaling"),
            ].spacing(10),

            row![
                toggler(core_settings.cat_data.show_invalid_cats).on_toggle(Message::ToggleInvalidCats),
                text("Show Invalid Cats"),
            ].spacing(10),

            text("Ability Display").size(24),
            row![
                toggler(core_settings.cat_data.expand_spirit_details).on_toggle(Message::ToggleExpandSpirit),
                text("Expand Spirit Details by Default"),
            ].spacing(10),

            text("Level Display").size(24),
            row![
                text("Default Level:"),
                text_input("Level", &self.default_cat_level_buffer)
                    .on_input(Message::DefaultLevelChanged)
                    .width(Length::Fixed(60.0)),
            ].spacing(10).align_y(Alignment::Center),

            row![
                toggler(core_settings.cat_data.auto_level_calculations).on_toggle(Message::ToggleAutoLevel),
                text("Auto Level Calculations"),
            ].spacing(10),

            row![
                toggler(core_settings.cat_data.bump_ultra_60).on_toggle(Message::ToggleBumpUltra),
                text("Lv60 For Ultra"),
            ].spacing(10),
        ].spacing(20).into()
    }

    fn view_enemies<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            text("Enemy List").size(24),
            row![
                toggler(core_settings.enemy_data.show_invalid_enemies).on_toggle(Message::ToggleInvalidEnemies),
                text("Show Invalid Enemies"),
            ].spacing(10),
        ].spacing(20).into()
    }

    fn view_stages<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let sidebar_options = vec!["Cover", "Push"];
        let current_sidebar = match core_settings.stages.sidebar_behavior {
            SidebarBehavior::Cover => "Cover",
            SidebarBehavior::Push => "Push",
        };

        column![
            text("Stage List").size(24),
            row![
                text("Sidebar Behavior:"),
                pick_list(
                    sidebar_options,
                    Some(current_sidebar),
                    |val| {
                        let behavior = match val {
                            "Cover" => SidebarBehavior::Cover,
                            _ => SidebarBehavior::Push,
                        };
                        Message::SidebarBehaviorSelected(behavior)
                    }
                ),
            ].spacing(10).align_y(Alignment::Center),
        ].spacing(20).into()
    }

    fn view_mods<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let export_options = vec!["Automatic", "Create", "Update"];
        let current_export = match core_settings.mods.export_behavior {
            ExportBehavior::Automatic => "Automatic",
            ExportBehavior::Create => "Create",
            ExportBehavior::Update => "Update",
        };

        column![
            text("Export").size(24),
            button("Manage PEM").on_press(Message::Pem(pem::Message::Open)),
            row![
                text("Export Behavior:"),
                pick_list(
                    export_options,
                    Some(current_export),
                    |val| {
                        let behavior = match val {
                            "Automatic" => ExportBehavior::Automatic,
                            "Create" => ExportBehavior::Create,
                            _ => ExportBehavior::Update,
                        };
                        Message::ExportBehaviorSelected(behavior)
                    }
                ),
            ].spacing(10).align_y(Alignment::Center),
        ].spacing(20).into()
    }

    fn view_data<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            text("Disk").size(24),
            self.disk.view().map(Message::Disk),

            text("Management").size(24),
            row![
                button("Manage Keys").on_press(Message::Keys(keys::Message::Open)),
                button("Manage Exceptions").on_press(Message::Exceptions(exceptions::Message::Open)),
            ].spacing(10),

            row![
                toggler(core_settings.game_data.enforce_key_validation).on_toggle(Message::ToggleKeyValidation),
                text("Enforce Key Validation"),
            ].spacing(10),
            row![
                toggler(core_settings.game_data.enable_ultra_compression).on_toggle(Message::ToggleUltraCompression),
                text("Enable Ultra Compression"),
            ].spacing(10),

            text("Android").size(24),
            row![
                text("Fallback IP Address:"),
                if self.ip_field_revealed {
                    Element::from(text_input("192.168.X.X", &self.manual_ip_buffer).on_input(Message::ManualIpChanged).width(Length::Fixed(120.0)))
                } else {
                    Element::from(button("Click to Reveal").on_press(Message::RevealIpField(true)))
                },
                button("👁").on_press(Message::RevealIpField(!self.ip_field_revealed)),
            ].spacing(10).align_y(Alignment::Center),
            row![
                toggler(core_settings.game_data.app_folder_persistence).on_toggle(Message::ToggleAppPersistence),
                text("App Folder Persistence"),
            ].spacing(10),
        ].spacing(20).into()
    }

    fn view_animation<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let centering_options: Vec<usize> = vec![0, 1, 2];

        column![
            text("Viewer").size(24),
            row![
                text("Centering Behavior:"),
                pick_list(
                    centering_options,
                    Some(core_settings.animation.centering_behavior),
                    Message::CenteringBehaviorSelected,
                ),
            ].spacing(10).align_y(Alignment::Center),
            row![
                toggler(core_settings.animation.debug_view).on_toggle(Message::ToggleDebugView),
                text("Enable Debug View"),
            ].spacing(10),

            text("Exporter").size(24),
            row![
                toggler(core_settings.animation.use_tight_bounds).on_toggle(Message::ToggleTightBounds),
                text("Use Tight Bounds"),
            ].spacing(10),
            row![
                toggler(core_settings.animation.auto_set_camera_region).on_toggle(Message::ToggleAutoCamera),
                text("Auto-Set Camera Region"),
            ].spacing(10),

            text("Showcase").size(18),
            row![text("Walk Frames:"), text_input("0", &self.showcase_walk_buffer).on_input(Message::ShowcaseWalkChanged).width(Length::Fixed(60.0))].spacing(10),
            row![text("Idle Frames:"), text_input("0", &self.showcase_idle_buffer).on_input(Message::ShowcaseIdleChanged).width(Length::Fixed(60.0))].spacing(10),
            row![text("KB Frames:"), text_input("0", &self.showcase_kb_buffer).on_input(Message::ShowcaseKbChanged).width(Length::Fixed(60.0))].spacing(10),
        ].spacing(20).into()
    }

    fn view_about<'a>(&'a self) -> Element<'a, Message> {
        let license_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../core/assets/licenses.txt"));

        column![
            text("About Battle Cats Complete").size(32),
            text("A high-performance Battle Cats toolkit built by Omochi"),
            text("Open Source & Legal Info").size(20),
            scrollable(text(license_text).size(11))
                .width(Length::Fill)
                .height(Length::Fill)
        ].spacing(15).into()
    }
}
