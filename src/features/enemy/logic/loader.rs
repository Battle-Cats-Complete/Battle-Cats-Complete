use super::state::EnemyListState;
use super::scanner;
use crate::features::settings::logic::state::ScannerConfig;

pub fn restart_scan(state: &mut EnemyListState, config: ScannerConfig) {
    state.is_cold_scan = state.enemies.is_empty(); 
    state.last_update_time = None;
    state.incoming_enemies.clear();
    
    state.enemy_list.clear_cache(); 
    state.detail_texture = None;
    state.detail_key.clear();

    let (tx, rx) = std::sync::mpsc::channel();
    state.scan_receiver = Some(rx);

    std::thread::spawn(move || {
        let results = scanner::scan_all(&config);
        let _ = tx.send(results);
    });
}

pub fn refresh_enemy(state: &mut EnemyListState, id: u32, config: &ScannerConfig) {
    let updated_enemies = scanner::scan_all(config);
    
    if let Some(new_enemy) = updated_enemies.into_iter().find(|e| e.id == id) {
        if let Some(pos) = state.enemies.iter().position(|e| e.id == id) { 
            state.enemies[pos] = new_enemy; 
        } else {
            state.enemies.push(new_enemy); 
            state.enemies.sort_by_key(|e| e.id); 
        }
    }
}

pub fn update_data(state: &mut EnemyListState) {
    if let Some(rx) = &state.scan_receiver {
        if let Ok(results) = rx.try_recv() {
            state.enemies = results;
            
            if state.is_cold_scan && state.selected_enemy.is_none() {
                if let Some(first_enemy) = state.enemies.first() {
                    state.selected_enemy = Some(first_enemy.id);
                }
            }
            
            state.scan_receiver = None;
            state.last_update_time = None;
        }
    }
}