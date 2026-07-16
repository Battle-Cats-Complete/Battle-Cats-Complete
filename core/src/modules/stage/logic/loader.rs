use crate::modules::settings::logic::state::ScannerConfig;

use super::scanner;
use super::state::StageDataState;

#[tracing::instrument(level = "debug", skip(state, config))]
pub fn restart_scan(state: &mut StageDataState, config: ScannerConfig) {
    tracing::debug!("Clearing stage cache and starting background scan");
    state.registry.clear_cache();
    state.initialized = false;
    state.scan_receiver = Some(scanner::start_scan(&config));
}

pub fn update_data(state: &mut StageDataState) {
    let Some(rx) = &state.scan_receiver else { return; };

    let Ok(new_reg) = rx.try_recv() else { return; };

    tracing::info!("Stage scan complete. Updating memory registry.");
    state.registry = new_reg;
    state.scan_receiver = None;
    state.initialized = true;
}