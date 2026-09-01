use std::time::Duration;

use iced::{task, Element, Task};
use tracing::debug;

use kore::domains::mining;

use crate::app::theme;
use crate::common::feedback::{Slot as Confirm, CONFIRM_LABEL};

const BANNER_LIFE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    RequestDelete,
    ConfirmExpired,
    DeleteFinished,
    DoneExpired,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Deleting,
    Done,
}

#[derive(Default)]
pub struct State {
    phase: Phase,
    present: bool,
    confirm: Confirm<()>,
    delete_handle: Option<task::Handle>,
    banner_handle: Option<task::Handle>,
}

impl State {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => {
                self.present = mining::has_snapshot();

                Task::none()
            }
            Message::RequestDelete => {
                if !self.confirm.is_set() {
                    return self.confirm.set((), Message::ConfirmExpired);
                }

                self.confirm.clear();

                self.start_delete()
            }
            Message::ConfirmExpired => {
                self.confirm.expire();

                Task::none()
            }
            Message::DeleteFinished => {
                self.phase = Phase::Done;
                self.delete_handle = None;
                self.present = mining::has_snapshot();

                let (banner_task, handle) =
                    Task::perform(async { smol::Timer::after(BANNER_LIFE).await }, |_| Message::DoneExpired)
                        .abortable();

                self.banner_handle = Some(handle);

                banner_task
            }
            Message::DoneExpired => {
                self.phase = Phase::Idle;
                self.banner_handle = None;

                Task::none()
            }
        }
    }

    fn start_delete(&mut self) -> Task<Message> {
        debug!("Deleting the mining snapshot");

        let (delete_task, handle) =
            Task::perform(smol::unblock(mining::forget), |()| Message::DeleteFinished).abortable();

        self.phase = Phase::Deleting;
        self.delete_handle = Some(handle);

        delete_task
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.phase {
            Phase::Deleting => {
                theme::sized_button("Deleting Snapshot...", theme::ACTION_BUTTON_WIDTH, theme::warning_button).into()
            }
            Phase::Done => {
                theme::sized_button("Deleted Snapshot!", theme::ACTION_BUTTON_WIDTH, theme::success_button).into()
            }
            Phase::Idle if self.present => {
                let label = if self.confirm.is_set() { CONFIRM_LABEL } else { "Delete Snapshot" };

                theme::sized_button(label, theme::ACTION_BUTTON_WIDTH, theme::danger_button)
                    .on_press(Message::RequestDelete)
                    .into()
            }
            Phase::Idle => {
                theme::sized_button("No Snapshot", theme::ACTION_BUTTON_WIDTH, theme::neutral_button).into()
            }
        }
    }
}
