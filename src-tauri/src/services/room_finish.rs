use crate::domain::room::RoomList;
use crate::repositories::{agent_runs, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::rooms_list;
use crate::services::session_load::{self, SessionError};

pub async fn run(engine: &Engine, room_id: &str) -> Result<RoomList, SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    let mut transaction = engine.pool.begin_with("BEGIN IMMEDIATE").await?;
    if !rooms::finish(&mut *transaction, &workspace_id, room_id).await? {
        return Err(SessionError::RoomNotFound);
    }
    let cancelled = agent_runs::cancel_all_active(&mut *transaction, room_id).await?;
    transaction.commit().await?;
    engine.registry.cancel_room(room_id);
    for run in cancelled {
        engine.emit(RunEvent::Status(run));
    }
    rooms_list::run(&engine.pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::{fast_limits, harness};
    use crate::services::{message_send, room_create};

    async fn count(engine: &Engine, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(&engine.pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn marks_the_room_as_finished_and_keeps_its_history() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[("ollama", "qwen")]).await;
        let kept = room_create::run(&h.engine.pool, "Kept", h.agents.clone())
            .await
            .unwrap();
        message_send::run(&h.engine.pool, &h.room_id, "hola")
            .await
            .unwrap();

        let list = run(&h.engine, &h.room_id).await.unwrap();

        let ids: Vec<&str> = list.rooms.iter().map(|listed| listed.id.as_str()).collect();
        assert_eq!(ids, [kept.id.as_str(), h.room_id.as_str()]);
        assert!(list.rooms[0].finished_at.is_none());
        assert!(list.rooms[1].finished_at.is_some());
        assert_eq!(count(&h.engine, "SELECT COUNT(*) FROM messages").await, 1);
        assert_eq!(
            count(&h.engine, "SELECT COUNT(*) FROM room_agents").await,
            2
        );
    }

    #[tokio::test]
    async fn rejects_unknown_rooms_and_finishing_a_room_twice() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[]).await;

        assert!(matches!(
            run(&h.engine, "missing").await,
            Err(SessionError::RoomNotFound)
        ));
        run(&h.engine, &h.room_id).await.unwrap();
        assert!(matches!(
            run(&h.engine, &h.room_id).await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
