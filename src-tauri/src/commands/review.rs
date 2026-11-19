use crate::domain::review::{DiffRequest, EnvironmentReview, FileDiff, FolderListing};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::review_registry::ReviewRegistry;
use crate::services::session_load::SessionError;
use crate::services::{environment_diff, environment_review};
use crate::system::storage;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn review_task_environment(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    reviews: State<'_, ReviewRegistry>,
    task_id: String,
) -> Result<EnvironmentReview, SessionError> {
    environment_review::run(&pool, &locks, &reviews, &task_id, storage::check).await
}

#[tauri::command]
pub async fn read_task_environment_diff(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    reviews: State<'_, ReviewRegistry>,
    task_id: String,
    request: DiffRequest,
) -> Result<FileDiff, SessionError> {
    environment_diff::run(&pool, &locks, &reviews, &task_id, &request, storage::check).await
}

#[tauri::command]
pub async fn list_task_environment_folder(
    pool: State<'_, SqlitePool>,
    locks: State<'_, RepositoryLocks>,
    reviews: State<'_, ReviewRegistry>,
    task_id: String,
    review_id: String,
    entry_id: u32,
) -> Result<FolderListing, SessionError> {
    environment_review::folder(
        &pool,
        &locks,
        &reviews,
        &task_id,
        &review_id,
        entry_id,
        storage::check,
    )
    .await
}
