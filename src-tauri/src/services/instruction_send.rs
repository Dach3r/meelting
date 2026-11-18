use crate::domain::agent_run::{AgentRun, Partial, is_canonical_client_request_id};
use crate::domain::room::{Message, normalize_body};
use crate::repositories::agent_runs::{self, NewRun};
use crate::repositories::{agents, messages, room_agents, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::session_load::{self, SessionError};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstructionSent {
    pub message: Message,
    pub run: AgentRun,
    pub partial: Option<Partial>,
}

pub struct Instruction<'a> {
    pub room_id: &'a str,
    pub agent_id: &'a str,
    pub body: &'a str,
    pub client_request_id: &'a str,
}

enum Sent {
    New(Message, AgentRun),
    Repeated(Message, AgentRun),
}

pub async fn run(
    engine: &Arc<Engine>,
    instruction: Instruction<'_>,
) -> Result<InstructionSent, SessionError> {
    if !is_canonical_client_request_id(instruction.client_request_id) {
        return Err(SessionError::InvalidClientRequestId);
    }
    let body = normalize_body(instruction.body).ok_or(SessionError::InvalidMessage)?;
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    match store(engine, &workspace_id, &instruction, &body).await? {
        Sent::Repeated(message, run) => Ok(InstructionSent {
            partial: engine.registry.partial(&run.id),
            message,
            run,
        }),
        Sent::New(message, run) => {
            engine.emit(RunEvent::MessageCreated(message.clone()));
            engine.emit(RunEvent::Status(run.clone()));
            engine.submit(run.clone());
            Ok(InstructionSent {
                partial: engine.registry.partial(&run.id),
                message,
                run,
            })
        }
    }
}

async fn store(
    engine: &Engine,
    workspace_id: &str,
    instruction: &Instruction<'_>,
    body: &str,
) -> Result<Sent, SessionError> {
    let room_id = instruction.room_id;
    let agent_id = instruction.agent_id;
    let mut transaction = engine.pool.begin_with("BEGIN IMMEDIATE").await?;
    if rooms::get(&mut *transaction, workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    if let Some(existing) = agent_runs::find_by_client_request(
        &mut *transaction,
        room_id,
        instruction.client_request_id,
    )
    .await?
    {
        let request = messages::get(&mut *transaction, room_id, &existing.request_message_id)
            .await?
            .ok_or(SessionError::ClientRequestConflict)?;
        if existing.agent_id != agent_id || request.body != body {
            return Err(SessionError::ClientRequestConflict);
        }
        return Ok(Sent::Repeated(request, existing));
    }
    if !room_agents::exists(&mut *transaction, room_id, agent_id).await? {
        return Err(SessionError::AgentNotFound);
    }
    if agent_runs::has_active(&mut *transaction, room_id, agent_id).await? {
        return Err(SessionError::AgentBusy);
    }
    let agent = agents::get(&mut *transaction, workspace_id, agent_id)
        .await?
        .ok_or(SessionError::AgentNotFound)?;
    let message = messages::insert_from_user(
        &mut *transaction,
        &Uuid::now_v7().to_string(),
        room_id,
        body,
    )
    .await?;
    let run = agent_runs::insert_queued(
        &mut *transaction,
        &NewRun {
            id: &Uuid::now_v7().to_string(),
            room_id,
            agent_id,
            request_message_id: &message.id,
            client_request_id: instruction.client_request_id,
            provider: &agent.provider,
            model: &agent.model,
            created_at: &message.created_at,
        },
    )
    .await?;
    rooms::touch(&mut *transaction, room_id, &message.created_at).await?;
    transaction.commit().await?;
    Ok(Sent::New(message, run))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_run::{ErrorKind, RunStatus};
    use crate::services::agent_run_engine::testing::{chat, fast_limits, harness};
    use crate::services::workspace_create;
    use futures_util::future::join_all;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer};

    const ID: &str = "0196f3a0-0000-7000-8000-000000000001";

    async fn local_server(expected_calls: u64) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .expect(expected_calls)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn stores_the_user_message_and_a_queued_run_with_the_agent_engine() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;

        let sent = h.send_with(0, "  Review it  ", ID).await.unwrap();

        assert_eq!(sent.message.agent_id, None);
        assert_eq!(sent.message.body, "Review it");
        assert_eq!(sent.run.status, RunStatus::Queued);
        assert_eq!(sent.run.attempts, 0);
        assert_eq!(sent.run.request_message_id, sent.message.id);
        assert_eq!(
            (sent.run.provider.as_str(), sent.run.model.as_str()),
            ("ollama", "qwen-test")
        );
        assert_eq!(sent.run.created_at, sent.message.created_at);
        let events = h.events();
        assert_eq!(events[0], RunEvent::MessageCreated(sent.message.clone()));
        assert_eq!(events[1], RunEvent::Status(sent.run.clone()));
        h.finished(&sent.run.id).await;
    }

    #[tokio::test]
    async fn repeating_the_same_request_returns_the_original_without_side_effects() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        let first = h.send_with(0, "Review it", ID).await.unwrap();
        let done = h.finished(&first.run.id).await;
        let events_before = h.events().len();

        let again = h.send_with(0, "  Review it ", ID).await.unwrap();

        assert_eq!(again.message, first.message);
        assert_eq!(again.run, done);
        assert_eq!(again.partial, None);
        assert_eq!(h.events().len(), events_before);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
        assert_eq!(
            h.count("SELECT COUNT(*) FROM messages WHERE agent_id IS NULL")
                .await,
            1
        );
    }

    #[tokio::test]
    async fn concurrent_repetitions_create_one_message_one_run_and_one_call() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;

        let results = join_all((0..12).map(|_| h.send_with(0, "Review it", ID))).await;

        let runs: Vec<String> = results
            .into_iter()
            .map(|result| result.unwrap().run.id)
            .collect();
        assert!(runs.iter().all(|id| *id == runs[0]));
        h.finished(&runs[0]).await;
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
        assert_eq!(
            h.count("SELECT COUNT(*) FROM messages WHERE agent_id IS NULL")
                .await,
            1
        );
    }

    #[tokio::test]
    async fn repeating_after_a_failure_never_reactivates_the_run() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(401))
            .expect(2)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        let first = h.send_with(0, "Review it", ID).await.unwrap();
        h.finished(&first.run.id).await;

        let again = h.send_with(0, "Review it", ID).await.unwrap();

        assert_eq!(again.run.status, RunStatus::Failed);
        assert_eq!(again.run.error_kind, Some(ErrorKind::ProviderAuthFailed));
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
        let manual_retry = h.send(0, "Review it").await.unwrap();
        assert_ne!(manual_retry.run.id, first.run.id);
        h.finished(&manual_retry.run.id).await;
    }

    #[tokio::test]
    async fn repeating_after_expiry_returns_the_expired_run() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        let mut transaction = h.engine.pool.begin().await.unwrap();
        let message = crate::repositories::messages::insert_from_user(
            &mut *transaction,
            "m1",
            &h.room_id,
            "Review it",
        )
        .await
        .unwrap();
        agent_runs::insert_queued(
            &mut *transaction,
            &NewRun {
                id: "run1",
                room_id: &h.room_id,
                agent_id: &h.agents[0],
                request_message_id: &message.id,
                client_request_id: ID,
                provider: "ollama",
                model: "qwen-test",
                created_at: &message.created_at,
            },
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        crate::services::agent_runs_expire::run(&h.engine.pool)
            .await
            .unwrap();

        let again = h.send_with(0, "Review it", ID).await.unwrap();

        assert_eq!(again.run.error_kind, Some(ErrorKind::Expired));
        assert!(h.events().is_empty());
    }

    #[tokio::test]
    async fn the_same_id_with_another_agent_or_text_is_a_conflict() {
        let server = local_server(1).await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("ollama", "qwen-test"), ("ollama", "qwen-test")],
        )
        .await;
        let first = h.send_with(0, "Review it", ID).await.unwrap();

        assert!(matches!(
            h.send_with(1, "Review it", ID).await,
            Err(SessionError::ClientRequestConflict)
        ));
        assert!(matches!(
            h.send_with(0, "Something else", ID).await,
            Err(SessionError::ClientRequestConflict)
        ));
        h.finished(&first.run.id).await;
        assert_eq!(
            h.count("SELECT COUNT(*) FROM messages WHERE agent_id IS NULL")
                .await,
            1
        );
    }

    #[tokio::test]
    async fn validates_the_id_body_room_and_agent_without_saving_anything() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        let other = crate::services::team_create::run(&h.engine.pool, "Other")
            .await
            .unwrap();
        let outsider = crate::services::agent_create::run(
            &h.engine.pool,
            crate::domain::team::AgentDraft {
                team_id: other[1].id.clone(),
                template_key: None,
                name: "Outsider".into(),
                role: "QA Engineer".into(),
                description: String::new(),
                instructions: String::new(),
                provider: "ollama".into(),
                model: "qwen-test".into(),
            },
        )
        .await
        .unwrap()[1]
            .agents[0]
            .id
            .clone();
        let send = |room_id: &str, agent_id: &str, body: &str, id: &str| {
            let engine = h.engine.clone();
            let (room_id, agent_id, body, id) = (
                room_id.to_string(),
                agent_id.to_string(),
                body.to_string(),
                id.to_string(),
            );
            async move {
                run(
                    &engine,
                    Instruction {
                        room_id: &room_id,
                        agent_id: &agent_id,
                        body: &body,
                        client_request_id: &id,
                    },
                )
                .await
            }
        };

        assert!(matches!(
            send(
                &h.room_id,
                &h.agents[0],
                "hi",
                "0196F3A0-0000-7000-8000-000000000001"
            )
            .await,
            Err(SessionError::InvalidClientRequestId)
        ));
        assert!(matches!(
            send(&h.room_id, &h.agents[0], "hi", "not-a-uuid").await,
            Err(SessionError::InvalidClientRequestId)
        ));
        assert!(matches!(
            send(&h.room_id, &h.agents[0], "   ", ID).await,
            Err(SessionError::InvalidMessage)
        ));
        assert!(matches!(
            send(&h.room_id, &h.agents[0], &"a".repeat(4001), ID).await,
            Err(SessionError::InvalidMessage)
        ));
        assert!(matches!(
            send("missing", &h.agents[0], "hi", ID).await,
            Err(SessionError::RoomNotFound)
        ));
        assert!(matches!(
            send(&h.room_id, &outsider, "hi", ID).await,
            Err(SessionError::AgentNotFound)
        ));
        workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();
        assert!(matches!(
            send(&h.room_id, &h.agents[0], "hi", ID).await,
            Err(SessionError::RoomNotFound)
        ));
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 0);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
    }

    #[tokio::test]
    async fn an_agent_with_an_active_run_is_busy() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]).set_delay(std::time::Duration::from_millis(300)))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        let first = h.send(0, "One").await.unwrap();

        assert!(matches!(
            h.send(0, "Two").await,
            Err(SessionError::AgentBusy)
        ));
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 1);
        h.finished(&first.run.id).await;
        assert!(h.send(0, "Three").await.is_ok());
    }

    #[tokio::test]
    async fn a_failed_run_insert_leaves_no_message() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        sqlx::query("CREATE TRIGGER reject_runs BEFORE INSERT ON agent_runs BEGIN SELECT RAISE(ABORT, 'rejected'); END")
            .execute(&h.engine.pool)
            .await
            .unwrap();
        let result = h.send(0, "Review it").await;

        assert!(matches!(result, Err(SessionError::Database(_))));
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 0);
        assert!(h.events().is_empty());
    }
}
