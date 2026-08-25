use std::fmt;
use std::path::PathBuf;

use iced::alignment::Horizontal;
use iced::widget::{column, container, pick_list, row};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use kore::domains::settings::Settings;
use kore::domains::utilities::animation as builder;

use crate::app::state::{AnimState, AppState};
use crate::app::theme;
use crate::common::dialog;
use crate::systems::animation as viewer;
use crate::widget::popup;

use super::picker;

const PANEL_PADDING: f32 = 12.0;
const ROW_GAP: f32 = 8.0;

#[derive(Clone, PartialEq, Eq)]
pub struct Track {
    index: usize,
    label: String,
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

impl fmt::Debug for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PickPng,
    PickImgcut,
    PickMamodel,
    PngPicked(Option<PathBuf>),
    ImgcutPicked(Option<PathBuf>),
    MamodelPicked(Option<PathBuf>),
    AddAnims,
    AnimsPicked(Vec<PathBuf>),
    TrackSelected(Track),
    RemoveAnim,
    Viewer(viewer::Message),
    Tick,
}

pub struct State {
    png: Option<PathBuf>,
    imgcut: Option<PathBuf>,
    mamodel: Option<PathBuf>,
    anims: Vec<PathBuf>,
    tracks: Vec<Track>,
    track_width: f32,
    selected: Option<Track>,
    viewer: viewer::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            png: None,
            imgcut: None,
            mamodel: None,
            anims: Vec::new(),
            tracks: Vec::new(),
            track_width: picker::BUTTON_WIDTH,
            selected: None,
            viewer: viewer::State::with_popup(popup::Kind::UtilityAnimationExport),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState) -> Task<Message> {
        match message {
            Message::PickPng => Task::perform(dialog::file("PNG Image", &["png"]), Message::PngPicked),
            Message::PickImgcut => {
                Task::perform(dialog::file("Sprite Cut List", &["imgcut"]), Message::ImgcutPicked)
            }
            Message::PickMamodel => Task::perform(dialog::file("Model", &["mamodel"]), Message::MamodelPicked),
            Message::AddAnims => Task::perform(dialog::files("Animation", &["maanim"]), Message::AnimsPicked),
            Message::PngPicked(picked) => {
                self.replace(picked, |state| &mut state.png);
                Task::none()
            }
            Message::ImgcutPicked(picked) => {
                self.replace(picked, |state| &mut state.imgcut);
                Task::none()
            }
            Message::MamodelPicked(picked) => {
                self.replace(picked, |state| &mut state.mamodel);
                Task::none()
            }
            Message::AnimsPicked(picked) => {
                for path in picked {
                    if !self.anims.contains(&path) {
                        self.anims.push(path);
                    }
                }

                self.refresh_tracks();
                Task::none()
            }
            Message::TrackSelected(track) => {
                self.selected = Some(track);
                Task::none()
            }
            Message::RemoveAnim => {
                if let Some(track) = self.selected.take()
                    && track.index < self.anims.len()
                {
                    self.anims.remove(track.index);
                    self.refresh_tracks();
                }
                Task::none()
            }
            Message::Tick => {
                self.sync(settings, &app_state.animation);
                self.viewer.tick();
                Task::none()
            }
            Message::Viewer(msg) => {
                self.viewer.update(msg, settings, &mut app_state.animation).map(Message::Viewer)
            }
        }
    }

    fn replace(&mut self, picked: Option<PathBuf>, field: impl Fn(&mut Self) -> &mut Option<PathBuf>) {
        let Some(path) = picked else {
            return;
        };

        *field(self) = Some(path);
        self.viewer.reset_playhead();
    }

    fn refresh_tracks(&mut self) {
        self.tracks = self
            .anims
            .iter()
            .enumerate()
            .map(|(index, path)| Track { index, label: picker::name_of(path) })
            .collect();

        self.track_width = picker::combo_width(self.tracks.iter().map(|track| &track.label));

        self.selected = None;
    }

    fn sync(&mut self, settings: &Settings, anim_state: &AnimState) {
        let (Some(png), Some(cut), Some(model)) = (&self.png, &self.imgcut, &self.mamodel) else {
            self.viewer.sync("", Default::default, settings, anim_state);
            return;
        };

        let key = builder::key(png, cut, model, &self.anims, settings.utilities.frame_count);
        let anims = &self.anims;

        let frames = settings.utilities.frame_count;

        self.viewer.sync(&key, || builder::clips(png, cut, model, anims, frames), settings, anim_state);
    }

    pub fn export_popup_visible(&self) -> bool {
        self.viewer.export_popup_open()
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.viewer.export_popup_view(window).map(|view| view.map(Message::Viewer))
    }

    pub fn expanded_view(&self, settings: &Settings, app_state: &AppState) -> Option<Element<'_, Message>> {
        self.viewer.expanded_view(settings, &app_state.animation).map(|view| view.map(Message::Viewer))
    }

    pub fn view<'a>(&'a self, settings: &Settings, app_state: &AppState) -> Element<'a, Message> {
        column![self.view_controls(), self.view_viewer(settings, app_state)]
            .spacing(ROW_GAP)
            .height(Length::Fill)
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let files = row![
            picker::slot("Add PNG", self.png.as_deref(), Message::PickPng),
            picker::slot("Add IMGCUT", self.imgcut.as_deref(), Message::PickImgcut),
            picker::slot("Add MAMODEL", self.mamodel.as_deref(), Message::PickMamodel),
        ]
        .spacing(ROW_GAP);

        let chooser = pick_list(self.tracks.as_slice(), self.selected.as_ref(), Message::TrackSelected)
            .placeholder("No MAANIM")
            .width(Length::Fixed(self.track_width))
            .padding(picker::COMBO_PADDING)
            .text_size(picker::TEXT_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        let armed = self.selected.is_some();
        let remove = picker::action("Remove MAANIM", Message::RemoveAnim)
            .on_press_maybe(armed.then_some(Message::RemoveAnim))
            .style(move |t: &Theme, status| {
                if armed { theme::danger_button(t, status) } else { theme::neutral_button(t, status) }
            });

        let tracks = row![
            picker::action("Add MAANIM", Message::AddAnims).style(theme::primary_button),
            chooser,
            remove,
        ]
        .spacing(ROW_GAP)
        .align_y(Alignment::Center);

        let body = column![centered(files), centered(tracks)].spacing(ROW_GAP);

        container(body).padding(PANEL_PADDING).into()
    }

    fn view_viewer<'a>(&'a self, settings: &Settings, app_state: &AppState) -> Element<'a, Message> {
        container(self.viewer.view(settings, &app_state.animation).map(Message::Viewer))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).width(Length::Fill).align_x(Horizontal::Center).into()
}
