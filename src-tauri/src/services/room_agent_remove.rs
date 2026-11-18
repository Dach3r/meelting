use crate::domain::room::Roster;
use crate::repositories::{agent_runs, room_agents, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::room_agents_add::roster;
use crate::services::session_load::{self, SessionError};

pub async fn run(engine: &Engine, room_id: &str, agent_id: &str) -> Result<Roster, SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    let mut transaction = engine.pool.begin_with("BEGIN IMMEDIATE").await?;
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    if !room_agents::leave(&mut *transaction, room_id, agent_id, &workspace_id).await? {
        return Err(SessionError::AgentNotFound);
    }
    let cancelled = agent_runs::cancel_active(&mut *transaction, room_id, agent_id).await?;
    transaction.commit().await?;
    if let Some(cancelled) = cancelled {
        engine.registry.cancel(&cancelled.id);
        engine.emit(RunEvent::Status(cancelled));
    }
    roster(&engine.pool, &workspace_id, room_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_run::ErrorKind;
    use crate::domain::recipient::Selection;
    use crate::services::agent_run_engine::testing::{chat, fast_limits, harness};
    use crate::services::intervention_send::{self, Intervention, InterventionSent};
    use crate::services::{room_agents_add, room_get};
    use std::time::Duration;
    use uuid::Uuid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer};

    #[tokio::test]
    async fn the_agent_leaves_cancelling_its_reply_and_keeping_its_history() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["late"]).set_delay(Duration::from_millis(500)))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        sqlx::query("INSERT INTO messages VALUES ('said', ?, ?, 'hi', 'x')")
            .bind(&h.room_id)
            .bind(&h.agents[0])
            .execute(&h.engine.pool)
            .await
            .unwrap();
        let sent = h.send(0, "take your time").await.unwrap();

        let roster = run(&h.engine, &h.room_id, &h.agents[0]).await.unwrap();

        assert!(roster.agents.is_empty());
        assert_eq!(roster.former_agents[0].id, h.agents[0]);
        assert_eq!(
            h.finished(&sent.run.id).await.error_kind,
            Some(ErrorKind::Cancelled)
        );
        let detail = room_get::run(&h.engine.pool, &h.engine.registry, &h.room_id)
            .await
            .unwrap();
        assert!(detail.room.agents.is_empty());
        assert_eq!(detail.former_agents, roster.former_agents);
        assert!(detail.messages.iter().any(|message| message.id == "said"));
    }

    #[tokio::test]
    async fn a_departed_agent_cannot_be_asked_until_it_rejoins() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        run(&h.engine, &h.room_id, &h.agents[0]).await.unwrap();

        assert!(matches!(
            h.send(0, "hola").await,
            Err(SessionError::AgentNotFound)
        ));
        let sent = intervention_send::run(
            &h.engine,
            Intervention {
                room_id: &h.room_id,
                body: "@Agent0 hola",
                selection: &Selection::Auto(None),
                mention_ids: &[],
                client_request_id: &Uuid::now_v7().to_string(),
            },
        )
        .await
        .unwrap();
        assert!(matches!(sent, InterventionSent::Sent { run: None, .. }));

        let roster = room_agents_add::run(&h.engine.pool, &h.room_id, vec![h.agents[0].clone()])
            .await
            .unwrap();
        assert_eq!(roster.agents[0].id, h.agents[0]);
        assert!(roster.former_agents.is_empty());
        let rejoined = h.send(0, "hola").await.unwrap();
        assert!(h.finished(&rejoined.run.id).await.error_kind.is_none());
    }

    #[tokio::test]
    async fn rejects_agents_that_are_not_in_the_room() {
        let h = harness(
            "http://127.0.0.1:9",
            fast_limits(),
            &[("ollama", "qwen-test")],
        )
        .await;
        run(&h.engine, &h.room_id, &h.agents[0]).await.unwrap();

        for agent_id in [h.agents[0].as_str(), "missing"] {
            assert!(matches!(
                run(&h.engine, &h.room_id, agent_id).await,
                Err(SessionError::AgentNotFound)
            ));
        }
        assert!(matches!(
            run(&h.engine, "missing", &h.agents[0]).await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
