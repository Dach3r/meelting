use crate::repositories::{agent_runs, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::session_load::{self, SessionError};

pub async fn run(engine: &Engine, room_id: &str) -> Result<(), SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    if rooms::get(&engine.pool, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    engine.registry.end_round(room_id);
    let cancelled = agent_runs::cancel_all_active(&engine.pool, room_id).await?;
    engine.registry.cancel_room(room_id);
    for run in cancelled {
        engine.emit(RunEvent::Status(run));
    }
    Ok(())
}
