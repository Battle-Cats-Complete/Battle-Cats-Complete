use std::path::PathBuf;

use iced::widget::{column, container, pick_list, row, rule, text_input};
use iced::{Alignment, Element, Length, Padding, Size};

use kore::domains::studio::{self as sets, Set, Slot};

use crate::app::theme;
use crate::common::feedback::{self, Slot as Confirm};
use crate::widget::{picker, popup};

pub(super) const TITLE: &str = "Manage";
pub(super) const SPEC: popup::Spec = popup::Spec::new(popup::Kind::StudioManage, Size::new(372.0, 422.0));

const PADDING: f32 = 10.0;
const GAP: f32 = 8.0;
const LABEL_SIZE: f32 = 13.0;
const NAME_HINT: &str = "Name";
const TRACK_HINT: &str = "No animation is selected";
pub(super) const NONE_ENTRY: &str = "";
const SEALED_HINT: &str = "Named by the mount it lives in";

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Rename,
    Import,
    ImportExpired,
    Imported(Option<PathBuf>),
    New,
    Recall(String),
    Pick(Slot),
    Picked(Slot, Option<PathBuf>),
    PickExpired,
    TrackChanged(String),
    TrackRename,
    AddAnims,
    AnimsPicked(Vec<PathBuf>),
    NewAnim,
    DropAnim,
    DropExpired,
    Reveal,
}

#[derive(Default)]
pub(super) struct State {
    set: Set,
    name: String,
    track: Option<String>,
    known: Vec<String>,
    sealed: bool,
    pub(super) picking: Confirm<Slot>,
    pub(super) importing: Confirm<()>,
    pub(super) dropping: Confirm<()>,
    pub(super) renamer: Confirm<()>,
    pub(super) tracker: Confirm<()>,
}

impl State {
    pub(super) fn set(&self) -> &Set {
        &self.set
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn adopt(&mut self, set: Set) {
        self.name = set.name.clone();
        self.set = set;
        self.sealed = false;
        self.track = None;
    }

    pub(super) fn seal(&mut self, set: Set) {
        self.name = set.name.clone();
        self.set = set;
        self.sealed = true;
        self.track = None;
    }

    pub(super) fn sealed(&self) -> bool {
        self.sealed
    }

    pub(super) fn rename(&mut self, name: String) {
        self.name = name;
    }

    pub(super) fn retrack(&mut self, name: String) {
        self.track = Some(name);
    }

    pub(super) fn typed_track(&self) -> Option<&str> {
        self.track.as_deref()
    }

    pub(super) fn untrack(&mut self) {
        self.track = None;
    }

    pub(super) fn place(&mut self, slot: Slot, path: PathBuf) {
        self.set.place(slot, Some(path));
        self.sealed = false;
    }

    pub(super) fn anims_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.set.anims
    }

    pub(super) fn restock(&mut self) {
        self.known = sets::sets();
    }

    pub(super) fn view<'a>(
        &'a self,
        mount: Option<String>,
        held: Option<&'a str>,
        seedable: bool,
        folder: bool,
    ) -> Element<'a, Message> {
        let body = column![self.set_side(mount, folder), self.track_side(held, seedable)]
            .spacing(GAP + 4.0)
            .width(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding::from(PADDING))
            .into()
    }

    fn set_side(&self, mount: Option<String>, folder: bool) -> Element<'_, Message> {
        let sealed = mount.is_some();
        let seated = match mount {
            Some(mount) => format!("\"{}\" in \"{}\"", self.name, mount),
            None => self.name.clone(),
        };

        let name = text_input(if sealed { SEALED_HINT } else { NAME_HINT }, &seated)
            .size(LABEL_SIZE)
            .padding(picker::COMBO_PADDING)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let name = match sealed {
            true => name,
            false => name.on_input(Message::NameChanged),
        };

        let sourcing = row![
            picker::action(self.importing.confirm_label("Import Set"), Message::Import)
                .width(Length::Fill)
                .style(match self.importing.is_set() {
                    true => theme::danger_button,
                    false => theme::primary_button,
                }),
            picker::action("New Set", Message::New).width(Length::Fill).style(theme::primary_button),
        ]
        .spacing(GAP);

        let listed = self.known.contains(&self.name);
        let options: Vec<String> =
            std::iter::once(NONE_ENTRY.to_owned()).chain(self.known.iter().cloned()).collect();
        let chosen = listed.then(|| self.name.clone()).or_else(|| Some(NONE_ENTRY.to_owned()));

        let recall = pick_list(options, chosen, Message::Recall)
            .width(Length::Fill)
            .padding(picker::COMBO_PADDING)
            .text_size(picker::TEXT_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        let files = Slot::ALL.into_iter().fold(row![].spacing(GAP), |listed, slot| {
            listed.push(self.slot_button(slot))
        });

        let reveal = picker::action("Open Folder", Message::Reveal)
            .width(Length::Fill)
            .on_press_maybe(folder.then_some(Message::Reveal))
            .style(theme::primary_button);

        column![heading("Set"), name, sourcing, recall, files, reveal]
            .spacing(GAP)
            .width(Length::Fill)
            .into()
    }

    fn track_side<'a>(&'a self, held: Option<&'a str>, seedable: bool) -> Element<'a, Message> {
        let named = self.track.as_deref().or(held).unwrap_or_default();

        let name = text_input(TRACK_HINT, named)
            .size(LABEL_SIZE)
            .padding(picker::COMBO_PADDING)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let name = match held.is_some() {
            true => name.on_input(Message::TrackChanged),
            false => name,
        };

        let sourcing = row![
            picker::action("Add MAANIM", Message::AddAnims)
                .width(Length::Fill)
                .style(theme::primary_button),
            picker::action("New MAANIM", Message::NewAnim)
                .width(Length::Fill)
                .on_press_maybe(seedable.then_some(Message::NewAnim))
                .style(theme::primary_button),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center);

        column![heading("Animation"), name, sourcing, self.drop_button(held.is_some())]
            .spacing(GAP)
            .width(Length::Fill)
            .into()
    }

    fn slot_button(&self, slot: Slot) -> Element<'_, Message> {
        let armed = self.picking.armed_for(&slot);
        let held = self.set.slot(slot);

        if armed {
            return picker::action(feedback::CONFIRM_SHORT_LABEL, Message::Pick(slot))
                .width(Length::Fill)
                .style(theme::danger_button)
                .into();
        }

        picker::slot(slot.label(), held, Message::Pick(slot)).width(Length::Fill).into()
    }

    fn drop_button(&self, droppable: bool) -> Element<'_, Message> {
        if !droppable {
            return picker::action("Remove MAANIM", Message::DropAnim)
                .width(Length::Fill)
                .on_press_maybe(None)
                .style(theme::neutral_button)
                .into();
        }

        let label = self.dropping.confirm_label("Remove MAANIM");

        picker::action(label, Message::DropAnim).width(Length::Fill).style(theme::danger_button).into()
    }
}

fn heading<'a>(name: &'a str) -> Element<'a, Message> {
    column![theme::centered_text(name).size(LABEL_SIZE).width(Length::Fill), rule::horizontal(1)]
        .spacing(2)
        .into()
}
