use std::path::PathBuf;

use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, slider, space, stack, text, text_input
};
use iced::{Alignment, Background, Border, Color, Element, Length, Subscription, Task, Theme};
use tracing::{error, info, warn};

use core::common::region::Region;
use core::modules::addons::paths;
use core::modules::mods::export::{self, apk, bcm, pack, ExportType};
use core::modules::mods::import::{self, ModImportTab, ModPackType};
use core::modules::mods::{ModDataState, ModMetadata};
use core::modules::settings::Settings;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveModal {
    #[default]
    None,
    Import,
    Export,
    DeleteConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetadataField {
    Author,
    Version,
    Package,
    Description,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SearchChanged(String),
    SelectMod(String),
    RenameBufferChanged(String),
    CommitRename,
    ToggleModStatus(String),
    OpenFolder(String),
    UpdateMetadata(MetadataField, String),
    CommitMetadata,
    ShowModal(ActiveModal),
    HideModal,
    ConfirmDelete,

    // Import Messages
    ImportTabSelected(ModImportTab),
    ImportPackageSuffixChanged(String),
    ImportSelectArchive,
    ImportFormatSelected(ModPackType),
    ImportSelectSource,
    StartImport,

    // Export Messages
    ExportTabSelected(ExportType),
    ExportTitleChanged(String),
    ExportPackageChanged(String),
    ExportRegionSelected(Region),
    ExportSelectAppFile,
    ExportCompressionChanged(f32),
    ExportPackNameChanged(String),
    StartExport,
}

#[derive(Default)]
pub struct State {
    pub data: ModDataState,
    pub search_query: String,
    pub active_modal: ActiveModal,
    pub bcm_compression: f32,
    pub selected_archive_path: Option<PathBuf>,
}

impl State {
    pub fn new(data: ModDataState) -> Self {
        Self {
            data,
            search_query: String::new(),
            active_modal: ActiveModal::None,
            bcm_compression: bcm::BCM_COMPRESSION_DEFAULT as f32,
            selected_archive_path: None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // TODO: Rewrite `core` to handle `iced` without ticking
        iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &Settings) -> Task<Message> {
        match message {
            Message::Tick => {
                let is_import_busy = import::process_events(&mut self.data);
                let is_export_busy = export::process_events(&mut self.data);

                if is_import_busy || is_export_busy {
                    // Implicitly triggers a repaint in iced when state mutates
                }
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::SelectMod(mod_folder) => {
                self.data.selected_mod = Some(mod_folder.clone());
                self.data.rename_buffer = mod_folder;
                self.populate_export_metadata();
                Task::none()
            }
            Message::RenameBufferChanged(new_name) => {
                self.data.rename_buffer = new_name;
                Task::none()
            }
            Message::CommitRename => {
                let Some(mod_idx) = self.get_selected_mod_idx() else { return Task::none(); };
                let old_name = self.data.loaded_mods[mod_idx].folder_name.clone();
                let new_name = self.data.rename_buffer.clone();

                if new_name.is_empty() || new_name == old_name {
                    self.data.rename_buffer = old_name;
                    return Task::none();
                }

                let old_path = std::path::Path::new("mods").join(&old_name);
                let new_path = std::path::Path::new("mods").join(&new_name);

                if !new_path.exists() && old_path.exists() && std::fs::rename(&old_path, &new_path).is_ok() {
                    if self.data.loaded_mods[mod_idx].enabled {
                        core::common::resolver::set_active_mod(Some(new_name.clone()));
                    }
                    self.data.loaded_mods[mod_idx].folder_name = new_name.clone();
                    self.data.selected_mod = Some(new_name.clone());
                    self.data.loaded_mods[mod_idx].metadata.title = new_name.clone();

                    if let Err(e) = self.data.loaded_mods[mod_idx].metadata.save(&new_path) {
                        warn!("Failed to save renamed metadata: {}", e);
                    }
                } else {
                    self.data.rename_buffer = old_name;
                }
                Task::none()
            }
            Message::ToggleModStatus(mod_folder) => {
                let Some(mod_idx) = self.data.loaded_mods.iter().position(|m| m.folder_name == mod_folder) else { return Task::none(); };
                let is_currently_enabled = self.data.loaded_mods[mod_idx].enabled;

                for m in self.data.loaded_mods.iter_mut() {
                    m.enabled = false;
                }

                if !is_currently_enabled {
                    if let Some(m) = self.data.loaded_mods.iter_mut().find(|m| m.folder_name == mod_folder) {
                        m.enabled = true;
                    }
                    core::common::resolver::set_active_mod(Some(mod_folder));
                } else {
                    core::common::resolver::set_active_mod(None);
                }

                self.data.needs_rescan = true;
                Task::none()
            }
            Message::OpenFolder(mod_folder) => {
                let mod_path = std::path::Path::new("mods").join(&mod_folder);
                let _ = open::that(mod_path);
                Task::none()
            }
            Message::UpdateMetadata(field, value) => {
                let Some(mod_idx) = self.get_selected_mod_idx() else { return Task::none(); };
                let meta = &mut self.data.loaded_mods[mod_idx].metadata;

                match field {
                    MetadataField::Author => meta.author = value,
                    MetadataField::Version => meta.version = value,
                    MetadataField::Package => meta.package = value,
                    MetadataField::Description => meta.description = value,
                }
                Task::none()
            }
            Message::CommitMetadata => {
                let Some(mod_idx) = self.get_selected_mod_idx() else { return Task::none(); };
                let mod_folder = self.data.loaded_mods[mod_idx].folder_name.clone();
                let mod_path = std::path::Path::new("mods").join(&mod_folder);

                self.data.loaded_mods[mod_idx].metadata.title = mod_folder;
                if let Err(e) = self.data.loaded_mods[mod_idx].metadata.save(&mod_path) {
                    error!("Failed to commit metadata: {}", e);
                }
                Task::none()
            }
            Message::ShowModal(modal) => {
                self.active_modal = modal;
                Task::none()
            }
            Message::HideModal => {
                self.active_modal = ActiveModal::None;
                Task::none()
            }
            Message::ConfirmDelete => {
                let Some(mod_folder) = self.data.selected_mod.clone() else { return Task::none(); };
                let path = std::path::Path::new("mods").join(&mod_folder);

                import::delete_mod_folder(path);
                self.data.selected_mod = None;
                self.data.needs_rescan = true;
                self.active_modal = ActiveModal::None;
                Task::none()
            }
            Message::ImportTabSelected(tab) => {
                self.data.import.tab = tab;
                Task::none()
            }
            Message::ImportPackageSuffixChanged(suffix) => {
                self.data.import.package_suffix = suffix;
                Task::none()
            }
            Message::ImportSelectArchive => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Archive", &["bcm", "zip"]).pick_file() {
                    self.selected_archive_path = Some(path);
                }
                Task::none()
            }
            Message::ImportFormatSelected(format) => {
                self.data.import.pack_type = format;
                self.selected_archive_path = None;
                Task::none()
            }
            Message::ImportSelectSource => {
                let path = match self.data.import.pack_type {
                    ModPackType::Apk => rfd::FileDialog::new().add_filter("APK", &["apk", "xapk", "apkm"]).pick_file(),
                    ModPackType::Pack => {
                        if let Some(files) = rfd::FileDialog::new().add_filter("Pack/List", &["pack", "list"]).pick_files() {
                            files.first().map(|f| {
                                let parent = f.parent().unwrap();
                                let stem = f.file_stem().unwrap().to_string_lossy();
                                parent.join(format!("{}.pack", stem))
                            })
                        } else {
                            None
                        }
                    }
                };

                if let Some(p) = path {
                    self.selected_archive_path = Some(p);
                }
                Task::none()
            }
            Message::StartImport => {
                if self.data.import.is_busy { return Task::none(); }

                match self.data.import.tab {
                    ModImportTab::Adb => import::start_adb_import(&mut self.data),
                    ModImportTab::Bcm => {
                        if let Some(path) = self.selected_archive_path.clone() {
                            import::start_bcm_import(&mut self.data, path);
                        }
                    }
                    ModImportTab::Pack => {
                        if let Some(path) = self.selected_archive_path.clone() {
                            import::start_pack_import(&mut self.data, path);
                        }
                    }
                }
                Task::none()
            }
            Message::ExportTabSelected(tab) => {
                self.data.export.tab = tab;
                Task::none()
            }
            Message::ExportTitleChanged(title) => {
                self.data.export.app_title = title;
                Task::none()
            }
            Message::ExportPackageChanged(pkg) => {
                self.data.export.package_suffix = pkg;
                Task::none()
            }
            Message::ExportRegionSelected(region) => {
                self.data.export.target_region = region;
                Task::none()
            }
            Message::ExportSelectAppFile => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Android App", &["apk", "xapk", "apkm", "apks"]).pick_file() {
                    self.data.export.selected_apk = Some(path);
                }
                Task::none()
            }
            Message::ExportCompressionChanged(val) => {
                self.bcm_compression = val;
                Task::none()
            }
            Message::ExportPackNameChanged(name) => {
                self.data.export.pack_name = name;
                Task::none()
            }
            Message::StartExport => {
                if self.data.export.is_busy || self.data.selected_mod.is_none() { return Task::none(); }

                match self.data.export.tab {
                    ExportType::Apk => apk::start_export(&mut self.data, settings),
                    ExportType::Bcm => bcm::start_bcm_export(&mut self.data, self.bcm_compression as i64),
                    ExportType::Pack => {
                        if self.data.export.pack_name.is_empty() {
                            self.data.export.pack_name = "DownloadLocal".to_string();
                        }
                        pack::start_pack_export(&mut self.data);
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let content = row![
            self.view_sidebar(),
            self.view_details()
        ]
            .width(Length::Fill)
            .height(Length::Fill);

        if self.active_modal != ActiveModal::None {
            stack![
                content,
                container(self.view_active_modal())
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
            ].into()
        } else {
            content.into()
        }
    }

    fn view_sidebar(&self) -> Element<Message> {
        let search_input = text_input("Search Mods...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(8);

        let import_btn = button(text("Import Mod").align_x(Alignment::Center))
            .width(Length::Fill)
            .on_press(Message::ShowModal(ActiveModal::Import));

        let mut mod_list = column![].spacing(4).width(Length::Fill);
        let query = self.search_query.to_lowercase();

        for mod_data in &self.data.loaded_mods {
            if !query.is_empty() && !mod_data.folder_name.to_lowercase().contains(&query) {
                continue;
            }

            let is_selected = self.data.selected_mod.as_deref() == Some(mod_data.folder_name.as_str());
            let is_enabled = mod_data.enabled;
            let folder_name = mod_data.folder_name.clone();

            let icon_path = std::path::Path::new("mods").join(&folder_name).join("icons").join("icon.png");

            let row_content = if icon_path.exists() {
                row![
                    image(icon_path).width(Length::Fixed(46.0)).height(Length::Fixed(46.0)),
                    text(folder_name.clone()).size(14)
                ].spacing(12).align_y(Alignment::Center)
            } else {
                row![text(folder_name.clone()).size(14)].align_y(Alignment::Center)
            };

            let item_btn = button(row_content)
                .width(Length::Fill)
                .padding(8)
                .on_press(Message::SelectMod(folder_name.clone()))
                .style(move |theme: &Theme, status| {
                    let palette = theme.palette();
                    let bg = if is_selected {
                        palette.primary
                    } else if is_enabled {
                        palette.success
                    } else if status == button::Status::Hovered {
                        Color { a: 0.1, ..palette.text }
                    } else {
                        Color::TRANSPARENT
                    };

                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: if is_selected || is_enabled { Color::WHITE } else { palette.text },
                        border: Border::default().rounded(4.0).width(if is_selected { 2.0 } else { 0.0 }).color(palette.primary),
                        ..Default::default()
                    }
                });

            mod_list = mod_list.push(item_btn);
        }

        container(
            column![
                search_input,
                import_btn,
                space().height(4),
                scrollable(mod_list).height(Length::Fill)
            ]
                .spacing(8)
        )
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .padding(8)
            .into()
    }

    fn view_details(&self) -> Element<Message> {
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
            .style(move |theme: &Theme, _status| {
                let palette = theme.palette();
                button::Style {
                    background: Some(Background::Color(if is_enabled { palette.danger } else { palette.success })),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(4.0),
                    ..Default::default()
                }
            });

        let open_btn = button(text("Open Folder").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::OpenFolder(mod_folder.clone()))
            .style(primary_button_style);

        let export_btn = button(text("Export Mod").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::ShowModal(ActiveModal::Export))
            .style(primary_button_style);

        let delete_btn = button(text("Delete Mod").align_x(Alignment::Center))
            .width(Length::Fixed(135.0))
            .on_press(Message::ShowModal(ActiveModal::DeleteConfirm))
            .style(danger_button_style);

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

    fn view_active_modal(&self) -> Element<Message> {
        match self.active_modal {
            ActiveModal::None => space().into(),
            ActiveModal::DeleteConfirm => self.view_delete_modal(),
            ActiveModal::Import => self.view_import_modal(),
            ActiveModal::Export => self.view_export_modal(),
        }
    }

    fn view_delete_modal(&self) -> Element<Message> {
        let title_str = format!("Are you sure you want to completely delete {}?", self.data.selected_mod.as_deref().unwrap_or("this mod"));
        let title = text(title_str).size(16);

        let yes_btn = button(text("Yes").align_x(Alignment::Center))
            .width(Length::Fixed(80.0))
            .on_press(Message::ConfirmDelete)
            .style(danger_button_style);

        let no_btn = button(text("No").align_x(Alignment::Center))
            .width(Length::Fixed(80.0))
            .on_press(Message::HideModal)
            .style(primary_button_style);

        self.modal_container("Confirm Deletion", column![title, row![yes_btn, no_btn].spacing(16)])
    }

    fn view_import_modal(&self) -> Element<Message> {
        let is_busy = self.data.import.is_busy;

        let tabs_row = row![
            self.tab_button("Android", self.data.import.tab == ModImportTab::Adb, Message::ImportTabSelected(ModImportTab::Adb)),
            self.tab_button("BCM", self.data.import.tab == ModImportTab::Bcm, Message::ImportTabSelected(ModImportTab::Bcm)),
            self.tab_button("Pack", self.data.import.tab == ModImportTab::Pack, Message::ImportTabSelected(ModImportTab::Pack)),
        ].spacing(8);

        let content: Element<Message> = match self.data.import.tab {
            ModImportTab::Adb => {
                let has_adb = paths::adb_status() == paths::Presence::Installed;
                column![
                    if has_adb { text("Import mod package using Android/Emulator") } else { text("Android Bridge is required. Download it in Settings > Add-Ons").color(Color::from_rgb(0.8, 0.6, 0.2)) },
                    row![
                        text("Package:"),
                        text_input("en", &self.data.import.package_suffix).on_input(Message::ImportPackageSuffixChanged).width(Length::Fixed(60.0))
                    ].align_y(Alignment::Center),
                    button(text(if has_adb { "Start Import" } else { "ADB Missing" }))
                        .on_press_maybe(if has_adb && !is_busy { Some(Message::StartImport) } else { None })
                        .style(primary_button_style)
                ].spacing(12).into()
            },
            ModImportTab::Bcm => {
                column![
                    text("Import packaged .bcm or .zip mod archives"),
                    row![
                        button("Select Archive").on_press_maybe(if !is_busy { Some(Message::ImportSelectArchive) } else { None }),
                        text(self.selected_archive_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "No archive selected".to_string()))
                    ].align_y(Alignment::Center).spacing(8),
                    button("Start Import").on_press_maybe(if !is_busy && self.selected_archive_path.is_some() { Some(Message::StartImport) } else { None }).style(primary_button_style)
                ].spacing(12).into()
            },
            ModImportTab::Pack => {
                let pack_types = vec!["APK".to_string(), "Pack".to_string()];
                let selected_pack_type = match self.data.import.pack_type {
                    ModPackType::Apk => "APK".to_string(),
                    ModPackType::Pack => "Pack".to_string(),
                };

                column![
                    text("Import modded files directly from game formats"),
                    row![
                        text("Format:"),
                        pick_list(
                            pack_types,
                            Some(selected_pack_type),
                            |s| Message::ImportFormatSelected(if s == "APK" { ModPackType::Apk } else { ModPackType::Pack })
                        )
                    ].align_y(Alignment::Center).spacing(8),
                    row![
                        button(if self.data.import.pack_type == ModPackType::Pack { "Select Pack/List" } else { "Select Source" })
                            .on_press_maybe(if !is_busy { Some(Message::ImportSelectSource) } else { None }),
                        text(self.selected_archive_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "No source selected".to_string()))
                    ].align_y(Alignment::Center).spacing(8),
                    button("Start Import").on_press_maybe(if !is_busy && self.selected_archive_path.is_some() { Some(Message::StartImport) } else { None }).style(primary_button_style)
                ].spacing(12).into()
            }
        };

        let log_display = scrollable(text(self.data.import.log_content.clone()).size(12)).height(Length::Fixed(150.0));
        let body = column![tabs_row, space().height(10), content, space().height(10), text(self.data.import.status_message.clone()), log_display].spacing(8);
        self.modal_container("Import Mod", body)
    }

    fn view_export_modal(&self) -> Element<Message> {
        let is_busy = self.data.export.is_busy;
        let is_ready = self.data.selected_mod.is_some();

        let tabs_row = row![
            self.tab_button("APK", self.data.export.tab == ExportType::Apk, Message::ExportTabSelected(ExportType::Apk)),
            self.tab_button("BCM", self.data.export.tab == ExportType::Bcm, Message::ExportTabSelected(ExportType::Bcm)),
            self.tab_button("Pack", self.data.export.tab == ExportType::Pack, Message::ExportTabSelected(ExportType::Pack)),
        ].spacing(8);

        let regions = vec!["English".to_string(), "Japanese".to_string(), "Korean".to_string(), "Taiwanese".to_string()];
        let selected_region = match self.data.export.target_region {
            Region::En => "English".to_string(),
            Region::Ja => "Japanese".to_string(),
            Region::Ko => "Korean".to_string(),
            Region::Tw => "Taiwanese".to_string(),
        };

        let region_select = pick_list(
            regions,
            Some(selected_region),
            |s| Message::ExportRegionSelected(match s.as_str() {
                "Japanese" => Region::Ja,
                "Korean" => Region::Ko,
                "Taiwanese" => Region::Tw,
                _ => Region::En,
            })
        );

        let content: Element<Message> = match self.data.export.tab {
            ExportType::Apk => {
                column![
                    text("Patch and export modded APK"),
                    row![text("Title:"), text_input("", &self.data.export.app_title).on_input(Message::ExportTitleChanged).width(Length::Fixed(150.0))].align_y(Alignment::Center).spacing(4),
                    row![text("Package:"), text_input("", &self.data.export.package_suffix).on_input(Message::ExportPackageChanged).width(Length::Fixed(60.0))].align_y(Alignment::Center).spacing(4),
                    row![text("Region:"), region_select].align_y(Alignment::Center).spacing(4),
                    row![
                        button("Select App File").on_press_maybe(if !is_busy { Some(Message::ExportSelectAppFile) } else { None }),
                        text(self.data.export.selected_apk.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_else(|| "No file selected".to_string()))
                    ].align_y(Alignment::Center).spacing(8),
                    button("Apply Mod").on_press_maybe(if !is_busy && is_ready && self.data.export.selected_apk.is_some() { Some(Message::StartExport) } else { None }).style(primary_button_style)
                ].spacing(12).into()
            },
            ExportType::Bcm => {
                column![
                    text("Package mod into a standalone .bcm archive"),
                    row![text("Title:"), text_input("", &self.data.export.app_title).on_input(Message::ExportTitleChanged).width(Length::Fixed(150.0))].align_y(Alignment::Center).spacing(4),
                    row![text("Compression:"), slider(bcm::BCM_COMPRESSION_MIN as f32..=bcm::BCM_COMPRESSION_MAX as f32, self.bcm_compression, Message::ExportCompressionChanged).width(Length::Fixed(150.0))].align_y(Alignment::Center).spacing(4),
                    button("Create BCM Package").on_press_maybe(if !is_busy && is_ready { Some(Message::StartExport) } else { None }).style(primary_button_style)
                ].spacing(12).into()
            },
            ExportType::Pack => {
                column![
                    text("Compile mod files into raw .pack and .list files"),
                    row![text("Name:"), text_input("DownloadLocal", &self.data.export.pack_name).on_input(Message::ExportPackNameChanged).width(Length::Fixed(150.0))].align_y(Alignment::Center).spacing(4),
                    row![text("Key:"), region_select].align_y(Alignment::Center).spacing(4),
                    button("Create Pack").on_press_maybe(if !is_busy && is_ready { Some(Message::StartExport) } else { None }).style(primary_button_style)
                ].spacing(12).into()
            },
        };

        let log_status = self.data.export.log_content.lines().last().unwrap_or("Ready").to_string();
        let log_display = scrollable(text(self.data.export.log_content.clone()).size(12)).height(Length::Fixed(150.0));

        let body = column![tabs_row, space().height(10), content, space().height(10), text(log_status), log_display].spacing(8);
        self.modal_container("Export Mod", body)
    }

    // Helper functions for reducing code duplication in the view
    fn modal_container<'a>(&self, title: &str, content: iced::widget::Column<'a, Message>) -> Element<'a, Message> {
        let header = row![
            text(title.to_string()).size(20),
            space().width(Length::Fill),
            button(text("X")).on_press(Message::HideModal).style(danger_button_style)
        ].align_y(Alignment::Center);

        container(
            scrollable(
                column![header, space().height(16), content].spacing(8)
            )
        )
            .width(Length::Fixed(500.0))
            .height(Length::Shrink)
            .padding(20)
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(Background::Color(palette.background)),
                    border: Border::default().rounded(8.0).width(1.0).color(palette.text),
                    ..Default::default()
                }
            })
            .into()
    }

    fn tab_button(&self, label: &str, is_active: bool, msg: Message) -> iced::widget::Button<'_, Message> {
        button(text(label.to_string()).align_x(Alignment::Center))
            .width(Length::Fixed(80.0))
            .on_press(msg)
            .style(move |theme: &Theme, _status| {
                let palette = theme.palette();
                button::Style {
                    background: Some(Background::Color(if is_active { palette.primary } else { Color { a: 0.2, ..palette.text } })),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(4.0),
                    ..Default::default()
                }
            })
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
}

// Styling closures for buttons
fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let bg = if status == button::Status::Hovered {
        Color { a: 0.8, ..palette.primary }
    } else {
        palette.primary
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border::default().rounded(4.0),
        ..Default::default()
    }
}

fn danger_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let bg = if status == button::Status::Hovered {
        Color { a: 0.8, ..palette.danger }
    } else {
        palette.danger
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border::default().rounded(4.0),
        ..Default::default()
    }
}