use std::sync::mpsc::Sender;

use crate::modules::settings::UserKeys;

pub fn verify(enforce_validation: bool, status_sender: &Sender<String>) -> Result<UserKeys, String> {
    let _ = status_sender.send("Validating keys...".to_string());

    let user_keys = UserKeys::load();

    if user_keys.is_empty() {
        let _ = status_sender.send("ERROR: Missing keys. Add keys at Settings > Data > Manage Keys".to_string());
        return Err("Missing keys".to_string());
    }

    if enforce_validation {
        let validation_results = user_keys.validate();
        let all_valid = validation_results.iter().all(|&(k, iv)| k && iv);

        if !all_valid {
            let _ = status_sender.send("ERROR: Keys do not match expected hash. Make sure you have the correct keys.".to_string());
            let _ = status_sender.send("If the keys have recently changed, you can continue by disabling Enforce Key Validation in Settings > Data".to_string());
            return Err("Validation failed".to_string());
        }

        let _ = status_sender.send("Keys validated.".to_string());
    }

    Ok(user_keys)
}