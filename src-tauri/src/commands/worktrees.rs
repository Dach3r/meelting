use crate::commands::dialog;
use crate::domain::file_copy::CopyPreview;
use crate::domain::task::Task;
use crate::domain::worktree::{Environment, RemovalCheck, WorktreesRoot};
use crate::services::file_copy::{self, CopyReport, CopySelections};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::services::worktree_trash::{self, EnvironmentFailure};
use crate::services::{
    environments_list, worktree_discard, worktree_prepare, worktree_purge, worktree_remove,
    worktrees_root_change, worktrees_root_get,
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

#[tauri::command]
pub async fn pick_task_environment_files(
    window: WebviewWindow,
    pool: State<'_, SqlitePool>,
    selections: State<'_, CopySelections>,
    task_id: String,
) -> Result<Option<CopyPreview>, SessionError> {
    file_copy::preview(&pool, &selections, &task_id, async |folder| {
        dialog::pick_files(&window, folder).await
    })
    .await
}

#[tauri::command]
pub async fn copy_task_environment_files(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    selections: State<'_, CopySelections>,
    task_id: String,
    selection_id: String,
    accept_secrets: bool,
) -> Result<CopyReport, SessionError> {
    file_copy::run(
        &pool,
        &locks,
        &selections,
        &task_id,
        &selection_id,
        accept_secrets,
        storage::check,
    )
    .await
}

#[tauri::command]
pub async fn list_task_environments(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<Environment>, SessionError> {
    environments_list::run(&pool).await
}

#[tauri::command]
pub async fn trash_task_environments(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_ids: Vec<String>,
) -> Result<Vec<EnvironmentFailure>, SessionError> {
    Ok(worktree_trash::trash(&pool, &locks, &task_ids, storage::check).await)
}

#[tauri::command]
pub async fn restore_task_environments(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_ids: Vec<String>,
) -> Result<Vec<EnvironmentFailure>, SessionError> {
    Ok(worktree_trash::restore(&pool, &locks, &task_ids, storage::check).await)
}

#[tauri::command]
pub async fn purge_task_environments(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    task_ids: Vec<String>,
) -> Result<Vec<EnvironmentFailure>, SessionError> {
    Ok(worktree_purge::run(&pool, &locks, &task_ids, storage::check).await)
}
