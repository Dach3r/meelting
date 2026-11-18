use crate::domain::agent_run::{AgentRun, Partial, is_canonical_client_request_id};
use crate::domain::recipient::{self, ChoiceReason, Resolution, Selection};
use crate::domain::room::{Message, normalize_body};
use crate::repositories::agent_runs::{self, NewRun};
use crate::repositories::{agents, messages, room_agents, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::services::message_send;
use crate::services::session_load::{self, SessionError};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum InterventionSent {
    Sent {
        message: Message,
        run: Option<Box<AgentRun>>,
        partial: Option<Partial>,
    },
    Choose {
        reason: ChoiceReason,
        candidates: Vec<String>,
    },
}

pub struct Intervention<'a> {
    pub room_id: &'a str,
    pub body: &'a str,
    pub selection: &'a Selection,
    pub mention_ids: &'a [String],
    pub client_request_id: &'a str,
}

enum Stored {
    Run(Message, AgentRun),
    Repeated(Message, AgentRun),
    Nobody,
    Choose(ChoiceReason, Vec<String>),
}

pub async fn run(
    engine: &Arc<Engine>,
    intervention: Intervention<'_>,
) -> Result<InterventionSent, SessionError> {
    if !is_canonical_client_request_id(intervention.client_request_id) {
        return Err(SessionError::InvalidClientRequestId);
    }
    let body = normalize_body(intervention.body).ok_or(SessionError::InvalidMessage)?;
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    let stored = match intervention.selection {
        Selection::Nobody => Stored::Nobody,
        _ => store(engine, &workspace_id, &intervention, &body).await?,
    };
    match stored {
        Stored::Nobody => {
            let message = message_send::run(&engine.pool, intervention.room_id, &body).await?;
            engine.emit(RunEvent::MessageCreated(message.clone()));
            Ok(InterventionSent::Sent {
                message,
                run: None,
                partial: None,
            })
        }
        Stored::Choose(reason, candidates) => Ok(InterventionSent::Choose { reason, candidates }),
        Stored::Repeated(message, run) => Ok(sent(engine, message, run)),
        Stored::Run(message, run) => {
            engine.emit(RunEvent::MessageCreated(message.clone()));
            engine.emit(RunEvent::Status(run.clone()));
            engine.submit(run.clone());
            Ok(sent(engine, message, run))
        }
    }
}

fn sent(engine: &Engine, message: Message, run: AgentRun) -> InterventionSent {
    InterventionSent::Sent {
        partial: engine.registry.partial(&run.id),
        message,
        run: Some(Box::new(run)),
    }
}

async fn store(
    engine: &Engine,
    workspace_id: &str,
    intervention: &Intervention<'_>,
    body: &str,
) -> Result<Stored, SessionError> {
    let room_id = intervention.room_id;
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
        intervention.client_request_id,
    )
    .await?
    {
        let request = messages::get(&mut *transaction, room_id, &existing.request_message_id)
            .await?
            .ok_or(SessionError::ClientRequestConflict)?;
        let other_agent = match intervention.selection {
            Selection::Agent(id) | Selection::Confirmed(id) => *id != existing.agent_id,
            Selection::Auto | Selection::Nobody => false,
        };
        if other_agent || request.body != body {
            return Err(SessionError::ClientRequestConflict);
        }
        return Ok(Stored::Repeated(request, existing));
    }
    let participants: Vec<_> = room_agents::list(&mut *transaction, workspace_id)
        .await?
        .into_iter()
        .filter(|(room, _)| room == room_id)
        .map(|(_, participant)| participant)
        .collect();
    let agent_id = match recipient::resolve(
        intervention.selection,
        body,
        intervention.mention_ids,
        &participants,
    )
    .map_err(|_| SessionError::AgentNotFound)?
    {
        Resolution::Nobody => return Ok(Stored::Nobody),
        Resolution::Choose { reason, candidates } => return Ok(Stored::Choose(reason, candidates)),
        Resolution::Agent { agent_id } => agent_id,
    };
    if agent_runs::has_active(&mut *transaction, room_id, &agent_id).await? {
        return Err(SessionError::AgentBusy);
    }
    let agent = agents::get(&mut *transaction, workspace_id, &agent_id)
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
            agent_id: &agent_id,
            request_message_id: &message.id,
            client_request_id: intervention.client_request_id,
            provider: &agent.provider,
            model: &agent.model,
            created_at: &message.created_at,
        },
    )
    .await?;
    rooms::touch(&mut *transaction, room_id, &message.created_at).await?;
    transaction.commit().await?;
    Ok(Stored::Run(message, run))
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
                    Intervention {
                        room_id: &room_id,
                        body: &body,
                        selection: &Selection::Agent(agent_id),
                        mention_ids: &[],
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

    async fn intervene(
        h: &crate::services::agent_run_engine::testing::Harness,
        selection: Selection,
        body: &str,
        mention_ids: &[String],
    ) -> Result<InterventionSent, SessionError> {
        run(
            &h.engine,
            Intervention {
                room_id: &h.room_id,
                body,
                selection: &selection,
                mention_ids,
                client_request_id: &Uuid::now_v7().to_string(),
            },
        )
        .await
    }

    fn run_of(sent: InterventionSent) -> Option<AgentRun> {
        match sent {
            InterventionSent::Sent { run, .. } => run.map(|run| *run),
            InterventionSent::Choose { .. } => panic!("expected a sent intervention"),
        }
    }

    fn two_agents() -> [(&'static str, &'static str); 2] {
        [("ollama", "qwen-test"), ("ollama", "qwen-test")]
    }

    #[tokio::test]
    async fn a_mention_runs_the_mentioned_agent() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let run = run_of(
            intervene(&h, Selection::Auto, "@Agent1 how?", &[])
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(run.agent_id, h.agents[1]);
        h.finished(&run.id).await;
    }

    #[tokio::test]
    async fn a_vocative_runs_the_named_agent() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let run = run_of(
            intervene(&h, Selection::Auto, "agent0, how?", &[])
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(run.agent_id, h.agents[0]);
        h.finished(&run.id).await;
    }

    #[tokio::test]
    async fn without_a_recipient_only_the_message_is_saved_and_announced() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let unknown = intervene(&h, Selection::Auto, "@Marta are you there?", &[])
            .await
            .unwrap();
        let plain = intervene(&h, Selection::Auto, "Just a note", &[])
            .await
            .unwrap();

        assert_eq!(run_of(unknown.clone()), None);
        assert_eq!(run_of(plain), None);
        let InterventionSent::Sent { message, .. } = unknown else {
            unreachable!()
        };
        assert_eq!(message.body, "@Marta are you there?");
        assert_eq!(h.events()[0], RunEvent::MessageCreated(message));
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 2);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
    }

    #[tokio::test]
    async fn explicit_nobody_keeps_a_mention_as_text() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let sent = intervene(
            &h,
            Selection::Nobody,
            "@Agent0 look",
            &[h.agents[0].clone()],
        )
        .await
        .unwrap();

        assert_eq!(run_of(sent), None);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
    }

    #[tokio::test]
    async fn ambiguity_several_agents_and_conflicts_save_nothing() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;
        let both = vec![h.agents[0].clone(), h.agents[1].clone()];

        assert_eq!(
            intervene(&h, Selection::Auto, "@QA Engineer thoughts?", &[])
                .await
                .unwrap(),
            InterventionSent::Choose {
                reason: ChoiceReason::Ambiguous,
                candidates: both.clone()
            }
        );
        assert_eq!(
            intervene(&h, Selection::Auto, "@Agent0 and @Agent1", &[])
                .await
                .unwrap(),
            InterventionSent::Choose {
                reason: ChoiceReason::Several,
                candidates: both.clone()
            }
        );
        assert_eq!(
            intervene(
                &h,
                Selection::Agent(h.agents[0].clone()),
                "@Agent1 how?",
                &[]
            )
            .await
            .unwrap(),
            InterventionSent::Choose {
                reason: ChoiceReason::Conflict,
                candidates: both
            }
        );
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 0);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
        assert!(h.events().is_empty());
    }

    #[tokio::test]
    async fn the_autocomplete_choice_and_a_confirmation_settle_the_recipient() {
        let server = local_server(2).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let chosen = run_of(
            intervene(
                &h,
                Selection::Auto,
                "@QA Engineer thoughts?",
                &[h.agents[1].clone()],
            )
            .await
            .unwrap(),
        )
        .unwrap();
        let confirmed = run_of(
            intervene(
                &h,
                Selection::Confirmed(h.agents[0].clone()),
                "@Agent1 how?",
                &[],
            )
            .await
            .unwrap(),
        )
        .unwrap();

        assert_eq!(chosen.agent_id, h.agents[1]);
        assert_eq!(confirmed.agent_id, h.agents[0]);
        h.finished(&chosen.id).await;
        h.finished(&confirmed.id).await;
    }

    #[tokio::test]
    async fn agents_of_other_rooms_or_workspaces_are_never_recipients() {
        let server = local_server(0).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;
        crate::services::room_create::run(&h.engine.pool, "Other", vec![h.agents[1].clone()])
            .await
            .unwrap();
        sqlx::query("DELETE FROM room_agents WHERE room_id = ? AND agent_id = ?")
            .bind(&h.room_id)
            .bind(&h.agents[1])
            .execute(&h.engine.pool)
            .await
            .unwrap();

        assert_eq!(
            run_of(
                intervene(&h, Selection::Auto, "@Agent1 hi", &[h.agents[1].clone()])
                    .await
                    .unwrap()
            ),
            None
        );
        assert!(matches!(
            intervene(&h, Selection::Agent(h.agents[1].clone()), "hi", &[]).await,
            Err(SessionError::AgentNotFound)
        ));
        assert!(matches!(
            intervene(&h, Selection::Confirmed(h.agents[1].clone()), "hi", &[]).await,
            Err(SessionError::AgentNotFound)
        ));
        workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();
        assert!(matches!(
            intervene(&h, Selection::Auto, "@Agent0 hi", &[]).await,
            Err(SessionError::RoomNotFound)
        ));
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
    }

    #[tokio::test]
    async fn during_a_run_the_same_agent_is_busy_and_others_answer_in_parallel() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]).set_delay(std::time::Duration::from_millis(300)))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;
        let first = run_of(
            intervene(&h, Selection::Auto, "@Agent0 one", &[])
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(matches!(
            intervene(&h, Selection::Auto, "Agent0, two", &[]).await,
            Err(SessionError::AgentBusy)
        ));
        let note = intervene(&h, Selection::Auto, "A note meanwhile", &[])
            .await
            .unwrap();
        let second = run_of(
            intervene(&h, Selection::Auto, "@Agent1 three", &[])
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(run_of(note), None);
        assert_eq!(h.finished(&first.id).await.status, RunStatus::Succeeded);
        assert_eq!(h.finished(&second.id).await.status, RunStatus::Succeeded);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 2);
    }

    #[tokio::test]
    async fn mentions_written_by_agents_never_start_runs() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["@Agent1 what do you think?"]))
            .expect(1)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;

        let run = run_of(
            intervene(&h, Selection::Auto, "@Agent0 ask around", &[])
                .await
                .unwrap(),
        )
        .unwrap();
        h.finished(&run.id).await;

        assert_eq!(
            h.count("SELECT COUNT(*) FROM messages WHERE agent_id IS NOT NULL")
                .await,
            1
        );
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
    }

    #[tokio::test]
    async fn a_repeated_automatic_intervention_returns_the_original_run() {
        let server = local_server(1).await;
        let h = harness(&server.uri(), fast_limits(), &two_agents()).await;
        let send = || {
            run(
                &h.engine,
                Intervention {
                    room_id: &h.room_id,
                    body: "@Agent1 how?",
                    selection: &Selection::Auto,
                    mention_ids: &[],
                    client_request_id: ID,
                },
            )
        };
        let first = run_of(send().await.unwrap()).unwrap();
        h.finished(&first.id).await;

        let again = run_of(send().await.unwrap()).unwrap();

        assert_eq!(again.id, first.id);
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
    }
}
