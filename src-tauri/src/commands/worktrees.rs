use crate::commands::dialog;
use crate::domain::worktree::WorktreesRoot;
use crate::services::session_load::SessionError;
use crate::services::{worktrees_root_change, worktrees_root_get};
use crate::system::storage;
use sqlx::SqlitePool;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub async fn get_worktrees_root(
    pool: State<'_, SqlitePool>,
) -> Result<WorktreesRoot, SessionError> {
    worktrees_root_get::run(&pool, storage::check).await
}

#[tauri::command]
pub async fn change_worktrees_root(
    window: WebviewWindow,
    pool: State<'_, SqlitePool>,
) -> Result<Option<WorktreesRoot>, SessionError> {
    let Some(folder) = dialog::pick_folder(&window).await else {
        return Ok(None);
    };
    worktrees_root_change::run(&pool, &folder, storage::check)
        .await
        .map(Some)
}
