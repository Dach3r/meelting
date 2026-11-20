use crate::domain::agent_run::{AgentRun, RunStatus};
use crate::domain::recipient::{Resolution, Selection};
use crate::services::agent_run_cancel;
use crate::services::agent_run_engine::{Emit, Engine, RunEvent};
use crate::services::availability_check::{self, AgentAvailability};
use crate::services::intervention_send::{self, Intervention, InterventionSent};
use crate::services::intervention_stop;
use crate::services::recipient_resolve;
use crate::services::run_notice;
use crate::services::session_load::SessionError;
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

pub fn emitter(app: AppHandle) -> Emit {
    Arc::new(move |event| {
        let _ = match event {
            RunEvent::Status(run) => {
                notify_when_unfocused(&app, &run);
                app.emit("agent_run:status", run)
            }
            RunEvent::Token(chunk) => app.emit("agent_run:token", chunk),
            RunEvent::MessageCreated(message) => app.emit("message:created", message),
        };
    })
}

fn notify_when_unfocused(app: &AppHandle, run: &AgentRun) {
    if !matches!(run.status, RunStatus::Succeeded | RunStatus::Failed) {
        return;
    }
    let focused = app
        .get_webview_window("main")
        .is_some_and(|window| window.is_focused().unwrap_or(false));
    if focused {
        return;
    }
    let app = app.clone();
    let run = run.clone();
    tauri::async_runtime::spawn(async move {
        let pool = app.state::<SqlitePool>();
        let Ok(Some(notice)) = run_notice::build(&pool, &run).await else {
            return;
        };
        let mut builder = app
            .notification()
            .builder()
            .title(notice.title)
            .body(notice.body);
        if notice.sound {
            builder = builder.sound("default");
        }
        let _ = builder.show();
    });
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
pub async fn resolve_recipient(
    engine: State<'_, Arc<Engine>>,
    room_id: String,
    body: String,
    selection: Selection,
    mention_ids: Vec<String>,
) -> Result<Resolution, SessionError> {
    recipient_resolve::run(&engine.pool, &room_id, &body, &selection, &mention_ids).await
}

#[tauri::command]
pub async fn check_agent_availability(
    engine: State<'_, Arc<Engine>>,
    room_id: String,
) -> Result<Vec<AgentAvailability>, SessionError> {
    availability_check::run(&engine, &room_id).await
}

#[tauri::command]
pub async fn cancel_agent_run(
    engine: State<'_, Arc<Engine>>,
    run_id: String,
) -> Result<(), SessionError> {
    agent_run_cancel::run(&engine, &run_id).await
}

#[tauri::command]
pub async fn stop_intervention(
    engine: State<'_, Arc<Engine>>,
    room_id: String,
) -> Result<(), SessionError> {
    intervention_stop::run(&engine, &room_id).await
}
