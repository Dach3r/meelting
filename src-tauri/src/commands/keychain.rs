use crate::system::keychain::{self, KeychainError};

#[tauri::command]
pub fn save_secret(name: String, secret: String) -> Result<(), KeychainError> {
    keychain::save(&name, &secret)
}

#[tauri::command]
pub fn secret_last_four(name: String) -> Result<Option<String>, KeychainError> {
    keychain::last_four(&name)
}

#[tauri::command]
pub fn delete_secret(name: String) -> Result<(), KeychainError> {
    keychain::remove(&name)
}
