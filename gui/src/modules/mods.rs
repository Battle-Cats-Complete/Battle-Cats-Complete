mod export;
mod import;
mod list;

use std::fs;
use std::path::Path;
use std::time::Duration;

use iced::widget::{
    button, column, container, row, scrollable, space, stack, text, text_input,
};
use iced::{Alignment, Background, Color, Element, Length, Size, Subscription, Task, Theme};
use tracing::warn;

use core::common::job::{JobEvent, JobOutcome};
use core::modules::mods::ModDataState;
use core::modules::settings::Settings;

use crate::app::theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetadataField {
    Author,
    Version,
    Package,
    Description,
}

#[derive(Debug, Clone)]
pub enum Message {
    SpinnerTick,
    List(list::Message),
    Import(import::Message),
    Export(export::Message),

    SearchChanged(String),
    RenameBufferChanged(String),
    CommitRename,
    ToggleModStatus(String),
    OpenFolder(String),
    UpdateMetadata(MetadataField, String),
    CommitMetadata,
    ShowDeleteConfirm,
    HideDeleteConfirm,
    ConfirmDelete,
    DeleteFinished(Result<(), String>),
}

pub struct State {
    data: ModDataState,
    list: list::State,
    import: import::State,
    export: export::State,
    delete_confirm_open: bool,
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
            delete_confirm_open: false,
        }
    }

    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub fn active_mod(&self) -> Option<String> {
        self.data.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.import.is_running() || self.export.is_running() {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::SpinnerTick)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        let task = self.update_inner(message, settings);

        self.list.refresh(&self.data.loaded_mods, &self.data.search_query);

        task
    }

    fn update_inner(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::SpinnerTick => {
                self.import.advance_spinner();
                self.export.advance_spinner();
                Task::none()
            }
            Message::List(msg) => {
                if let list::Message::SelectMod(folder) = &msg {
                    self.select_mod(folder.clone());
                }
                self.list.update(msg);
                Task::none()
            }
            Message::Import(msg) => self.import.update(msg, &mut self.data, settings).map(Message::Import),
            Message::Export(msg) => self.export.update(msg, &mut self.data, settings).map(Message::Export),

            Message::SearchChanged(query) => {
                self.data.search_query = query;
                Task::none()
            }
            Message::RenameBufferChanged(new_name) => {
                self.data.rename_buffer = new_name;
                Task::none()
            }
            Message::CommitRename => {
                self.commit_rename();
                Task::none()
            }
            Message::ToggleModStatus(mod_folder) => {
                self.toggle_mod_status(mod_folder);
                Task::none()
            }
            Message::OpenFolder(mod_folder) => {
                let mod_path = Path::new("mods").join(&mod_folder);
                let _ = open::that(mod_path);
                Task::none()
            }
            Message::UpdateMetadata(field, value) => {
                self.update_metadata(field, value);
                Task::none()
            }
            Message::CommitMetadata => {
                self.commit_metadata();
                Task::none()
            }
            Message::ShowDeleteConfirm => {
                self.delete_confirm_open = true;
                Task::none()
            }
            Message::HideDeleteConfirm => {
                self.delete_confirm_open = false;
                Task::none()
            }
            Message::ConfirmDelete => self.confirm_delete(),
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
        self.data.selected_mod = Some(folder.clone());
        self.data.rename_buffer = folder;
        self.populate_export_metadata();
    }

    fn toggle_mod_status(&mut self, mod_folder: String) {
        let Some(idx) = self.data.loaded_mods.iter().position(|m| m.folder_name == mod_folder) else { return; };
        let is_currently_enabled = self.data.loaded_mods[idx].enabled;

        for m in self.data.loaded_mods.iter_mut() {
            m.enabled = false;
        }

        if !is_currently_enabled {
            self.data.loaded_mods[idx].enabled = true;
            core::common::resolver::set_active_mod(Some(mod_folder));
        } else {
            core::common::resolver::set_active_mod(None);
        }

        self.data.refresh_mods();
    }

    fn commit_rename(&mut self) {
        let Some(idx) = self.get_selected_mod_idx() else { return; };
        let old_name = self.data.loaded_mods[idx].folder_name.clone();
        let new_name = self.data.rename_buffer.clone();

        if new_name.is_empty() || new_name == old_name {
            self.data.rename_buffer = old_name;
            return;
        }

        let old_path = Path::new("mods").join(&old_name);
        let new_path = Path::new("mods").join(&new_name);

        if !new_path.exists() && old_path.exists() && std::fs::rename(&old_path, &new_path).is_ok() {
            if self.data.loaded_mods[idx].enabled {
                core::common::resolver::set_active_mod(Some(new_name.clone()));
            }
            self.data.loaded_mods[idx].folder_name = new_name.clone();
            self.data.selected_mod = Some(new_name.clone());
            self.data.loaded_mods[idx].metadata.title = new_name.clone();

            if let Err(e) = self.data.loaded_mods[idx].metadata.save(&new_path) {
                warn!("Failed to save renamed metadata: {}", e);
            }
        } else {
            self.data.rename_buffer = old_name;
        }
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

    fn confirm_delete(&mut self) -> Task<Message> {
        let Some(mod_folder) = self.data.selected_mod.clone() else { return Task::none(); };
        let path = Path::new("mods").join(&mod_folder);

        self.data.selected_mod = None;
        self.delete_confirm_open = false;

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
        let content = row![
            self.view_sidebar(),
            self.view_details()
        ]
            .width(Length::Fill)
            .height(Length::Fill);

        let modal: Option<Element<'_, Message>> = if self.delete_confirm_open {
            Some(self.view_delete_modal())
        } else {
            None
        };

        match modal {
            Some(modal_element) => stack![
                content,
                container(modal_element)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|theme: &Theme| {
                        let palette = theme.palette();
                        container::Style {
                            background: Some(Background::Color(Color { a: 0.8, ..palette.background })),
                            ..Default::default()
                        }
                    })
            ].into(),
            None => content.into(),
        }
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
        let search_input = text_input("Search Mods...", &self.data.search_query)
            .on_input(Message::SearchChanged)
            .padding(8);

        let import_btn = button(text("Import Mod").align_x(Alignment::Center))
            .width(Length::Fill)
            .on_press(Message::Import(import::Message::Open));

        let mod_list = self.list.view(&self.data.loaded_mods, self.data.selected_mod.as_deref()).map(Message::List);

        container(
            column![
                search_input,
                import_btn,
                space().height(4),
                mod_list
            ]
                .spacing(8)
        )
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .padding(8)
            .into()
    }

    fn view_details(&self) -> Element<'_, Message> {
        let Some(mod_idx) = self.get_selected_mod_idx() else {
            return container(text("Please select or import a Mod").size(18))
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
            .size(25)
            .padding(10);

        let toggle_btn = button(
            text(if is_enabled { "Disable Mod" } else { "Enable Mod" }).align_x(Alignment::Center)
        )
            .width(Length::Fixed(135.0))
            .on_press(Message::ToggleModStatus(mod_folder.clone()))
            .style(move |theme: &Theme, _status| theme::solid_button(if is_enabled { theme.palette().danger } else { theme.palette().success }));

        let open_btn = button(text("Open Folder").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::OpenFolder(mod_folder.clone()))
            .style(theme::primary_button);

        let export_btn = button(text("Export Mod").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::Export(export::Message::Open))
            .style(theme::primary_button);

        let delete_btn = button(text("Delete Mod").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::ShowDeleteConfirm)
            .style(theme::danger_button);

        let actions_row = row![toggle_btn, open_btn, export_btn, delete_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        let meta_col = column![
            text("Information").size(18),
            row![
                text("Author:").width(Length::Fixed(80.0)),
                text_input("", &mod_data.metadata.author)
                    .on_input(|s| Message::UpdateMetadata(MetadataField::Author, s))
                    .on_submit(Message::CommitMetadata)
                    .width(Length::Fixed(200.0))
            ].align_y(Alignment::Center),
            row![
                text("Version:").width(Length::Fixed(80.0)),
                text_input("", &mod_data.metadata.version)
                    .on_input(|s| Message::UpdateMetadata(MetadataField::Version, s))
                    .on_submit(Message::CommitMetadata)
                    .width(Length::Fixed(200.0))
            ].align_y(Alignment::Center),
            row![
                text("Package:").width(Length::Fixed(80.0)),
                text_input("", &mod_data.metadata.package)
                    .on_input(|s| Message::UpdateMetadata(MetadataField::Package, s))
                    .on_submit(Message::CommitMetadata)
                    .width(Length::Fixed(200.0))
            ].align_y(Alignment::Center),
        ].spacing(12);

        let desc_col = column![
            text("Description").size(18),
            text_input("Enter mod description here...", &mod_data.metadata.description)
                .on_input(|s| Message::UpdateMetadata(MetadataField::Description, s))
                .on_submit(Message::CommitMetadata)
                .width(Length::Fill)
        ].spacing(8);

        container(
            column![
                header,
                space().height(10),
                actions_row,
                space().height(20),
                meta_col,
                space().height(20),
                desc_col
            ]
                .spacing(8)
                .width(Length::Fill)
        )
            .padding(16)
            .into()
    }

    fn view_delete_modal(&self) -> Element<'_, Message> {
        let title_str = format!("Are you sure you want to completely delete {}?", self.data.selected_mod.as_deref().unwrap_or("this mod"));
        let title = text(title_str).size(16);

        let yes_btn = button(text("Yes").align_x(Alignment::Center))
            .width(Length::Fixed(80.0))
            .on_press(Message::ConfirmDelete)
            .style(theme::danger_button);

        let no_btn = button(text("No").align_x(Alignment::Center))
            .width(Length::Fixed(80.0))
            .on_press(Message::HideDeleteConfirm)
            .style(theme::primary_button);

        self.modal_container(
            "Confirm Deletion",
            Message::HideDeleteConfirm,
            column![title, row![yes_btn, no_btn].spacing(16)].into()
        )
    }
    fn modal_container<'a>(&self, title: &str, close_msg: Message, content: Element<'a, Message>) -> Element<'a, Message> {
        let header = row![
            text(title.to_string()).size(20),
            space().width(Length::Fill),
            button(text("X")).on_press(close_msg).style(theme::danger_button)
        ].align_y(Alignment::Center);

        container(
            scrollable(
                column![header, space().height(16), content].spacing(8)
            )
        )
            .width(Length::Fixed(500.0))
            .height(Length::Shrink)
            .padding(20)
            .style(theme::confirm_modal_container)
            .into()
    }
}

fn job_finished(result: Result<(), String>) -> JobEvent {
    JobEvent::Finished(match result {
        Ok(()) => JobOutcome::Completed,
        Err(message) => JobOutcome::Failed(message),
    })
}

