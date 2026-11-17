mod commands;
mod services;
mod system;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::keychain::save_secret,
            commands::keychain::secret_last_four,
            commands::keychain::delete_secret,
            commands::worktree::run_in_worktree,
            commands::completion::stream_completion
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
