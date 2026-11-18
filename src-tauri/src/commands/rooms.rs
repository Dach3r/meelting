use crate::domain::room::{Message, Room, RoomDetail, RoomList};
use crate::services::session_load::SessionError;
use crate::services::{message_send, room_create, room_delete, room_get, rooms_list};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn list_rooms(pool: State<'_, SqlitePool>) -> Result<RoomList, SessionError> {
    rooms_list::run(&pool).await
}

#[tauri::command]
pub async fn create_room(
    pool: State<'_, SqlitePool>,
    name: String,
    agent_ids: Vec<String>,
) -> Result<Room, SessionError> {
    room_create::run(&pool, &name, agent_ids).await
}

#[tauri::command]
pub async fn get_room(
    pool: State<'_, SqlitePool>,
    room_id: String,
) -> Result<RoomDetail, SessionError> {
    room_get::run(&pool, &room_id).await
}

#[tauri::command]
pub async fn delete_room(
    pool: State<'_, SqlitePool>,
    room_id: String,
) -> Result<RoomList, SessionError> {
    room_delete::run(&pool, &room_id).await
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    room_id: String,
    body: String,
) -> Result<Message, SessionError> {
    let message = message_send::run(&pool, &room_id, &body).await?;
    let _ = app.emit("message:created", &message);
    Ok(message)
}
