use crate::domain::task::{Task, TaskDraft, TaskOrigin, TaskStatus};
use crate::services::agent_run_engine::Engine;
use crate::services::session_load::SessionError;
use crate::services::{
    task_delete, task_room_link, task_save, task_status_change, task_title_generate, tasks_list,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn list_tasks(pool: State<'_, SqlitePool>) -> Result<Vec<Task>, SessionError> {
    tasks_list::run(&pool).await
}

#[tauri::command]
pub async fn create_task(
    app: AppHandle,
    engine: State<'_, Arc<Engine>>,
    draft: TaskDraft,
    origin: Option<TaskOrigin>,
) -> Result<Task, SessionError> {
    let task = task_save::create(&engine.pool, draft, origin).await?;
    generate_title(&app, &engine, &task);
    Ok(task)
}

#[tauri::command]
pub async fn update_task(
    app: AppHandle,
    engine: State<'_, Arc<Engine>>,
    task_id: String,
    draft: TaskDraft,
) -> Result<Task, SessionError> {
    let task = task_save::update(&engine.pool, &task_id, draft).await?;
    generate_title(&app, &engine, &task);
    Ok(task)
}

#[tauri::command]
pub async fn link_task_to_room(
    pool: State<'_, SqlitePool>,
    task_id: String,
    origin: TaskOrigin,
) -> Result<Task, SessionError> {
    task_room_link::run(&pool, &task_id, origin).await
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

fn generate_title(app: &AppHandle, engine: &Arc<Engine>, task: &Task) {
    let app = app.clone();
    task_title_generate::spawn(engine, task.clone(), move |task| {
        let _ = app.emit("task:updated", task);
    });
}
