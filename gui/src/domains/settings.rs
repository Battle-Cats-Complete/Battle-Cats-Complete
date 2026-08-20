mod addons;
mod disk;
mod exceptions;
pub(crate) mod general;
mod keys;
mod pem;

use iced::mouse::Interaction;
use iced::widget::{
    column, container, mouse_area, pick_list, row, rule, scrollable, text, text_input, Space, Stack,
};
use iced::{Alignment, Element, Length, Size, Task};

use kore::domains::settings::{lang, nightly, ContextScope, EditorMode, EditorValues};
use kore::domains::settings::{
    ExportBehavior, ImportStructure, Settings as CoreSettings, SidebarBehavior,
};

use crate::app::theme;
use crate::common::feedback::NIGHTLY_ONLY_NOTICE;
use crate::app::UpdateStatus;
use crate::widget::{combo_row, hover_hint, list_row, smooth_scroll, toggle_row};

const SECTION_SPACING: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Cats,
    Enemies,
    Stages,
    Mods,
    Files,
    Import,
    Animation,
    AddOns,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(Tab),
    General(general::Message),
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
    ToggleIgnoreModifiedApp(bool),
    ToggleAppPersistence(bool),
    ImportStructureSelected(ImportStructure),
    Keys(keys::Message),
    OpenKeysPopup,
    Exceptions(exceptions::Message),
    Disk(disk::Message),
    Pem(pem::Message),
    Addons(addons::Message),
    ToggleDebugView(bool),
    ToggleUnlockGameMount(bool),
    EditorModeSelected(EditorMode),
    ContextScopeSelected(ContextScope),
    EditorValuesSelected(EditorValues),
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

    general: general::State,
    keys: keys::State,
    exceptions: exceptions::State,
    pem: pem::State,
    addons: addons::State,
    disk: disk::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: Tab::General,
            default_cat_level_buffer: "1".to_string(),
            showcase_walk_buffer: "0".to_string(),
            showcase_idle_buffer: "0".to_string(),
            showcase_kb_buffer: "0".to_string(),
            general: general::State::default(),
            keys: keys::State::default(),
            exceptions: exceptions::State::default(),
            pem: pem::State::default(),
            addons: addons::State::default(),
            disk: disk::State::default(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, core_settings: &mut CoreSettings) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                match tab {
                    Tab::General => {
                        lang::ensure_complete_list(&mut core_settings.general.language_priority);
                        if !nightly::features_available() {
                            core_settings.general.enable_nightly = false;
                        }
                    }
                    Tab::Cats => self.default_cat_level_buffer = core_settings.cat_data.default_level.to_string(),
                    Tab::Files => return self.disk.update(disk::Message::Refresh).map(Message::Disk),
                    Tab::Animation => {
                        self.showcase_walk_buffer = core_settings.animation.default_showcase_walk.to_string();
                        self.showcase_idle_buffer = core_settings.animation.default_showcase_idle.to_string();
                        self.showcase_kb_buffer = core_settings.animation.default_showcase_kb.to_string();
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::General(msg) => self.general.update(msg, core_settings).map(Message::General),

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
            Message::ToggleIgnoreModifiedApp(val) => {
                core_settings.game_data.ignore_modified_app = val;
                Task::none()
            }
            Message::ToggleAppPersistence(val) => {
                core_settings.game_data.app_folder_persistence = val;
                Task::none()
            }
            Message::ImportStructureSelected(structure) => {
                core_settings.game_data.import_structure = structure;
                Task::none()
            }
            Message::Keys(msg) => self.keys.update(msg).map(Message::Keys),
            Message::OpenKeysPopup => self.keys.update(keys::Message::Open).map(Message::Keys),
            Message::Exceptions(msg) => self.exceptions.update(msg).map(Message::Exceptions),
            Message::Disk(msg) => self.disk.update(msg).map(Message::Disk),

            Message::Addons(msg) => self.addons.update(msg).map(Message::Addons),

            Message::EditorModeSelected(mode) => {
                core_settings.files.editor_mode = mode;
                Task::none()
            }
            Message::ContextScopeSelected(scope) => {
                core_settings.files.context_scope = scope;
                Task::none()
            }
            Message::EditorValuesSelected(values) => {
                core_settings.files.editor_values = values;
                Task::none()
            }
            Message::ToggleUnlockGameMount(val) => {
                core_settings.files.unlock_game_mount = val;
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

    pub fn take_language_change(&mut self) -> bool {
        self.general.take_language_change()
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

    pub fn view<'a>(&'a self, core_settings: &'a CoreSettings, updater_status: &'a UpdateStatus) -> Element<'a, Message> {
        let tab_area: Element<'a, Message> = if self.active_tab == Tab::About {
            container(self.view_about()).width(Length::Fill).height(Length::Fill).padding(15).into()
        } else {
            smooth_scroll(
                scrollable(container(self.view_tab_content(core_settings, updater_status)).padding(15))
                    .width(Length::Fill)
                    .height(Length::Fill)
            ).into()
        };

        let main_content = row![self.view_sidebar(), tab_area].height(Length::Fill);

        let mut layers: Vec<Element<'a, Message>> = vec![main_content.into()];

        if self.active_tab == Tab::General && self.general.is_dragging() {
            layers.push(
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                    .interaction(Interaction::Grabbing)
                    .on_move(|point| Message::General(general::Message::LanguageDragMove(point)))
                    .on_release(Message::General(general::Message::LanguageDragEnd))
                    .into()
            );
        }

        Stack::with_children(layers).into()
    }

    fn view_sidebar<'a>(&'a self) -> Element<'a, Message> {
        const SIDEBAR_WIDTH: f32 = 110.0;

        let tabs = [
            (Tab::General, "General"),
            (Tab::Cats, "Cats"),
            (Tab::Enemies, "Enemies"),
            (Tab::Stages, "Stages"),
            (Tab::Mods, "Mods"),
            (Tab::Files, "Files"),
            (Tab::Import, "Import"),
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

    fn view_tab_content<'a>(&'a self, core_settings: &'a CoreSettings, updater_status: &'a UpdateStatus) -> Element<'a, Message> {
        match self.active_tab {
            Tab::General => column![
                header_section(text("Keys & IV").size(24), self.view_keys(core_settings)),
                self.general.view(core_settings, updater_status).map(Message::General),
            ].spacing(SECTION_SPACING).into(),
            Tab::Cats => self.view_cats(core_settings),
            Tab::Enemies => self.view_enemies(core_settings),
            Tab::Stages => self.view_stages(core_settings),
            Tab::Mods => self.view_mods(core_settings),
            Tab::Files => self.view_files(core_settings),
            Tab::Import => self.view_import(core_settings),
            Tab::Animation => self.view_animation(core_settings),
            Tab::AddOns => self.addons.view().map(Message::Addons),
            Tab::About => self.view_about(),
        }
    }

    fn view_cats<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let banner_options: Vec<usize> = vec![0, 1, 2, 3];

        let list_content = column![
            row![
                text("Preferred Banner Form"),
                pick_list(
                    banner_options,
                    Some(core_settings.cat_data.preferred_banner_form),
                    Message::PreferredBannerSelected,
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
            ].spacing(10).align_y(Alignment::Center),

            toggle_row(core_settings.cat_data.show_invalid_cats, text("Show Invalid Cats"), Some(Message::ToggleInvalidCats)),
        ].spacing(10);

        let ability_content = toggle_row(core_settings.cat_data.expand_spirit_details, text("Expand Spirit Details by Default"), Some(Message::ToggleExpandSpirit));

        let level_content = column![
            row![
                text("Default Level"),
                text_input("Level", &self.default_cat_level_buffer)
                    .on_input_maybe((!core_settings.cat_data.auto_level_calculations).then_some(Message::DefaultLevelChanged))
                    .width(Length::Fixed(60.0))
                    .style(theme::rounded_input),
            ].spacing(10).align_y(Alignment::Center),

            hover_hint(
                toggle_row(core_settings.cat_data.auto_level_calculations, text("Auto Level Calculations"), Some(Message::ToggleAutoLevel)),
                "Automatically calculates the max reasonable level for a unit based on their level caps",
            ),

            hover_hint(
                toggle_row(core_settings.cat_data.bump_ultra_60, text("Lv60 For Ultra"), Some(Message::ToggleBumpUltra)),
                "Automatically bumps the level to 60 (if not higher already) when an Ultra Form or Ultra Talent is selected",
            ),
        ].spacing(10);

        column![
            header_section(text("Cat List").size(24), list_content),
            header_section(text("Ability Display").size(24), ability_content),
            header_section(text("Level Display").size(24), level_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_enemies<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let list_content = toggle_row(core_settings.enemy_data.show_invalid_enemies, text("Show Invalid Enemies"), Some(Message::ToggleInvalidEnemies));

        column![
            header_section(text("Enemy List").size(24), list_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_stages<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let sidebar_options = vec!["Cover", "Push"];
        let current_sidebar = match core_settings.stages.sidebar_behavior {
            SidebarBehavior::Cover => "Cover",
            SidebarBehavior::Push => "Push",
        };

        let list_content = row![
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
        ].spacing(10).align_y(Alignment::Center);

        column![
            header_section(text("Stage List").size(24), list_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_mods<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let export_options = vec!["Automatic", "Create", "Update"];
        let current_export = match core_settings.mods.export_behavior {
            ExportBehavior::Automatic => "Automatic",
            ExportBehavior::Create => "Create",
            ExportBehavior::Update => "Update",
        };

        let export_content = column![
            theme::sized_button("Manage PEM", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Pem(pem::Message::Open)),
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
        ].spacing(10);

        column![
            header_section(text("Export").size(24), export_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_keys<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            theme::sized_button("Manage Keys", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Keys(keys::Message::Open)),
            hover_hint(
                toggle_row(core_settings.game_data.enforce_key_validation, text("Enforce Key Validation"), Some(Message::ToggleKeyValidation)),
                "Prevents decryption/encryption if the cryptographic keys don't match the known official file hashes\nTurn this off only if the game keys have changed and you haven't updated BCC yet",
            ),
        ].spacing(10).into()
    }

    fn view_import<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let management_content = column![
            theme::sized_button("Manage Exceptions", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Exceptions(exceptions::Message::Open)),

            hover_hint(
                row![
                    text("Import Structure"),
                    pick_list(
                        ImportStructure::ALL,
                        Some(core_settings.game_data.import_structure),
                        Message::ImportStructureSelected,
                    ).style(theme::combo_box).menu_style(theme::combo_box_menu),
                ].spacing(10).align_y(Alignment::Center),
                core_settings.game_data.import_structure.hint(),
            ),

            hover_hint(
                toggle_row(core_settings.game_data.ignore_modified_app, text("Ignore Modified App"), Some(Message::ToggleIgnoreModifiedApp)),
                "Imports modded versions of the app with Vanilla package names as if they are Vanilla intalls, bypassing the import refusal",
            ),
        ].spacing(10);

        let android_content = hover_hint(
            toggle_row(core_settings.game_data.app_folder_persistence, text("App Folder Persistence"), Some(Message::ToggleAppPersistence)),
            "Skip the deletion of the \"game/app\" directory after android import",
        );

        column![
            header_section(text("Management").size(24), management_content),
            header_section(text("Android").size(24), android_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_files<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let mount_row = hover_hint(
            toggle_row(
                core_settings.files.unlock_game_mount,
                text("Unlock \"game\" Mount"),
                Some(Message::ToggleUnlockGameMount),
            ),
            "Allows editing vanilla game files in place\nPrefer creating a Mod so the original data stays intact",
        );

        let mode = core_settings.files.editor_mode;

        let mode_row = combo_row(
            "UTF-8 Mode",
            mode.hint(),
            EditorMode::ALL,
            Some(mode),
            Some(Message::EditorModeSelected),
        );

        let nightly = core_settings.general.enable_nightly;
        let scope = core_settings.files.context_scope;

        let scope_row = combo_row(
            "Context Scope",
            if nightly { scope.hint() } else { NIGHTLY_ONLY_NOTICE },
            ContextScope::ALL,
            Some(scope),
            nightly.then_some(Message::ContextScopeSelected),
        );

        let values = core_settings.files.editor_values;

        let values_row = combo_row(
            "Editor Values",
            if nightly { values.hint() } else { NIGHTLY_ONLY_NOTICE },
            EditorValues::ALL,
            Some(values),
            nightly.then_some(Message::EditorValuesSelected),
        );

        column![
            header_section(text("Disk").size(24), self.disk.view().map(Message::Disk)),
            header_section(text("Viewer").size(24), mode_row),
            header_section(text("Editor").size(24), column![scope_row, values_row, mount_row].spacing(10)),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_animation<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let viewer_content = toggle_row(core_settings.animation.debug_view, text("Enable Debug View"), Some(Message::ToggleDebugView));

        let exporter_content = column![
            hover_hint(
                toggle_row(core_settings.animation.use_tight_bounds, text("Use Tight Bounds"), Some(Message::ToggleTightBounds)),
                "Automatically crops out minor vfx and glow when calculating camera bounds",
            ),
            hover_hint(
                toggle_row(core_settings.animation.auto_set_camera_region, text("Auto-Set Camera Region"), Some(Message::ToggleAutoCamera)),
                "Automatically calculates a Units tight bounding box when exporting\nThis setting may cause lag spikes on some devices",
            ),
        ].spacing(10);

        let showcase_content = column![
            row![text("Walk Frames"), text_input("0", &self.showcase_walk_buffer).on_input(Message::ShowcaseWalkChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("Idle Frames"), text_input("0", &self.showcase_idle_buffer).on_input(Message::ShowcaseIdleChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("KB Frames"), text_input("0", &self.showcase_kb_buffer).on_input(Message::ShowcaseKbChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
        ].spacing(10);

        column![
            header_section(text("Viewer").size(24), viewer_content),
            header_section(text("Exporter").size(24), exporter_content),
            header_section(text("Showcase").size(18), showcase_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_about<'a>(&'a self) -> Element<'a, Message> {
        let license_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../kore/assets/licenses.txt"));

        let header = column![
            text("About Battle Cats Complete").size(32),
            text("A high-performance Battle Cats toolkit built by Omochi"),
            text("Open Source & Legal Info").size(20),
        ].spacing(10);

        let legal_area = container(
            smooth_scroll(
                scrollable(text(license_text).size(11))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
        ).width(Length::Fill).height(Length::Fill);

        column![
            column![header, rule::horizontal(1)].spacing(15),
            legal_area,
        ].spacing(0).height(Length::Fill).into()
    }
}

fn header_section<'a, M: 'a>(header: impl Into<Element<'a, M>>, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![header.into(), content.into()].spacing(10).into()
}

