use crate::services::completion_stream::{self, CompletionStreamError};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn stream_completion(
    app: AppHandle,
    base_url: String,
    model: String,
    prompt: String,
    secret_name: Option<String>,
) -> Result<(), CompletionStreamError> {
    completion_stream::run(
        &base_url,
        &model,
        &prompt,
        secret_name.as_deref(),
        |token| {
            let _ = app.emit("llm-token", token);
        },
    )
    .await
}
