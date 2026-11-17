mod keychain;

#[tauri::command]
fn save_secret(name: String, secret: String) -> Result<(), keychain::KeychainError> {
    keychain::save(&name, &secret)
}

#[tauri::command]
fn secret_last_four(name: String) -> Result<Option<String>, keychain::KeychainError> {
    keychain::last_four(&name)
}

#[tauri::command]
fn delete_secret(name: String) -> Result<(), keychain::KeychainError> {
    keychain::remove(&name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_secret, secret_last_four, delete_secret])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
