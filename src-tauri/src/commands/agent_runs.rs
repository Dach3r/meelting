use crate::domain::recipient::Selection;
use crate::services::agent_run_cancel;
use crate::services::agent_run_engine::{Emit, Engine, RunEvent};
use crate::services::intervention_send::{self, Intervention, InterventionSent};
use crate::services::session_load::SessionError;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub fn emitter(app: AppHandle) -> Emit {
    Arc::new(move |event| {
        let _ = match event {
            RunEvent::Status(run) => app.emit("agent_run:status", run),
            RunEvent::Token(chunk) => app.emit("agent_run:token", chunk),
            RunEvent::MessageCreated(message) => app.emit("message:created", message),
        };
    })
}

#[tauri::command]
pub async fn send_intervention(
    engine: State<'_, Arc<Engine>>,
    room_id: String,
    body: String,
    selection: Selection,
    mention_ids: Vec<String>,
    client_request_id: String,
) -> Result<InterventionSent, SessionError> {
    intervention_send::run(
        &engine,
        Intervention {
            room_id: &room_id,
            body: &body,
            selection: &selection,
            mention_ids: &mention_ids,
            client_request_id: &client_request_id,
        },
    )
    .await
}

#[tauri::command]
pub async fn cancel_agent_run(
    engine: State<'_, Arc<Engine>>,
    run_id: String,
) -> Result<(), SessionError> {
    agent_run_cancel::run(&engine, &run_id).await
}
