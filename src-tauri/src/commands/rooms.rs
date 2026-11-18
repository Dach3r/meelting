use crate::domain::room::{Room, RoomDetail, RoomList};
use crate::services::agent_run_engine::Engine;
use crate::services::session_load::SessionError;
use crate::services::{room_create, room_delete, room_get, rooms_list};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::State;

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
    engine: State<'_, Arc<Engine>>,
    room_id: String,
) -> Result<RoomDetail, SessionError> {
    room_get::run(&engine.pool, &engine.registry, &room_id).await
}

#[tauri::command]
pub async fn delete_room(
    engine: State<'_, Arc<Engine>>,
    room_id: String,
) -> Result<RoomList, SessionError> {
    room_delete::run(&engine.pool, &engine.registry, &room_id).await
}
