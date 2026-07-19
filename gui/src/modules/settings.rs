use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{
    button, column, container, opaque, pick_list, row, scrollable, stack, text, text_input, toggler,
    Container, Scrollable,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use tracing::{debug, error, info, trace, warn};

use core::modules::settings::{
    ExportBehavior, ExceptionRule, RuleHandling, Settings as CoreSettings, SidebarBehavior, UpdateMode,
};

use crate::app::UpdaterAction;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    DeleteGameFolder,
    DeleteRawFolder,
    DeleteCache,
    DeleteAddon(String),
    ResetExceptions,
    ConfirmPemGenerate,
    ConfirmPemDelete,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TabSelected(Tab),
    CloseModal,
    OpenModal(Modal),

    // General Tab
    ToggleLogging(bool),
    ToggleNightly(bool),
    UpdateModeSelected(UpdateMode),
    LanguageMoveUp(usize),
    LanguageMoveDown(usize),
    RestoreDefaultLanguages,
    CreateDesktopData,
    DeleteDesktopData,
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
    ExecuteFolderDeletion(Modal),
    ToggleKeyValidation(bool),
    ToggleUltraCompression(bool),
    ManualIpChanged(String),
    ToggleAppPersistence(bool),
    RevealIpField(bool),

    // Addons Tab
    ExecuteAddonDeletion(String),
    InstallAddon(String),
    OemDriverSelected(u8),
    DownloadOemDriver,

    // Animation Tab
    CenteringBehaviorSelected(usize),
    ToggleDebugView(bool),
    ToggleTightBounds(bool),
    ToggleAutoCamera(bool),
    ShowcaseWalkChanged(String),
    ShowcaseIdleChanged(String),
    ShowcaseKbChanged(String),

    // PEM Modals
    PemImportRequested,
    PemExportRequested,
    PemGenerated,
    ExecutePemGenerate,
    ExecutePemDelete,
    FilePicked(Option<PathBuf>),
}

pub struct State {
    pub active_tab: Tab,
    pub active_modal: Option<Modal>,
    pub ip_field_revealed: bool,
    pub manual_ip_buffer: String,

    pub default_cat_level_buffer: String,
    pub showcase_walk_buffer: String,
    pub showcase_idle_buffer: String,
    pub showcase_kb_buffer: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: Tab::General,
            active_modal: None,
            ip_field_revealed: false,
            manual_ip_buffer: String::new(),
            default_cat_level_buffer: "1".to_string(),
            showcase_walk_buffer: "0".to_string(),
            showcase_idle_buffer: "0".to_string(),
            showcase_kb_buffer: "0".to_string(),
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
                // Background polling placeholder for Addons/Deleters
                Task::none()
            }
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            Message::CloseModal => {
                self.active_modal = None;
                Task::none()
            }
            Message::OpenModal(modal) => {
                self.active_modal = Some(modal);
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
                core_settings.general.language_priority = vec![
                    "en".to_string(), "ja".to_string(), "tw".to_string(), "ko".to_string()
                ];
                Task::none()
            }
            Message::CreateDesktopData => {
                info!("Creating desktop data integration");
                Task::none()
            }
            Message::DeleteDesktopData => {
                info!("Deleting desktop data integration");
                Task::none()
            }
            Message::ManualUpdateCheck => {
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

            // Data Tab
            Message::ExecuteFolderDeletion(target_modal) => {
                match target_modal {
                    Modal::DeleteGameFolder => info!("Deleting game folder"),
                    Modal::DeleteRawFolder => info!("Deleting raw folder"),
                    Modal::DeleteCache => info!("Clearing cache"),
                    _ => {}
                }
                self.active_modal = None;
                Task::none()
            }
            Message::ToggleKeyValidation(val) => {
                core_settings.game_data.enforce_key_validation = val;
                Task::none()
            }
            Message::ToggleUltraCompression(val) => {
                core_settings.game_data.enable_ultra_compression = val;
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

            // Addons Tab
            Message::InstallAddon(name) => {
                info!("Triggering addon installation: {}", name);
                Task::none()
            }
            Message::ExecuteAddonDeletion(name) => {
                info!("Executing deletion of addon: {}", name);
                self.active_modal = None;
                Task::none()
            }
            Message::OemDriverSelected(_idx) => {
                Task::none()
            }
            Message::DownloadOemDriver => {
                info!("Downloading OEM Driver");
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

            // PEM
            Message::PemImportRequested => {
                Task::perform(
                    async {
                        let path_handle = rfd::AsyncFileDialog::new()
                            .add_filter("PEM", &["pem", "txt"])
                            .pick_file()
                            .await;
                        path_handle.map(|h| h.path().to_path_buf())
                    },
                    Message::FilePicked,
                )
            }
            Message::FilePicked(path_opt) => {
                if let Some(path) = path_opt {
                    info!("File picked: {:?}", path);
                }
                Task::none()
            }
            Message::PemExportRequested => {
                info!("Exporting PEM");
                Task::none()
            }
            Message::ExecutePemGenerate => {
                info!("Generating new PEM");
                self.active_modal = None;
                Task::none()
            }
            Message::ExecutePemDelete => {
                info!("Deleting custom PEM");
                self.active_modal = None;
                Task::none()
            }
            Message::PemGenerated => {
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

        if let Some(modal) = &self.active_modal {
            let modal_content = self.view_modal(modal);
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
            Tab::AddOns => self.view_addons(),
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
        for (i, lang) in core_settings.general.language_priority.iter().enumerate() {
            let row_lang = row![
                text(lang).width(Length::Fixed(80.0)),
                button("↑").on_press_maybe(if i > 0 { Some(Message::LanguageMoveUp(i)) } else { None }),
                button("↓").on_press_maybe(if i < langs_len.saturating_sub(1) { Some(Message::LanguageMoveDown(i)) } else { None }),
            ].spacing(5).align_y(Alignment::Center);
            lang_col = lang_col.push(row_lang);
        }

        column![
            text("System").size(24),
            row![
                button("Create Desktop Data").on_press(Message::CreateDesktopData),
                button("Delete Desktop Data").on_press(Message::DeleteDesktopData).style(button::danger),
            ].spacing(10),
            button("Check for Update Now").on_press(Message::ManualUpdateCheck),

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
            button("Manage PEM").on_press(Message::OpenModal(Modal::ConfirmPemGenerate)), // Shortcut for now
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
            row![
                button("Delete \"game\" Folder").style(button::danger).on_press(Message::OpenModal(Modal::DeleteGameFolder)),
                button("Delete \"raw\" Folder").style(button::danger).on_press(Message::OpenModal(Modal::DeleteRawFolder)),
                button("Clear Cache").style(button::danger).on_press(Message::OpenModal(Modal::DeleteCache)),
            ].spacing(10),

            text("Management").size(24),
            row![
                button("Manage Keys").on_press(Message::OpenModal(Modal::ResetExceptions)), // Mocking modal for now
                button("Manage Exceptions").on_press(Message::OpenModal(Modal::ResetExceptions)),
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
                text_input("192.168.X.X", &self.manual_ip_buffer).on_input(Message::ManualIpChanged).width(Length::Fixed(120.0)),
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

    fn view_addons<'a>(&'a self) -> Element<'a, Message> {
        column![
            text("Android Bridge").size(24),
            text("Enables Android device & emulator imports."),
            row![
                button("Download ADB").on_press(Message::InstallAddon("adb".to_string())).style(button::success),
                button("Delete ADB").on_press(Message::OpenModal(Modal::DeleteAddon("adb".to_string()))).style(button::danger),
            ].spacing(10),

            text("APKEditor").size(24),
            row![
                button("Download APKEditor").on_press(Message::InstallAddon("apkeditor".to_string())).style(button::success),
                button("Delete APKEditor").on_press(Message::OpenModal(Modal::DeleteAddon("apkeditor".to_string()))).style(button::danger),
            ].spacing(10),

            text("FFMPEG").size(24),
            row![
                button("Download FFMPEG").on_press(Message::InstallAddon("ffmpeg".to_string())).style(button::success),
                button("Delete FFMPEG").on_press(Message::OpenModal(Modal::DeleteAddon("ffmpeg".to_string()))).style(button::danger),
            ].spacing(10),

            text("AVIFENC").size(24),
            row![
                button("Download AVIFENC").on_press(Message::InstallAddon("avifenc".to_string())).style(button::success),
                button("Delete AVIFENC").on_press(Message::OpenModal(Modal::DeleteAddon("avifenc".to_string()))).style(button::danger),
            ].spacing(10),
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

    fn view_modal<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        let (title, content, confirm_msg) = match modal {
            Modal::DeleteGameFolder => (
                "Confirm Deletion",
                "Are you sure you want to delete the \"game\" folder?\nMost app function will be lost.",
                Message::ExecuteFolderDeletion(Modal::DeleteGameFolder)
            ),
            Modal::DeleteRawFolder => (
                "Confirm Deletion",
                "Are you sure you want to delete the \"raw\" folder?\nYou may need to import again.",
                Message::ExecuteFolderDeletion(Modal::DeleteRawFolder)
            ),
            Modal::DeleteCache => (
                "Confirm Deletion",
                "Are you sure you want to clear the Cache?",
                Message::ExecuteFolderDeletion(Modal::DeleteCache)
            ),
            Modal::DeleteAddon(name) => (
                "Confirm Deletion",
                "Are you sure you want to delete this addon?",
                Message::ExecuteAddonDeletion(name.clone())
            ),
            Modal::ResetExceptions => (
                "Confirm Reset",
                "Are you sure you want to reset to default exception rules?",
                Message::CloseModal
            ),
            Modal::ConfirmPemGenerate => (
                "Confirm Generate",
                "Are you sure you want to overwrite your current PEM?",
                Message::ExecutePemGenerate
            ),
            Modal::ConfirmPemDelete => (
                "Confirm Delete",
                "Are you sure you want to delete your custom PEM?",
                Message::ExecutePemDelete
            ),
        };

        container(
            column![
                text(title).size(24),
                text(content),
                row![
                    button("Yes").on_press(confirm_msg).style(button::danger),
                    button("No").on_press(Message::CloseModal),
                ].spacing(10).align_y(Alignment::Center)
            ].spacing(20).padding(25).align_x(Alignment::Center)
        )
            .style(|theme: &Theme| {
                container::background(theme.palette().background)
                    .border(iced::Border {
                        color: theme.palette().text,
                        width: 1.0,
                        radius: 8.0.into(),
                    })
            })
            .into()
    }
}