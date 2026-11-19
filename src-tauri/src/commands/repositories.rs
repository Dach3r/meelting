use crate::commands::dialog;
use crate::domain::repository::{Repository, RepositoryBranches};
use crate::services::session_load::SessionError;
use crate::services::{
    repositories_list, repository_branches_list, repository_register, repository_unregister,
};
use crate::system::storage;
use sqlx::SqlitePool;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub async fn list_repositories(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<Repository>, SessionError> {
    repositories_list::run(&pool).await
}

#[tauri::command]
pub async fn register_repository(
    window: WebviewWindow,
    pool: State<'_, SqlitePool>,
) -> Result<Option<Vec<Repository>>, SessionError> {
    let Some(folder) = dialog::pick_folder(&window).await else {
        return Ok(None);
    };
    repository_register::run(&pool, &folder, storage::check)
        .await
        .map(Some)
}

#[tauri::command]
pub async fn unregister_repository(
    pool: State<'_, SqlitePool>,
    repository_id: String,
) -> Result<Vec<Repository>, SessionError> {
    repository_unregister::run(&pool, &repository_id).await
}

#[tauri::command]
pub async fn list_repository_branches(
    pool: State<'_, SqlitePool>,
    repository_id: String,
) -> Result<RepositoryBranches, SessionError> {
    repository_branches_list::run(&pool, &repository_id).await
}
