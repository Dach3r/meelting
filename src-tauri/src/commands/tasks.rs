use crate::domain::task::{Task, TaskDraft, TaskOrigin, TaskStatus};
use crate::services::session_load::SessionError;
use crate::services::{task_delete, task_save, task_status_change, tasks_list};
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn list_tasks(pool: State<'_, SqlitePool>) -> Result<Vec<Task>, SessionError> {
    tasks_list::run(&pool).await
}

#[tauri::command]
pub async fn create_task(
    pool: State<'_, SqlitePool>,
    draft: TaskDraft,
    origin: Option<TaskOrigin>,
) -> Result<Task, SessionError> {
    task_save::create(&pool, draft, origin).await
}

#[tauri::command]
pub async fn update_task(
    pool: State<'_, SqlitePool>,
    task_id: String,
    draft: TaskDraft,
) -> Result<Task, SessionError> {
    task_save::update(&pool, &task_id, draft).await
}

#[tauri::command]
pub async fn change_task_status(
    pool: State<'_, SqlitePool>,
    task_id: String,
    status: TaskStatus,
) -> Result<Task, SessionError> {
    task_status_change::run(&pool, &task_id, status).await
}

#[tauri::command]
pub async fn delete_task(
    pool: State<'_, SqlitePool>,
    task_id: String,
) -> Result<Vec<Task>, SessionError> {
    task_delete::run(&pool, &task_id).await
}
