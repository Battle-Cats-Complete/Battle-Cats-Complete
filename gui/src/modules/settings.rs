mod addons;
mod disk;
mod exceptions;
mod keys;
mod pem;

use iced::widget::{
    button, column, container, opaque, pick_list, row, scrollable, stack, text, text_input, toggler, tooltip,
};
use iced::{Alignment, Element, Length, Size, Task, Theme};
use tracing::debug;

use core::modules::settings::lang;
use core::modules::settings::{
    ExportBehavior, Settings as CoreSettings, SidebarBehavior, UpdateMode,
};

use crate::app::theme;
#[cfg(target_os = "linux")]
use crate::common::feedback::Slot;
use crate::widget::{list_row, smooth_scroll};

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
    TabSelected(Tab),
    ToggleLogging(bool),
    ToggleNightly(bool),
    UpdateModeSelected(UpdateMode),
    LanguageMoveUp(usize),
    LanguageMoveDown(usize),
    RestoreDefaultLanguages,
    #[cfg(target_os = "linux")]
    ToggleDesktopData,
    #[cfg(target_os = "linux")]
    DesktopFeedbackExpired,
    ManualUpdateCheck,
    PreferredBannerSelected(usize),
    ToggleInvalidCats(bool),
    ToggleExpandSpirit(bool),
    DefaultLevelChanged(String),
    ToggleAutoLevel(bool),
    ToggleBumpUltra(bool),
    ToggleInvalidEnemies(bool),
    SidebarBehaviorSelected(SidebarBehavior),
    ExportBehaviorSelected(ExportBehavior),
    ToggleKeyValidation(bool),
    ToggleAppPersistence(bool),
    Keys(keys::Message),
    OpenKeysPopup,
    Exceptions(exceptions::Message),
    Disk(disk::Message),
    Pem(pem::Message),
    Addons(addons::Message),
    ToggleDebugView(bool),
    ToggleTightBounds(bool),
    ToggleAutoCamera(bool),
    ShowcaseWalkChanged(String),
    ShowcaseIdleChanged(String),
    ShowcaseKbChanged(String),
}

pub struct State {
    pub active_tab: Tab,

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
    desktop_feedback: Slot<DesktopFeedback>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: Tab::General,
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
            desktop_feedback: Slot::default(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, core_settings: &mut CoreSettings) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                match tab {
                    Tab::General => lang::ensure_complete_list(&mut core_settings.general.language_priority),
                    Tab::Cats => self.default_cat_level_buffer = core_settings.cat_data.default_level.to_string(),
                    Tab::Animation => {
                        self.showcase_walk_buffer = core_settings.animation.default_showcase_walk.to_string();
                        self.showcase_idle_buffer = core_settings.animation.default_showcase_idle.to_string();
                        self.showcase_kb_buffer = core_settings.animation.default_showcase_kb.to_string();
                    }
                    _ => {}
                }
                Task::none()
            }

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
                let kind = if success { feedback } else { DesktopFeedback::Failed };
                self.desktop_feedback.set(kind, Message::DesktopFeedbackExpired)
            }
            #[cfg(target_os = "linux")]
            Message::DesktopFeedbackExpired => {
                self.desktop_feedback.expire();
                Task::none()
            }
            Message::ManualUpdateCheck => {
                debug!("Manual update check requested, deferring to root app");
                Task::none()
            }

            Message::PreferredBannerSelected(val) => {
                core_settings.cat_data.preferred_banner_form = val;
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

            Message::ToggleInvalidEnemies(val) => {
                core_settings.enemy_data.show_invalid_enemies = val;
                Task::none()
            }

            Message::SidebarBehaviorSelected(val) => {
                core_settings.stages.sidebar_behavior = val;
                Task::none()
            }

            Message::ExportBehaviorSelected(val) => {
                core_settings.mods.export_behavior = val;
                Task::none()
            }
            Message::Pem(msg) => self.pem.update(msg).map(Message::Pem),

            Message::ToggleKeyValidation(val) => {
                core_settings.game_data.enforce_key_validation = val;
                Task::none()
            }
            Message::ToggleAppPersistence(val) => {
                core_settings.game_data.app_folder_persistence = val;
                Task::none()
            }
            Message::Keys(msg) => self.keys.update(msg).map(Message::Keys),
            Message::OpenKeysPopup => self.keys.update(keys::Message::Open).map(Message::Keys),
            Message::Exceptions(msg) => self.exceptions.update(msg).map(Message::Exceptions),
            Message::Disk(msg) => self.disk.update(msg).map(Message::Disk),

            Message::Addons(msg) => self.addons.update(msg).map(Message::Addons),

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

    pub fn keys_popup_open(&self) -> bool {
        self.keys.is_open
    }

    pub fn exceptions_popup_open(&self) -> bool {
        self.exceptions.is_open
    }

    pub fn pem_popup_open(&self) -> bool {
        self.pem.is_open
    }

    pub fn keys_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.keys.is_open.then(|| self.keys.view(window).map(Message::Keys))
    }

    pub fn exceptions_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.exceptions.is_open.then(|| self.exceptions.view(window).map(Message::Exceptions))
    }

    pub fn pem_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.pem.is_open.then(|| self.pem.view(window).map(Message::Pem))
    }

    pub fn view<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let main_content = row![
            self.view_sidebar(),
            smooth_scroll(
                scrollable(container(self.view_tab_content(core_settings)).padding(15))
                    .width(Length::Fill)
                    .height(Length::Fill)
            ),
        ]
            .height(Length::Fill);

        let modal: Option<Element<'a, Message>> = if self.addons.is_modal_open() {
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

    fn view_sidebar<'a>(&'a self) -> Element<'a, Message> {
        const SIDEBAR_WIDTH: f32 = 110.0;

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

        let mut tab_list = column![].spacing(4);

        for (tab_enum, label) in tabs {
            let is_active = self.active_tab == tab_enum;
            let row_content = container(theme::button_label(label).size(14)).padding([8, 12]).width(Length::Fill);

            tab_list = tab_list.push(list_row(row_content, is_active, true, Length::Fill, Message::TabSelected(tab_enum)));
        }

        container(smooth_scroll(scrollable(tab_list).width(Length::Fill).height(Length::Fill)))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(8)
            .style(theme::list_panel_container)
            .into()
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
        let update_modes = vec!["Prompt", "Ignore"];
        let current_update_mode = match core_settings.general.update_mode {
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
            let (label, color) = match self.desktop_feedback.get() {
                Some(DesktopFeedback::Created) => ("Desktop Data Created!", [40, 160, 40]),
                Some(DesktopFeedback::Deleted) => ("Desktop Data Deleted!", [40, 160, 40]),
                Some(DesktopFeedback::Failed) => ("Failed!", [180, 50, 50]),
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
            hover_hint(
                row![
                    toggler(core_settings.general.enable_logging).on_toggle(Message::ToggleLogging).style(theme::ios_toggle),
                    text("Enable Logging"),
                ].spacing(10).align_y(Alignment::Center),
                "Enables logs for easy debugging\nDisable to improve performance\nDevs may refuse to debug without logs",
            ),
            row![
                toggler(core_settings.general.enable_nightly).on_toggle(Message::ToggleNightly).style(theme::ios_toggle),
                text("Enable Nightly Features 🌙"),
            ].spacing(10).align_y(Alignment::Center),
            row![
                text("Update Handling"),
                pick_list(
                    update_modes,
                    Some(current_update_mode),
                    |val| {
                        let mode = match val {
                            "Prompt" => UpdateMode::Prompt,
                            _ => UpdateMode::Ignore,
                        };
                        Message::UpdateModeSelected(mode)
                    }
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
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
                text("Preferred Banner Form"),
                pick_list(
                    banner_options,
                    Some(core_settings.cat_data.preferred_banner_form),
                    Message::PreferredBannerSelected,
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
            ].spacing(10).align_y(Alignment::Center),

            row![
                toggler(core_settings.cat_data.show_invalid_cats).on_toggle(Message::ToggleInvalidCats).style(theme::ios_toggle),
                text("Show Invalid Cats"),
            ].spacing(10).align_y(Alignment::Center),

            text("Ability Display").size(24),
            row![
                toggler(core_settings.cat_data.expand_spirit_details).on_toggle(Message::ToggleExpandSpirit).style(theme::ios_toggle),
                text("Expand Spirit Details by Default"),
            ].spacing(10).align_y(Alignment::Center),

            text("Level Display").size(24),
            row![
                text("Default Level"),
                text_input("Level", &self.default_cat_level_buffer)
                    .on_input_maybe((!core_settings.cat_data.auto_level_calculations).then_some(Message::DefaultLevelChanged))
                    .width(Length::Fixed(60.0))
                    .style(theme::rounded_input),
            ].spacing(10).align_y(Alignment::Center),

            hover_hint(
                row![
                    toggler(core_settings.cat_data.auto_level_calculations).on_toggle(Message::ToggleAutoLevel).style(theme::ios_toggle),
                    text("Auto Level Calculations"),
                ].spacing(10).align_y(Alignment::Center),
                "Automatically calculates the max reasonable level for a unit based on their level caps",
            ),

            hover_hint(
                row![
                    toggler(core_settings.cat_data.bump_ultra_60).on_toggle(Message::ToggleBumpUltra).style(theme::ios_toggle),
                    text("Lv60 For Ultra"),
                ].spacing(10).align_y(Alignment::Center),
                "Automatically bumps the level to 60 (if not higher already) when an Ultra Form or Ultra Talent is selected",
            ),
        ].spacing(20).into()
    }

    fn view_enemies<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            text("Enemy List").size(24),
            row![
                toggler(core_settings.enemy_data.show_invalid_enemies).on_toggle(Message::ToggleInvalidEnemies).style(theme::ios_toggle),
                text("Show Invalid Enemies"),
            ].spacing(10).align_y(Alignment::Center),
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
                text("Sidebar Behavior"),
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
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
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
                hover_hint(
                    text("Export Behavior"),
                    "Determines whether to scan and automatically choose, always create a new APK, or always overwrite the input APK.",
                ),
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
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
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

            hover_hint(
                row![
                    toggler(core_settings.game_data.enforce_key_validation).on_toggle(Message::ToggleKeyValidation).style(theme::ios_toggle),
                    text("Enforce Key Validation"),
                ].spacing(10).align_y(Alignment::Center),
                "Prevents decryption/encryption if the cryptographic keys don't match the known official file hashes\nTurn this off only if the game keys have changed and you haven't updated BCC yet",
            ),
            text("Android").size(24),
            hover_hint(
                row![
                    toggler(core_settings.game_data.app_folder_persistence).on_toggle(Message::ToggleAppPersistence).style(theme::ios_toggle),
                    text("App Folder Persistence"),
                ].spacing(10).align_y(Alignment::Center),
                "Skip the deletion of the \"game/app\" directory after android import",
            ),
        ].spacing(20).into()
    }

    fn view_animation<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            text("Viewer").size(24),
            row![
                toggler(core_settings.animation.debug_view).on_toggle(Message::ToggleDebugView).style(theme::ios_toggle),
                text("Enable Debug View"),
            ].spacing(10).align_y(Alignment::Center),

            text("Exporter").size(24),
            hover_hint(
                row![
                    toggler(core_settings.animation.use_tight_bounds).on_toggle(Message::ToggleTightBounds).style(theme::ios_toggle),
                    text("Use Tight Bounds"),
                ].spacing(10).align_y(Alignment::Center),
                "Automatically crops out minor vfx and glow when calculating camera bounds",
            ),
            hover_hint(
                row![
                    toggler(core_settings.animation.auto_set_camera_region).on_toggle(Message::ToggleAutoCamera).style(theme::ios_toggle),
                    text("Auto-Set Camera Region"),
                ].spacing(10).align_y(Alignment::Center),
                "Automatically calculates a Units tight bounding box when exporting\nThis setting may cause lag spikes on some devices",
            ),

            text("Showcase").size(18),
            row![text("Walk Frames"), text_input("0", &self.showcase_walk_buffer).on_input(Message::ShowcaseWalkChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("Idle Frames"), text_input("0", &self.showcase_idle_buffer).on_input(Message::ShowcaseIdleChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("KB Frames"), text_input("0", &self.showcase_kb_buffer).on_input(Message::ShowcaseKbChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
        ].spacing(20).into()
    }

    fn view_about<'a>(&'a self) -> Element<'a, Message> {
        let license_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../core/assets/licenses.txt"));

        column![
            text("About Battle Cats Complete").size(32),
            text("A high-performance Battle Cats toolkit built by Omochi"),
            text("Open Source & Legal Info").size(20),
            smooth_scroll(
                scrollable(text(license_text).size(11))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
        ].spacing(15).into()
    }
}

fn hover_hint<'a, M: 'a>(content: impl Into<Element<'a, M>>, hint: &'a str) -> Element<'a, M> {
    tooltip(
        content,
        container(text(hint)).padding(6).style(container::bordered_box),
        tooltip::Position::Top,
    )
    .into()
}
