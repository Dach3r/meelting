mod keychain;
mod terminal;
mod worktree;

use tauri::{Emitter, Manager};

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

#[tauri::command]
async fn run_in_worktree(
    app: tauri::AppHandle,
    repo_path: String,
    command: String,
) -> Result<String, String> {
    let root = app
        .path()
        .temp_dir()
        .map_err(|error| error.to_string())?
        .join("meelting-worktrees");
    let worktree = worktree::create(repo_path.as_ref(), &root)
        .await
        .map_err(|error| error.to_string())?;
    let output_app = app.clone();
    terminal::spawn(
        &command,
        &worktree,
        move |text| {
            let _ = output_app.emit("terminal-output", text);
        },
        move |code| {
            let _ = app.emit("terminal-exit", code);
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(worktree.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_secret,
            secret_last_four,
            delete_secret,
            run_in_worktree
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
