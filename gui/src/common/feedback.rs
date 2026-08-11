use std::time::Duration;

use iced::{task, Task};

const EXPIRY: Duration = Duration::from_secs(2);

pub const CONFIRM_LABEL: &str = "Are You Sure?";

pub struct Slot<T> {
    value: Option<T>,
    handle: Option<task::Handle>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self { value: None, handle: None }
    }
}

impl<T> Slot<T> {
    pub fn set<M: Send + 'static>(&mut self, value: T, expired: M) -> Task<M> {
        self.value = Some(value);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        let (task, handle) = Task::perform(
            async {
                smol::Timer::after(EXPIRY).await;
            },
            move |_| expired,
        )
        .abortable();
        self.handle = Some(handle);
        task
    }

    pub fn expire(&mut self) {
        self.value = None;
        self.handle = None;
    }

    pub fn clear(&mut self) {
        self.value = None;
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn is_set(&self) -> bool {
        self.value.is_some()
    }

    pub fn confirm_label<'a>(&self, idle: &'a str) -> &'a str {
        if self.is_set() { CONFIRM_LABEL } else { idle }
    }
}

impl<T: PartialEq> Slot<T> {
    pub fn armed_for(&self, value: &T) -> bool {
        self.value.as_ref() == Some(value)
    }

    pub fn take(&mut self, value: &T) -> bool {
        if !self.armed_for(value) {
            return false;
        }

        self.clear();
        true
    }
}
