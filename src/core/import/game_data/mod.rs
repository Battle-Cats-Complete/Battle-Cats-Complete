pub mod import;
pub mod export;

use std::sync::mpsc::Sender;

// Public logic
pub fn import_standard_folder(path: &str, tx: Sender<String>) -> Result<String, String> {
    import::import_from_folder(path, tx)
}

pub fn create_game_zip(tx: Sender<String>, level: i32) -> Result<(), String> {
    export::create_game_zip(tx, level)
}

pub fn import_standard_zip(path: &str, tx: Sender<String>) -> Result<String, String> {
    // Always use the standard zip extractor
    import::import_from_zip(path, tx)
}

// Developer logic
#[cfg(feature = "dev")]
pub fn run_dev_decryption(path: &str, region: &str, tx: Sender<String>) -> Result<String, String> {
    // We wrap the result to return a String on success
        .map(|_| "Success! Decryption complete.".to_string())
}