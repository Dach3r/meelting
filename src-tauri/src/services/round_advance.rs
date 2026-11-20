use crate::domain::agent_run::AgentRun;
use crate::repositories::agent_runs::{self, NewRun};
use crate::repositories::{agents, messages, room_agents, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(engine: &Arc<Engine>, room_id: &str) {
    while let Some(agent_id) = engine.registry.next_in_round(room_id) {
        if let Ok(Some(next)) = queue(engine, room_id, &agent_id).await {
            engine.emit(RunEvent::Status(next.clone()));
            engine.submit(next, true);
            return;
        }
    }
}

async fn queue(
    engine: &Engine,
    room_id: &str,
    agent_id: &str,
) -> Result<Option<AgentRun>, sqlx::Error> {
    let mut transaction = engine.pool.begin_with("BEGIN IMMEDIATE").await?;
    let Some(workspace_id) = rooms::workspace_id(&mut *transaction, room_id).await? else {
        return Ok(None);
    };
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let participates = room_agents::list(&mut *transaction, &workspace_id)
        .await?
        .iter()
        .any(|(room, participant)| room == room_id && participant.id == agent_id);
    if !participates || agent_runs::has_active(&mut *transaction, room_id, agent_id).await? {
        return Ok(None);
    }
    let Some(agent) = agents::get(&mut *transaction, &workspace_id, agent_id).await? else {
        return Ok(None);
    };
    let Some(last) = messages::recent(&mut *transaction, room_id, 1).await?.pop() else {
        return Ok(None);
    };
    let run = agent_runs::insert_queued(
        &mut *transaction,
        &NewRun {
            id: &Uuid::now_v7().to_string(),
            room_id,
            agent_id,
            request_message_id: &last.id,
            client_request_id: &Uuid::now_v7().to_string(),
            provider: &agent.provider,
            model: &agent.model,
            created_at: &last.created_at,
        },
    )
    .await?;
    transaction.commit().await?;
    Ok(Some(run))
}

#[cfg(test)]
mod tests {
    use crate::domain::agent_run::{ErrorKind, RunStatus};
    use crate::services::agent_run_engine::testing::{
        chat, delta, fast_limits, harness, scripted_server,
    };
    use crate::services::{intervention_stop, room_get};
    use serde_json::json;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn answers(server: &MockServer, model: &str, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "model": model })))
            .respond_with(response)
            .mount(server)
            .await;
    }

    fn local(models: &[&'static str]) -> Vec<(&'static str, &'static str)> {
        models.iter().map(|model| ("ollama", *model)).collect()
    }

    #[tokio::test]
    async fn the_team_answers_in_turn_seeing_the_previous_replies_and_may_pass() {
        let server = MockServer::start().await;
        answers(&server, "m0", chat(&["Idea A"])).await;
        answers(&server, "m1", chat(&["PASS"])).await;
        answers(&server, "m2", chat(&["Idea C"])).await;
        let h = harness(&server.uri(), fast_limits(), &local(&["m0", "m1", "m2"])).await;

        h.send_team("Which stack?").await;
        let runs = h.settled_runs(3).await;

        let statuses: Vec<_> = runs.iter().map(|run| run.status).collect();
        assert_eq!(
            statuses,
            [
                RunStatus::Succeeded,
                RunStatus::Passed,
                RunStatus::Succeeded
            ]
        );
        let agents: Vec<_> = runs.iter().map(|run| run.agent_id.as_str()).collect();
        assert_eq!(
            agents,
            h.agents.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert!(runs[1].response_message_id.is_none());
        assert_eq!(runs[1].error_kind, None);
        let detail = room_get::run(&h.engine.pool, &h.engine.registry, &h.room_id)
            .await
            .unwrap();
        let bodies: Vec<_> = detail.messages.iter().map(|m| m.body.as_str()).collect();
        assert_eq!(bodies, ["Which stack?", "Idea A", "Idea C"]);
        assert_eq!(runs[2].request_message_id, detail.messages[1].id);
        let third = server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .map(|request| String::from_utf8_lossy(&request.body).to_string())
            .find(|body| body.contains("\"m2\""))
            .unwrap();
        assert!(third.contains("[Agent0 · QA Engineer] Idea A"));
        assert!(third.contains("exactly PASS"));
    }

    #[tokio::test]
    async fn a_round_never_exceeds_the_budget_of_four() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&server)
            .await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &local(&["m", "m", "m", "m", "m", "m"]),
        )
        .await;

        h.send_team("Everyone?").await;
        let runs = h.settled_runs(4).await;

        assert!(runs.iter().all(|run| run.status == RunStatus::Succeeded));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 4);
    }

    #[tokio::test]
    async fn a_failed_turn_does_not_stop_the_round() {
        let server = MockServer::start().await;
        answers(&server, "bad", ResponseTemplate::new(400)).await;
        answers(&server, "good", chat(&["Fine"])).await;
        let h = harness(&server.uri(), fast_limits(), &local(&["bad", "good"])).await;

        h.send_team("Hello team").await;
        let runs = h.settled_runs(2).await;

        assert_eq!(runs[0].status, RunStatus::Failed);
        assert_eq!(runs[0].error_kind, Some(ErrorKind::ProviderRejected));
        assert_eq!(runs[1].status, RunStatus::Succeeded);
    }

    #[tokio::test]
    async fn a_reply_that_is_just_pass_outside_a_round_is_a_normal_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["PASS"]))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &local(&["m"])).await;

        let sent = h.send(0, "Say PASS").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.status, RunStatus::Succeeded);
        assert!(done.response_message_id.is_some());
    }

    #[tokio::test]
    async fn stopping_cancels_the_active_turn_and_discards_the_pending_ones() {
        let url = scripted_server(vec![(Duration::ZERO, delta("thinking"))]).await;
        let h = harness(&url, fast_limits(), &local(&["m", "m", "m"])).await;

        let sent = h.send_team("Hello team").await;
        h.wait_until(&sent.run.id, |run| run.status == RunStatus::Running)
            .await;
        intervention_stop::run(&h.engine, &h.room_id).await.unwrap();

        let runs = h.settled_runs(1).await;
        assert_eq!(runs[0].error_kind, Some(ErrorKind::Cancelled));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 1);
        assert_eq!(h.engine.registry.next_in_round(&h.room_id), None);
    }

    #[tokio::test]
    async fn a_new_intervention_discards_the_pending_turns_of_the_previous_round() {
        let url = scripted_server(vec![(Duration::ZERO, delta("thinking"))]).await;
        let h = harness(&url, fast_limits(), &local(&["m", "m", "m"])).await;

        let first = h.send_team("Hello team").await;
        h.wait_until(&first.run.id, |run| run.status == RunStatus::Running)
            .await;
        h.send(2, "A word with you").await.unwrap();

        assert_eq!(h.engine.registry.next_in_round(&h.room_id), None);
    }
}
