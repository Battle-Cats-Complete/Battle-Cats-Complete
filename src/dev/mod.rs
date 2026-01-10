#[cfg(feature = "dev")]
pub mod keys;
#[cfg(feature = "dev")]
pub mod decrypt;
#[cfg(feature = "dev")]
pub mod modpack;
#[cfg(feature = "dev")]
use std::sync::mpsc::Sender;

#[cfg(feature = "dev")]
pub fn run_decryption(folder_path: &str, region_code: &str, tx: Sender<String>) -> Result<(), String> {
    if region_code == "mod" {
        modpack::run(folder_path, tx)
    } else {
        decrypt::run(folder_path, region_code, tx)
    }
}