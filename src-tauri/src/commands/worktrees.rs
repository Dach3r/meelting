use crate::commands::dialog;
use crate::domain::task::Task;
use crate::domain::worktree::{RemovalCheck, WorktreesRoot};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::services::{
    worktree_discard, worktree_prepare, worktree_remove, worktrees_root_change, worktrees_root_get,
};
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

#[tauri::command]
pub async fn prepare_task_environment(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_id: String,
) -> Result<Task, SessionError> {
    worktree_prepare::run(&pool, &locks, &task_id, storage::check).await
}

#[tauri::command]
pub async fn discard_task_environment(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_id: String,
) -> Result<Task, SessionError> {
    worktree_discard::run(&pool, &locks, &task_id).await
}

#[tauri::command]
pub async fn check_task_environment_removal(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_id: String,
) -> Result<RemovalCheck, SessionError> {
    worktree_remove::check(&pool, &locks, &task_id, storage::check).await
}

#[tauri::command]
pub async fn remove_task_environment(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_id: String,
) -> Result<Task, SessionError> {
    worktree_remove::run(&pool, &locks, &task_id, storage::check).await
}
