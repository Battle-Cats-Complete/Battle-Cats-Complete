mod export;
mod import;
mod list;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use iced::alignment::Horizontal;
use iced::futures::channel::mpsc;
use iced::widget::{
    button, column, container, row, rule, space, text, text_editor, text_input, Container,
};
use iced::{Alignment, Element, Length, Size, Task, Theme};
use tracing::{info, warn};

use kore::common::job::{JobEvent, JobOutcome};
use kore::domains::mods::{self, ModDataState};
use kore::domains::settings::Settings;
use kore::Vault;

use crate::app::state::ModsListState;
use crate::app::theme;
use crate::common::feedback::Slot;
use crate::widget::section;

const RULE_THICKNESS: f32 = 1.0;
const RULE_PADDING: f32 = 8.0;
const SECTION_GAP: f32 = 16.0;

const SCROLLBAR_GAP: f32 = 2.0;

const MOD_LIST_WIDTH: f32 = 200.0;
const SIDEBAR_PADDING: f32 = 8.0;

const POPUP_PADDING: f32 = 16.0;
const PICKED_LABEL_MAX: usize = 22;
const PICKED_LABEL_TAIL: usize = 6;

const DETAILS_PADDING: f32 = 16.0;
const FIELD_LABEL_WIDTH: f32 = 90.0;
const FIELD_GAP: f32 = 8.0;
const FIELD_ROW_SPACING: f32 = 8.0;
const CONTENT_WIDTH: f32 = 515.0;

const CARD_PADDING: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetadataField {
    Author,
    Version,
    Package,
    Description,
}

#[derive(Debug, Clone)]
pub enum Message {
    List(list::Message),
    Import(import::Message),
    Export(export::Message),

    SearchChanged(String),
    RenameBufferChanged(String),
    CommitRename,
    ToggleModStatus(String),
    OpenFolder(String),
    UpdateMetadata(MetadataField, String),
    DescriptionAction(text_editor::Action),
    CommitMetadata,
    DeleteRequested,
    DeleteExpired,
    DeleteFinished(Result<(), String>),
    MountFinished { folder: String, enabled: bool, error: Option<String> },
}

pub struct State {
    data: ModDataState,
    list: list::State,
    import: import::State,
    export: export::State,
    description: text_editor::Content,
    confirm_delete: Slot<()>,
    mounting: bool,
    mount_target: Option<String>,
}

impl State {
    pub fn new(mut data: ModDataState) -> Self {
        data.refresh_mods();

        let mut list = list::State::default();
        list.refresh(&data.loaded_mods, &data.search_query);

        Self {
            data,
            list,
            import: import::State::default(),
            export: export::State::default(),
            description: text_editor::Content::new(),
            confirm_delete: Slot::default(),
            mounting: false,
            mount_target: None,
        }
    }

    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub(crate) fn invalidate_assets(&mut self, folders: &HashSet<String>) {
        self.list.invalidate(folders, &self.data.loaded_mods);
    }

    pub(crate) fn resync(&self, vault: &Vault, folders: &HashSet<String>) {
        let Some(active) = self.active_mod() else {
            return;
        };

        if !folders.contains(&active) {
            return;
        }

        if let Err(err) = mods::enable(vault, &active) {
            warn!(mod_name = %active, "Failed to re-index a changed mod: {}", err);
        }
    }

    pub fn active_mod(&self) -> Option<String> {
        self.data.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone())
    }

    pub(crate) fn restore_state(&mut self, state: &ModsListState) {
        self.data.search_query = state.search_query.clone();
        self.list.refresh(&self.data.loaded_mods, &self.data.search_query);

        let Some(folder) = state.selected_mod.as_ref() else {
            return;
        };

        if !self.data.loaded_mods.iter().any(|entry| &entry.folder_name == folder) {
            info!(mod_name = %folder, "Persisted mod selection no longer exists, starting with none selected");
            return;
        }

        self.select_mod(folder.clone());
    }

    pub(crate) fn sync_state(&self, state: &mut ModsListState) {
        if state.selected_mod != self.data.selected_mod {
            state.selected_mod = self.data.selected_mod.clone();
        }
        if state.search_query != self.data.search_query {
            state.search_query = self.data.search_query.clone();
        }
    }

    pub(crate) fn mounting(&self) -> bool {
        self.mounting
    }

    pub(crate) fn intended_mod(&self) -> Option<String> {
        if self.mounting {
            return self.mount_target.clone();
        }

        self.active_mod()
    }

    pub fn update(&mut self, message: Message, settings: &Settings, vault: &Arc<Vault>) -> Task<Message> {
        let task = self.update_inner(message, settings, vault);

        self.list.refresh(&self.data.loaded_mods, &self.data.search_query);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &Settings, vault: &Arc<Vault>) -> Task<Message> {
        match message {
            Message::List(msg) => {
                if let list::Message::SelectMod(folder) = &msg {
                    self.select_mod(folder.clone());
                }
                self.list.update(msg);
                Task::none()
            }
            Message::Import(msg) => self.import.update(msg, &mut self.data, settings).map(Message::Import),
            Message::Export(msg) => {
                if matches!(msg, export::Message::Open) {
                    self.populate_export_metadata();
                }
                self.export.update(msg, &mut self.data, settings).map(Message::Export)
            }

            Message::SearchChanged(query) => {
                self.data.search_query = query;
                Task::none()
            }
            Message::RenameBufferChanged(new_name) => {
                self.data.rename_buffer = new_name;
                Task::none()
            }
            Message::CommitRename => self.commit_rename(vault),
            Message::ToggleModStatus(mod_folder) => self.toggle_mod_status(mod_folder, vault),
            Message::OpenFolder(mod_folder) => {
                let mod_path = Path::new("mods").join(&mod_folder);
                let _ = open::that(mod_path);
                Task::none()
            }
            Message::UpdateMetadata(field, value) => {
                self.update_metadata(field, value);
                Task::none()
            }
            Message::DescriptionAction(action) => {
                let is_edit = action.is_edit();
                self.description.perform(action);

                if is_edit {
                    self.update_metadata(MetadataField::Description, self.description.text());
                }
                Task::none()
            }
            Message::CommitMetadata => {
                self.commit_metadata();
                Task::none()
            }
            Message::DeleteRequested => {
                if self.confirm_delete.is_set() {
                    return self.delete_selected();
                }

                self.confirm_delete.set((), Message::DeleteExpired)
            }
            Message::DeleteExpired => {
                self.confirm_delete.expire();
                Task::none()
            }
            Message::MountFinished { folder, enabled, error } => {
                self.mounting = false;
                self.mount_target = None;

                for entry in self.data.loaded_mods.iter_mut() {
                    entry.enabled = false;
                }

                if let Some(err) = error {
                    warn!(mod_name = %folder, "Mount failed, mod left disabled: {}", err);
                    return Task::none();
                }

                if enabled && let Some(idx) = self.data.loaded_mods.iter().position(|m| m.folder_name == folder) {
                    self.data.loaded_mods[idx].enabled = true;
                }

                Task::none()
            }
            Message::DeleteFinished(result) => {
                if let Err(e) = result {
                    warn!("Failed to delete mod folder: {}", e);
                }
                self.data.refresh_mods();
                Task::none()
            }
        }
    }

    fn select_mod(&mut self, folder: String) {
        self.confirm_delete.clear();
        self.data.selected_mod = Some(folder.clone());
        self.data.rename_buffer = folder;

        let description = self.get_selected_mod_idx().map_or_else(
            text_editor::Content::new,
            |idx| text_editor::Content::with_text(&self.data.loaded_mods[idx].metadata.description),
        );
        self.description = description;

        self.populate_export_metadata();
    }

    fn toggle_mod_status(&mut self, mod_folder: String, vault: &Arc<Vault>) -> Task<Message> {
        if self.mounting || !self.data.loaded_mods.iter().any(|m| m.folder_name == mod_folder) {
            return Task::none();
        }

        self.confirm_delete.clear();

        let enabling = !self.data.loaded_mods.iter().any(|m| m.folder_name == mod_folder && m.enabled);
        let previous = self.data.active_mod();

        self.spawn_mount(vault, previous, enabling.then(|| mod_folder.clone()), mod_folder)
    }

    fn spawn_mount(
        &mut self,
        vault: &Arc<Vault>,
        previous: Option<String>,
        target: Option<String>,
        folder: String,
    ) -> Task<Message> {
        self.mounting = true;
        self.mount_target = target.clone();

        let vault = Arc::clone(vault);
        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            if let Some(name) = previous {
                mods::disable(&vault, &name);
            }

            let mut error = None;
            let enabled = target.is_some();

            if let Some(name) = target {
                match mods::enable(&vault, &name) {
                    Ok(conflicts) => {
                        for conflict in &conflicts {
                            warn!(key = %conflict.key, "duplicate filename in mod, all copies excluded: {:?}", conflict.paths);
                        }
                    }
                    Err(err) => error = Some(err.to_string()),
                }
            }

            let _ = tx.unbounded_send(Message::MountFinished { folder, enabled, error });
        });

        Task::stream(rx)
    }

    fn commit_rename(&mut self, vault: &Arc<Vault>) -> Task<Message> {
        let Some(idx) = self.get_selected_mod_idx() else { return Task::none(); };
        let old_name = self.data.loaded_mods[idx].folder_name.clone();
        let new_name = self.data.rename_buffer.clone();

        if self.mounting {
            self.data.rename_buffer = old_name;
            return Task::none();
        }

        if new_name.is_empty() || new_name == old_name {
            self.data.rename_buffer = old_name;
            return Task::none();
        }

        let mods_root = Path::new("mods");
        let old_path = mods_root.join(&old_name);
        let new_path = mods_root.join(&new_name);

        let recasing = new_name.eq_ignore_ascii_case(&old_name);
        let clashes = !recasing && mods::taken(mods_root, &new_name);

        if clashes || !old_path.exists() || fs::rename(&old_path, &new_path).is_err() {
            self.data.rename_buffer = old_name;
            return Task::none();
        }

        let was_enabled = self.data.loaded_mods[idx].enabled;

        self.data.loaded_mods[idx].folder_name = new_name.clone();
        self.data.selected_mod = Some(new_name.clone());
        self.data.loaded_mods[idx].metadata.title = new_name.clone();

        if let Err(e) = self.data.loaded_mods[idx].metadata.save(&new_path) {
            warn!("Failed to save renamed metadata: {}", e);
        }

        if !was_enabled {
            return Task::none();
        }

        self.spawn_mount(vault, Some(old_name), Some(new_name.clone()), new_name)
    }

    fn update_metadata(&mut self, field: MetadataField, value: String) {
        let Some(idx) = self.get_selected_mod_idx() else { return; };
        let meta = &mut self.data.loaded_mods[idx].metadata;

        match field {
            MetadataField::Author => meta.author = value,
            MetadataField::Version => meta.version = value,
            MetadataField::Package => meta.package = value,
            MetadataField::Description => meta.description = value,
        }

        self.commit_metadata();
    }

    fn commit_metadata(&mut self) {
        let Some(idx) = self.get_selected_mod_idx() else { return; };
        let mod_folder = self.data.loaded_mods[idx].folder_name.clone();
        let mod_path = Path::new("mods").join(&mod_folder);

        self.data.loaded_mods[idx].metadata.title = mod_folder;
        if let Err(e) = self.data.loaded_mods[idx].metadata.save(&mod_path) {
            tracing::error!("Failed to commit metadata: {}", e);
        }
    }

    fn delete_selected(&mut self) -> Task<Message> {
        self.confirm_delete.clear();

        let Some(mod_folder) = self.data.selected_mod.clone() else { return Task::none(); };

        if self.data.loaded_mods.iter().any(|m| m.folder_name == mod_folder && m.enabled) {
            warn!(mod_name = %mod_folder, "Refusing to delete an enabled mod; disable it first");
            return Task::none();
        }

        let path = Path::new("mods").join(&mod_folder);

        self.data.selected_mod = None;

        Task::perform(
            async move { fs::remove_dir_all(&path).map_err(|e| e.to_string()) },
            Message::DeleteFinished,
        )
    }

    fn get_selected_mod_idx(&self) -> Option<usize> {
        self.data.selected_mod.as_ref().and_then(|id| {
            self.data.loaded_mods.iter().position(|m| &m.folder_name == id)
        })
    }

    fn populate_export_metadata(&mut self) {
        if let Some(idx) = self.get_selected_mod_idx() {
            let meta = &self.data.loaded_mods[idx].metadata;
            self.data.export.app_title = meta.title.clone();
            self.data.export.package_suffix = meta.package.clone();
        } else {
            self.data.export.app_title.clear();
            self.data.export.package_suffix.clear();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        row![
            self.view_sidebar(),
            self.view_details()
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn import_popup_open(&self) -> bool {
        self.import.is_open
    }

    pub fn export_popup_open(&self) -> bool {
        self.export.is_open
    }

    pub fn import_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.import
            .is_open
            .then(|| self.import.view(&self.data, window).map(Message::Import))
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.export
            .is_open
            .then(|| self.export.view(&self.data, window).map(Message::Export))
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        const SEARCH_BUTTON_GAP: f32 = 4.0;
        const BUTTON_LIST_GAP: f32 = 8.0;

        let search_input = text_input("Search Mods...", &self.data.search_query)
            .on_input(Message::SearchChanged)
            .padding(4)
            .size(13)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let import_open = self.import_popup_open();
        let import_btn = button(theme::button_label("Add Mod").size(13))
            .on_press(Message::Import(import::Message::Open))
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |t: &Theme, status| theme::toggle_button(t, status, import_open));

        let mod_list = self.list.view(&self.data.loaded_mods, self.data.selected_mod.as_deref()).map(Message::List);

        container(
            column![
                search_input,
                space().height(SEARCH_BUTTON_GAP),
                import_btn,
                space().height(BUTTON_LIST_GAP),
                mod_list
            ]
                .spacing(0)
                .height(Length::Fill)
        )
            .width(Length::Fixed(MOD_LIST_WIDTH + SIDEBAR_PADDING * 2.0))
            .height(Length::Fill)
            .padding(SIDEBAR_PADDING)
            .style(theme::list_panel_container)
            .into()
    }

    fn view_details(&self) -> Element<'_, Message> {
        let Some(mod_idx) = self.get_selected_mod_idx() else {
            return container(theme::centered_text("Please select or import a Mod").size(18))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let mod_data = &self.data.loaded_mods[mod_idx];
        let is_enabled = mod_data.enabled;
        let mod_folder = mod_data.folder_name.clone();

        let header = text_input("Mod Title", &self.data.rename_buffer)
            .on_input(Message::RenameBufferChanged)
            .on_submit(Message::CommitRename)
            .size(22)
            .padding(8)
            .width(Length::Fixed(CONTENT_WIDTH))
            .style(theme::rounded_input);

        let toggle_style: theme::ButtonStyleFn = if is_enabled { theme::danger_button } else { theme::success_button };
        let delete_label = self.confirm_delete.confirm_label("Delete Mod");

        let actions_row = row![
            theme::sized_button(if is_enabled { "Disable Mod" } else { "Enable Mod" }, theme::POPUP_ACTION_BUTTON_WIDTH, toggle_style)
                .on_press_maybe((!self.mounting).then(|| Message::ToggleModStatus(mod_folder.clone()))),
            theme::sized_button("Open Folder", theme::POPUP_ACTION_BUTTON_WIDTH, theme::primary_button)
                .on_press(Message::OpenFolder(mod_folder)),
            theme::sized_button("Export Mod", theme::POPUP_ACTION_BUTTON_WIDTH, theme::primary_button)
                .on_press(Message::Export(export::Message::Open)),
            theme::sized_button(delete_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::danger_button)
                .on_press_maybe((!is_enabled && !self.mounting).then_some(Message::DeleteRequested)),
        ]
            .spacing(10)
            .align_y(Alignment::Center);

        let actions_row = container(actions_row)
            .width(Length::Fixed(CONTENT_WIDTH))
            .align_x(Horizontal::Center);

        let information = content_card(
            section(
                "Information",
                Length::Fill,
                column![
                    metadata_row("Author", &mod_data.metadata.author, MetadataField::Author),
                    metadata_row("Version", &mod_data.metadata.version, MetadataField::Version),
                    metadata_row("Package", &mod_data.metadata.package, MetadataField::Package),
                ].spacing(FIELD_ROW_SPACING),
            )
        );

        let description = content_card(
            section(
                "Description",
                Length::Fill,
                text_editor(&self.description)
                    .placeholder("Enter mod description here...")
                    .on_action(Message::DescriptionAction)
                    .padding(8)
                    .size(14)
                    .height(Length::Fill)
                    .style(theme::rounded_editor),
            )
        )
            .height(Length::Fill);

        container(
            column![
                header,
                space().height(RULE_PADDING),
                content_rule(),
                space().height(RULE_PADDING),
                actions_row,
                space().height(RULE_PADDING),
                content_rule(),
                space().height(RULE_PADDING),
                information,
                space().height(SECTION_GAP),
                description
            ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(DETAILS_PADDING)
            .into()
    }
}

fn field_row<'a, M: 'a>(label: &'a str, control: impl Into<Element<'a, M>>) -> Element<'a, M> {
    row![
        text(label).size(14).width(Length::Fixed(FIELD_LABEL_WIDTH)),
        control.into()
    ]
        .align_y(Alignment::Center)
        .spacing(FIELD_GAP)
        .width(Length::Fill)
        .into()
}

fn picked_label(path: &Path) -> String {
    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().to_string());
    let chars: Vec<char> = name.chars().collect();

    if chars.len() <= PICKED_LABEL_MAX {
        return name;
    }

    let head: String = chars[..PICKED_LABEL_MAX - PICKED_LABEL_TAIL - 3].iter().collect();
    let tail: String = chars[chars.len() - PICKED_LABEL_TAIL..].iter().collect();

    format!("{}…{}", head, tail)
}

fn content_rule<'a>() -> Element<'a, Message> {
    container(rule::horizontal(RULE_THICKNESS)).width(Length::Fixed(CONTENT_WIDTH)).into()
}

fn content_card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content.into())
        .width(Length::Fixed(CONTENT_WIDTH))
        .padding(CARD_PADDING)
        .style(theme::card_container_outlined)
}

fn metadata_row<'a>(label: &'a str, value: &'a str, field: MetadataField) -> Element<'a, Message> {
    field_row(
        label,
        text_input("", value)
            .on_input(move |s| Message::UpdateMetadata(field, s))
            .on_submit(Message::CommitMetadata)
            .padding(6)
            .size(14)
            .width(Length::Fill)
            .style(theme::rounded_input),
    )
}

fn job_finished(result: Result<(), String>) -> JobEvent {
    JobEvent::Finished(result.map_or_else(JobOutcome::Failed, |()| JobOutcome::Completed))
}

