use crate::services::worktree_run;
use tauri::{AppHandle, Emitter, Manager};

const WORKTREES_DIR: &str = "meelting-worktrees";

#[tauri::command]
pub async fn run_in_worktree(
    app: AppHandle,
    repo_path: String,
    command: String,
) -> Result<String, String> {
    let root = app
        .path()
        .temp_dir()
        .map_err(|error| error.to_string())?
        .join(WORKTREES_DIR);
    let output_app = app.clone();
    let worktree = worktree_run::run(
        repo_path.as_ref(),
        &root,
        &command,
        move |text| {
            let _ = output_app.emit("terminal-output", text);
        },
        move |code| {
            let _ = app.emit("terminal-exit", code);
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(worktree.to_string_lossy().into_owned())
}
