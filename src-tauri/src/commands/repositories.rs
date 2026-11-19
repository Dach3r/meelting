use crate::domain::repository::{Repository, RepositoryBranches};
use crate::services::session_load::SessionError;
use crate::services::{
    repositories_list, repository_branches_list, repository_register, repository_unregister,
};
use sqlx::SqlitePool;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

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
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .pick_folder(move |folder| {
            let _ = sender.send(folder);
        });
    let Some(folder) = receiver.await.ok().flatten() else {
        return Ok(None);
    };
    let folder = folder
        .into_path()
        .map_err(|_| SessionError::NotARepository)?;
    repository_register::run(&pool, &folder).await.map(Some)
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
