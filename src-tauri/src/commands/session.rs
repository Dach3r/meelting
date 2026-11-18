use crate::catalogs::agent_roles::RoleGroup;
use crate::catalogs::agent_templates::AgentTemplate;
use crate::catalogs::llm::Provider;
use crate::domain::locale::Locale;
use crate::domain::session::Session;
use crate::domain::team::{AgentDetail, AgentDraft, AgentEdit, Team};
use crate::domain::theme::Theme;
use crate::services::session_load::SessionError;
use crate::services::{
    agent_create, agent_delete, agent_get, agent_roles_list, agent_templates_list, agent_update,
    first_launch, llm_providers_list, locale_change, session_load, team_create, teams_list,
    theme_change, workspace_create, workspace_switch,
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

#[tauri::command]
pub async fn get_agent(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<AgentDetail, SessionError> {
    agent_get::run(&pool, &agent_id).await
}

#[tauri::command]
pub async fn update_agent(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    edit: AgentEdit,
) -> Result<Vec<Team>, SessionError> {
    agent_update::run(&pool, &agent_id, edit).await
}

#[tauri::command]
pub async fn delete_agent(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<Vec<Team>, SessionError> {
    agent_delete::run(&pool, &agent_id).await
}

#[tauri::command]
pub fn list_agent_roles() -> Vec<RoleGroup> {
    agent_roles_list::run()
}

#[tauri::command]
pub fn list_llm_providers() -> Vec<Provider> {
    llm_providers_list::run()
}

#[tauri::command]
pub async fn list_agent_templates(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AgentTemplate>, SessionError> {
    agent_templates_list::run(&pool).await
}

#[tauri::command]
pub async fn change_locale(
    pool: State<'_, SqlitePool>,
    locale: Locale,
) -> Result<Session, SessionError> {
    locale_change::run(&pool, locale).await
}

#[tauri::command]
pub async fn change_theme(
    pool: State<'_, SqlitePool>,
    theme: Theme,
) -> Result<Session, SessionError> {
    theme_change::run(&pool, theme).await
}
