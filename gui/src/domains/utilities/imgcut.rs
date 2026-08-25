use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, pick_list, row};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};
use image::ImageFormat;

use kore::domains::utilities::imgcut::{self, Sheet};

use crate::app::theme;
use crate::common::feedback::{Slot, FAILURE_LABEL};
use crate::widget::picture;

use super::picker;

const PANEL_PADDING: f32 = 12.0;
const ROW_GAP: f32 = 8.0;
const BUTTON_RADIUS: f32 = 4.0;

const EMPTY_LABEL: &str = "Add a PNG and an IMGCUT to begin";
const LOADING_LABEL: &str = "Reading sprite sheet...";
const SHEET_LABEL: &str = "Spritesheet";

const EXPORT_IDLE: &str = "Export Image";
const EXPORT_BUSY: &str = "Exporting...";
const EXPORT_DONE: &str = "Exported!";

#[derive(Clone, PartialEq, Eq, Default)]
pub enum Choice {
    #[default]
    Sheet,
    Cut(usize, String),
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Choice::Sheet => f.write_str(SHEET_LABEL),
            Choice::Cut(_, label) => f.write_str(label),
        }
    }
}

impl fmt::Debug for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct Loaded {
    sheet: Sheet,
    full: picture::Source,
    choices: Vec<Choice>,
    outlines: Vec<picture::Outline>,
    combo_width: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    PickPng,
    PickImgcut,
    PngPicked(Option<PathBuf>),
    ImgcutPicked(Option<PathBuf>),
    Loaded(Result<Arc<Loaded>, String>),
    ChoiceSelected(Choice),
    Picture(picture::Message),
    Export,
    Exported(bool),
    ExportExpired,
}

impl fmt::Debug for Loaded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Loaded").field("cuts", &self.choices.len()).finish()
    }
}

#[derive(Default)]
pub struct State {
    png: Option<PathBuf>,
    imgcut: Option<PathBuf>,
    loaded: Option<Arc<Loaded>>,
    cropped: Option<picture::Source>,
    choice: Choice,
    picture: picture::State,
    reading: bool,
    exporting: bool,
    export_feedback: Slot<bool>,
    notice: Option<String>,
}

impl State {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickPng => Task::perform(picker::ask("PNG Image", &["png"]), Message::PngPicked),
            Message::PickImgcut => {
                Task::perform(picker::ask("Sprite Cut List", &["imgcut"]), Message::ImgcutPicked)
            }
            Message::PngPicked(Some(path)) => {
                if self.imgcut.is_none() {
                    self.imgcut = imgcut::partner(&path, "imgcut");
                }

                self.png = Some(path);
                self.read()
            }
            Message::ImgcutPicked(Some(path)) => {
                if self.png.is_none() {
                    self.png = imgcut::partner(&path, "png");
                }

                self.imgcut = Some(path);
                self.read()
            }
            Message::PngPicked(None) | Message::ImgcutPicked(None) => Task::none(),
            Message::Loaded(Ok(loaded)) => {
                self.reading = false;
                self.choice = Choice::Sheet;
                self.cropped = None;
                self.notice = None;
                self.picture.reset();
                self.loaded = Some(loaded);
                Task::none()
            }
            Message::Loaded(Err(problem)) => {
                self.reading = false;
                self.loaded = None;
                self.cropped = None;
                self.notice = Some(problem);
                Task::none()
            }
            Message::ChoiceSelected(choice) => {
                self.choice = choice;
                self.picture.reset();
                self.refresh_crop();
                Task::none()
            }
            Message::Picture(msg) => {
                self.picture.update(msg);
                Task::none()
            }
            Message::Export => self.export(),
            Message::Exported(ok) => {
                self.exporting = false;
                self.export_feedback.set(ok, Message::ExportExpired)
            }
            Message::ExportExpired => {
                self.export_feedback.expire();
                Task::none()
            }
        }
    }

    fn read(&mut self) -> Task<Message> {
        let (Some(png), Some(cut)) = (self.png.clone(), self.imgcut.clone()) else {
            return Task::none();
        };

        self.reading = true;
        self.notice = None;

        Task::perform(smol::unblock(move || assemble(&png, &cut)), Message::Loaded)
    }

    fn refresh_crop(&mut self) {
        let Choice::Cut(index, _) = self.choice else {
            self.cropped = None;
            return;
        };

        let Some(loaded) = &self.loaded else {
            return;
        };

        let Some(image) = loaded.sheet.crop(index) else {
            self.cropped = None;
            self.notice = Some("That cut has no pixels to show".to_string());
            return;
        };

        let (width, height) = (image.width(), image.height());
        let mut encoded = Vec::new();

        self.cropped = match image.write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png) {
            Ok(()) => Some(picture::Source::new(encoded, width, height)),
            Err(problem) => {
                self.notice = Some(format!("Could not render that cut: {}", problem));
                None
            }
        };
    }

    fn export(&mut self) -> Task<Message> {
        let Some(loaded) = self.loaded.clone() else {
            return Task::none();
        };

        if self.exporting {
            return Task::none();
        }

        self.exporting = true;
        let choice = self.choice.clone();

        Task::perform(
            smol::unblock(move || {
                let outcome = match choice {
                    Choice::Sheet => imgcut::export_all(&loaded.sheet),
                    Choice::Cut(index, _) => imgcut::export_cut(&loaded.sheet, index),
                };

                outcome.is_ok()
            }),
            Message::Exported,
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![self.view_controls(), self.view_canvas()]
            .spacing(ROW_GAP)
            .height(Length::Fill)
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let files = row![
            picker::slot("Add PNG", self.png.as_deref(), Message::PickPng),
            picker::slot("Add IMGCUT", self.imgcut.as_deref(), Message::PickImgcut),
        ]
        .spacing(ROW_GAP);

        let empty: &[Choice] = &[];
        let (choices, width) = self
            .loaded
            .as_deref()
            .map_or((empty, picker::BUTTON_WIDTH), |loaded| (loaded.choices.as_slice(), loaded.combo_width));

        let chooser = pick_list(choices, Some(&self.choice), Message::ChoiceSelected)
            .width(Length::Fixed(width))
            .padding(picker::COMBO_PADDING)
            .text_size(picker::TEXT_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        let actions = row![chooser, self.view_export()].spacing(ROW_GAP).align_y(Alignment::Center);

        let mut body = column![centered(files), centered(actions)].spacing(ROW_GAP);

        if let Some(notice) = &self.notice {
            body = body.push(centered(theme::centered_text(notice.as_str()).size(picker::TEXT_SIZE - 1.0)));
        }

        container(body).padding(PANEL_PADDING).into()
    }

    fn view_export(&self) -> Element<'_, Message> {
        let busy = self.exporting;
        let feedback = self.export_feedback.get().copied();
        let ready = self.loaded.is_some() && !busy;
        let lit = busy || feedback.is_some() || ready;

        let label = if busy {
            EXPORT_BUSY.to_string()
        } else {
            feedback.map_or_else(
                || EXPORT_IDLE.to_string(),
                |ok| if ok { EXPORT_DONE.to_string() } else { FAILURE_LABEL.to_string() },
            )
        };

        picker::action(label, Message::Export)
            .on_press_maybe(ready.then_some(Message::Export))
            .style(move |t: &Theme, status| {
                if !lit {
                    return theme::neutral_button(t, status);
                }

                button::Style {
                    background: Some(Background::Color(feedback_color(t, busy, feedback))),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(BUTTON_RADIUS),
                    ..button::Style::default()
                }
            })
            .into()
    }

    fn view_canvas(&self) -> Element<'_, Message> {
        let canvas: Element<'_, Message> = match (&self.choice, &self.cropped, self.loaded.as_deref()) {
            (Choice::Cut(_, _), Some(cropped), _) => self.picture.view(cropped).map(Message::Picture),
            (Choice::Sheet, _, Some(loaded)) => {
                self.picture.view_outlined(&loaded.full, &loaded.outlines).map(Message::Picture)
            }
            _ => {
                let label = if self.reading { LOADING_LABEL } else { EMPTY_LABEL };

                theme::centered_text(label).width(Length::Fill).height(Length::Fill).into()
            }
        };

        container(canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(PANEL_PADDING)
            .style(theme::workspace_container)
            .into()
    }
}

fn assemble(png: &Path, cut: &Path) -> Result<Arc<Loaded>, String> {
    let sheet = imgcut::load(png, cut)?;

    let choices: Vec<Choice> = std::iter::once(Choice::Sheet)
        .chain(sheet.cuts.iter().enumerate().map(|(index, cut)| Choice::Cut(index, cut.label(index))))
        .collect();

    let outlines = sheet
        .cuts
        .iter()
        .map(|cut| picture::Outline {
            x: cut.x as f32,
            y: cut.y as f32,
            width: cut.width as f32,
            height: cut.height as f32,
        })
        .collect();

    let combo_width = picker::combo_width(choices.iter().map(Choice::to_string));

    let bytes = std::fs::read(png).map_err(|error| format!("Could not read {}: {}", png.display(), error))?;
    let (width, height) = sheet.size();

    Ok(Arc::new(Loaded { sheet, full: picture::Source::new(bytes, width, height), choices, outlines, combo_width }))
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).width(Length::Fill).align_x(Horizontal::Center).into()
}

fn feedback_color(theme: &Theme, busy: bool, feedback: Option<bool>) -> Color {
    let palette = theme.palette();

    if busy {
        return palette.warning;
    }

    feedback.map_or_else(|| palette.primary, |ok| if ok { palette.success } else { palette.danger })
}
