use crate::services::agent_run_engine::Engine;
use crate::services::provider_key_check::{self, ProviderKeyResult};
use crate::services::provider_keys_list::{self, ProviderKeyStatus};
use crate::services::session_load::SessionError;
use crate::services::{provider_key_delete, provider_key_save};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_provider_keys(
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<ProviderKeyStatus>, SessionError> {
    provider_keys_list::run(&engine).await
}

#[tauri::command]
pub async fn save_provider_key(
    engine: State<'_, Arc<Engine>>,
    provider: String,
    key: String,
) -> Result<ProviderKeyResult, SessionError> {
    provider_key_save::run(&engine, &provider, &key).await
}

#[tauri::command]
pub async fn check_provider_key(
    engine: State<'_, Arc<Engine>>,
    provider: String,
) -> Result<ProviderKeyResult, SessionError> {
    provider_key_check::run(&engine, &provider).await
}

#[tauri::command]
pub async fn delete_provider_key(
    engine: State<'_, Arc<Engine>>,
    provider: String,
) -> Result<Vec<ProviderKeyStatus>, SessionError> {
    provider_key_delete::run(&engine, &provider).await
}
