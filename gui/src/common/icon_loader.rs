use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver};
use image::RgbaImage;

pub(crate) type Composite = fn(&PathBuf, &RgbaImage) -> Option<(u32, u32, Vec<u8>)>;

pub(crate) struct LoadRequest {
    pub(crate) id: u32,
    pub(crate) path: PathBuf,
    pub(crate) background: Arc<RgbaImage>,
    pub(crate) generation: u64,
}

#[derive(Clone)]
pub struct LoadResult {
    pub(crate) id: u32,
    pub(crate) payload: Option<(u32, u32, Vec<u8>)>,
    pub(crate) generation: u64,
}

enum LoadCommand {
    Rank(Vec<u32>),
    Request(LoadRequest),
    Focus(usize),
    Done,
}

pub(crate) struct Dispatcher {
    tx: Sender<LoadCommand>,
}

impl Dispatcher {
    pub(crate) fn set_rank(&self, ids: Vec<u32>) {
        let _ = self.tx.send(LoadCommand::Rank(ids));
    }

    pub(crate) fn set_focus(&self, row: usize) {
        let _ = self.tx.send(LoadCommand::Focus(row));
    }

    pub(crate) fn request(&self, request: LoadRequest) {
        let _ = self.tx.send(LoadCommand::Request(request));
    }
}

struct DoneGuard(Sender<LoadCommand>);

impl Drop for DoneGuard {
    fn drop(&mut self) {
        let _ = self.0.send(LoadCommand::Done);
    }
}

pub(crate) fn spawn(composite: Composite) -> (Dispatcher, UnboundedReceiver<LoadResult>) {
    let (tx_command, rx_command) = mpsc::channel::<LoadCommand>();
    let (tx_result, rx_result) = unbounded::<LoadResult>();
    let tx_done = tx_command.clone();

    thread::spawn(move || {
        let limit = rayon::current_num_threads().max(1) * 2;
        let mut pending: Vec<LoadRequest> = Vec::new();
        let mut rank: HashMap<u32, usize> = HashMap::new();
        let mut focus = 0usize;
        let mut in_flight = 0usize;

        while let Ok(command) = rx_command.recv() {
            match command {
                LoadCommand::Rank(ids) => rank = ids.into_iter().enumerate().map(|(row, id)| (id, row)).collect(),
                LoadCommand::Request(request) => pending.push(request),
                LoadCommand::Focus(row) => focus = row,
                LoadCommand::Done => in_flight = in_flight.saturating_sub(1),
            }

            while in_flight < limit {
                let Some(pick) = best_pending(&pending, &rank, focus) else { break };
                let request = pending.swap_remove(pick);
                in_flight += 1;

                let tx = tx_result.clone();
                let guard = DoneGuard(tx_done.clone());
                rayon::spawn(move || {
                    let _guard = guard;
                    let payload = composite(&request.path, &request.background);
                    let _ = tx.unbounded_send(LoadResult { id: request.id, payload, generation: request.generation });
                });
            }
        }
    });

    (Dispatcher { tx: tx_command }, rx_result)
}

fn best_pending(pending: &[LoadRequest], rank: &HashMap<u32, usize>, focus: usize) -> Option<usize> {
    pending
        .iter()
        .enumerate()
        .min_by_key(|(_, request)| match rank.get(&request.id) {
            Some(&row) => (0, row.abs_diff(focus)),
            None => (1, 0),
        })
        .map(|(index, _)| index)
}
