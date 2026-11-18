use crate::repositories::agent_runs;
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::session_load::{self, SessionError};

pub async fn run(engine: &Engine, run_id: &str) -> Result<(), SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    if let Some(cancelled) = agent_runs::cancel(&engine.pool, &workspace_id, run_id).await? {
        engine.registry.cancel(run_id);
        engine.emit(RunEvent::Status(cancelled));
    }
    Ok(())
}
