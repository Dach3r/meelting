use crate::domain::locale::Locale;
use crate::domain::session::Session;
use crate::domain::team::{AgentDraft, Team};
use crate::services::session_load::SessionError;
use crate::services::{
    agent_create, first_launch, session_load, team_create, teams_list, workspace_create,
    workspace_switch,
};
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn load_session(pool: State<'_, SqlitePool>) -> Result<Option<Session>, SessionError> {
    session_load::run(&pool).await
}

#[tauri::command]
pub async fn complete_first_launch(
    pool: State<'_, SqlitePool>,
    locale: Locale,
    workspace_name: String,
) -> Result<Session, SessionError> {
    first_launch::run(&pool, locale, &workspace_name).await
}

#[tauri::command]
pub async fn create_workspace(
    pool: State<'_, SqlitePool>,
    name: String,
) -> Result<Session, SessionError> {
    workspace_create::run(&pool, &name).await
}

#[tauri::command]
pub async fn switch_workspace(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
) -> Result<Session, SessionError> {
    workspace_switch::run(&pool, &workspace_id).await
}

#[tauri::command]
pub async fn list_teams(pool: State<'_, SqlitePool>) -> Result<Vec<Team>, SessionError> {
    teams_list::run(&pool).await
}

#[tauri::command]
pub async fn create_team(
    pool: State<'_, SqlitePool>,
    name: String,
) -> Result<Vec<Team>, SessionError> {
    team_create::run(&pool, &name).await
}

#[tauri::command]
pub async fn create_agent(
    pool: State<'_, SqlitePool>,
    draft: AgentDraft,
) -> Result<Vec<Team>, SessionError> {
    agent_create::run(&pool, draft).await
}
