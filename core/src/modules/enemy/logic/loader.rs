use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use crate::modules::settings::logic::state::ScannerConfig;

use super::scanner;
use super::state::EnemyDataState;

pub fn restart_scan(state: &mut EnemyDataState, config: ScannerConfig) {
    state.is_cold_scan = true;
    state.last_update_time = None;
    state.incoming_enemies.clear();
    state.active_scan_ids.clear();
    state.detail_key.clear();

    state.enemies.clear();

    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn resync_scan(state: &mut EnemyDataState, config: ScannerConfig) {
    state.active_scan_ids.clear();
    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn refresh_enemy(state: &mut EnemyDataState, id: u32, config: &ScannerConfig) {
    match scanner::scan_single(id, config) {
        Some(new_enemy) => {
            match state.enemies.binary_search_by_key(&new_enemy.id, |e| e.id) {
                Ok(pos) => state.enemies[pos] = new_enemy,
                Err(pos) => state.enemies.insert(pos, new_enemy),
            }
        }
        None => {
            if let Ok(pos) = state.enemies.binary_search_by_key(&id, |e| e.id) {
                state.enemies.remove(pos);
                if state.selected_enemy == Some(id) {
                    state.selected_enemy = None;
                }
            }
        }
    }
}

pub fn update_data(state: &mut EnemyDataState) {
    let Some(rx) = &state.scan_receiver else { return };

    let mut received_any = false;
    let mut is_done = false;

    loop {
        match rx.try_recv() {
            Ok(entry) => {
                let id = entry.id;

                state.active_scan_ids.insert(id);

                match state.enemies.binary_search_by_key(&id, |e| e.id) {
                    Ok(pos) => state.enemies[pos] = entry,
                    Err(pos) => state.enemies.insert(pos, entry),
                }

                received_any = true;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                is_done = true;
                break;
            }
        }
    }

    if received_any {
        let now = Instant::now();
        state.last_update_time = Some(now);

        if state.selected_enemy.is_none() && !state.enemies.is_empty() {
            state.selected_enemy = Some(state.enemies[0].id);
        }
    }

    if is_done {
        state.enemies.retain(|e| state.active_scan_ids.contains(&e.id));

        if let Some(sel) = state.selected_enemy
            && !state.active_scan_ids.contains(&sel) {
                state.selected_enemy = None;
            }

        state.scan_receiver = None;
    }
}