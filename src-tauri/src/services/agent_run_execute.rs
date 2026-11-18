use crate::catalogs::llm::{Location, Protocol, Provider};
use crate::domain::agent_run::{
    AgentRun, ErrorKind, LocationLimits, NextStep, TOKEN_FLUSH, after_failure,
};
use crate::domain::locale::Locale;
use crate::domain::prompt::{self, Author, Budget, HistoryMessage, Prompt, PromptInput};
use crate::repositories::{agent_runs, agents, messages, profile, room_agents, rooms};
use crate::services::agent_run_engine::{Engine, RunEvent};
use crate::system::llm_stream::{Failure, StreamRequest, Usage};
use crate::system::{anthropic_messages, chat_completions, keychain, openai_responses};
use std::sync::Arc;
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Copy)]
struct Target<'a> {
    provider: &'a Provider,
    model_max: Option<u32>,
}

enum Outcome {
    Succeeded(Usage),
    Failed(Failure),
    DeadlineExceeded,
    Cancelled,
    Aborted,
}

pub async fn run(engine: Arc<Engine>, run: AgentRun, cancel: CancellationToken) {
    let target = resolve(&engine.providers, &run.provider, &run.model);
    let location = target.map_or(Location::Cloud, |target| target.provider.location);
    let limits = *engine.limits.at(location);
    let mut deadline: Option<Instant> = None;
    loop {
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return,
            _ = reached(deadline) => return finish_failed(&engine, &run.id, ErrorKind::Timeout).await,
            permit = engine.slots(location).acquire() => permit.expect("run slots are never closed"),
        };
        let claimed = match agent_runs::claim(&engine.pool, &run.id).await {
            Ok(Some(claimed)) => claimed,
            _ => return engine.registry.remove(&run.id),
        };
        engine.registry.start_attempt(&run.id, claimed.attempts);
        engine.emit(RunEvent::Status(claimed.clone()));
        let deadline_at = *deadline.get_or_insert_with(|| Instant::now() + limits.max_duration);

        match attempt(&engine, &claimed, target, &limits, deadline_at, &cancel).await {
            Outcome::Cancelled => return,
            Outcome::Aborted => return engine.registry.remove(&run.id),
            Outcome::DeadlineExceeded => {
                return finish_failed(&engine, &run.id, ErrorKind::Timeout).await;
            }
            Outcome::Succeeded(usage) => return complete(&engine, &claimed, usage).await,
            Outcome::Failed(failure) => {
                let remaining = deadline_at.saturating_duration_since(Instant::now());
                match after_failure(
                    failure.kind,
                    failure.retry_after,
                    claimed.attempts,
                    remaining,
                    &engine.limits,
                ) {
                    NextStep::Fail(kind) => return finish_failed(&engine, &run.id, kind).await,
                    NextStep::Retry(wait) => {
                        let requeued = match agent_runs::requeue(&engine.pool, &run.id).await {
                            Ok(Some(requeued)) => requeued,
                            _ => return engine.registry.remove(&run.id),
                        };
                        engine.emit(RunEvent::Status(requeued));
                        drop(permit);
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => return,
                            _ = tokio::time::sleep(wait) => {}
                        }
                    }
                }
            }
        }
    }
}

fn resolve<'a>(providers: &'a [Provider], provider: &str, model: &str) -> Option<Target<'a>> {
    let provider = providers.iter().find(|entry| entry.key == provider)?;
    match provider.location {
        Location::Local => Some(Target {
            provider,
            model_max: None,
        }),
        Location::Cloud => provider
            .models
            .iter()
            .find(|entry| entry.key == model)
            .map(|entry| Target {
                provider,
                model_max: Some(entry.max_output_tokens),
            }),
    }
}

async fn reached(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

async fn attempt(
    engine: &Engine,
    run: &AgentRun,
    target: Option<Target<'_>>,
    limits: &LocationLimits,
    deadline: Instant,
    cancel: &CancellationToken,
) -> Outcome {
    let Some(target) = target else {
        return Outcome::Failed(ErrorKind::ModelUnavailable.into());
    };
    let prompt = match engine.registry.prompt(&run.id) {
        Some(prompt) => prompt,
        None => match freeze_context(engine, run, limits).await {
            Ok(prompt) => prompt,
            Err(outcome) => return outcome,
        },
    };
    let api_key = match target.provider.location {
        Location::Local => None,
        Location::Cloud => match read_key(&engine.pool, run).await {
            Some(key) => Some(key),
            None => return Outcome::Failed(ErrorKind::ProviderNotConfigured.into()),
        },
    };
    let request = StreamRequest {
        base_url: &target.provider.base_url,
        api_key: api_key.as_deref(),
        model: &run.model,
        max_output_tokens: engine
            .limits
            .output_tokens(target.provider.location, target.model_max),
        prompt: &prompt,
        connect_timeout: limits.connect_timeout,
        idle_timeout: limits.idle_timeout,
    };
    let registry = &engine.registry;
    let max_bytes = engine.limits.max_response_bytes;
    let mut on_text = |text: &str| {
        registry
            .push(&run.id, text, max_bytes)
            .map_err(|_| Failure::from(ErrorKind::OutputTruncated))
    };
    let protocol = target.provider.protocol;
    let output_tokens_param = target
        .provider
        .output_tokens_param
        .map_or("max_tokens", |param| param.as_str());
    let call = async {
        match protocol {
            Protocol::AnthropicMessages => anthropic_messages::stream(&request, &mut on_text).await,
            Protocol::OpenaiResponses => openai_responses::stream(&request, &mut on_text).await,
            Protocol::ChatCompletions => {
                chat_completions::stream(&request, output_tokens_param, &mut on_text).await
            }
        }
    };
    tokio::pin!(call);
    let mut ticker = tokio::time::interval(TOKEN_FLUSH);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Outcome::Cancelled,
            _ = tokio::time::sleep_until(deadline) => return Outcome::DeadlineExceeded,
            result = &mut call => {
                return match result {
                    Ok(usage) => Outcome::Succeeded(usage),
                    Err(failure) => Outcome::Failed(failure),
                };
            }
            _ = ticker.tick() => {
                if let Some(chunk) = registry.flush(&run.id) {
                    engine.emit(RunEvent::Token(chunk));
                }
            }
        }
    }
}

async fn freeze_context(
    engine: &Engine,
    run: &AgentRun,
    limits: &LocationLimits,
) -> Result<Arc<Prompt>, Outcome> {
    let input = load_context(engine, run, limits)
        .await
        .ok()
        .flatten()
        .ok_or(Outcome::Aborted)?;
    let budget = Budget {
        chars: limits.prompt_chars,
        messages: limits.history_messages,
    };
    let prompt = prompt::build(&input, budget)
        .map_err(|_| Outcome::Failed(ErrorKind::ContextTooLarge.into()))?;
    engine
        .registry
        .set_prompt(&run.id, prompt)
        .ok_or(Outcome::Aborted)
}

async fn load_context(
    engine: &Engine,
    run: &AgentRun,
    limits: &LocationLimits,
) -> Result<Option<PromptInput>, sqlx::Error> {
    let mut transaction = engine.pool.begin().await?;
    let Some(workspace_id) = rooms::workspace_id(&mut *transaction, &run.room_id).await? else {
        return Ok(None);
    };
    let Some((_, room_name, _)) =
        rooms::get(&mut *transaction, &workspace_id, &run.room_id).await?
    else {
        return Ok(None);
    };
    let Some(agent) = agents::get(&mut *transaction, &workspace_id, &run.agent_id).await? else {
        return Ok(None);
    };
    let Some(request) =
        messages::get(&mut *transaction, &run.room_id, &run.request_message_id).await?
    else {
        return Ok(None);
    };
    let participants: Vec<_> = room_agents::list(&mut *transaction, &workspace_id)
        .await?
        .into_iter()
        .filter(|(room_id, _)| *room_id == run.room_id)
        .map(|(_, participant)| participant)
        .collect();
    let history = messages::before(
        &mut *transaction,
        &run.room_id,
        &request,
        limits.history_messages as i64,
    )
    .await?;
    let locale = profile::get(&mut *transaction)
        .await?
        .map_or(Locale::En, |profile| profile.locale);
    transaction.commit().await?;

    let history = history
        .into_iter()
        .map(|message| HistoryMessage {
            author: match message.agent_id {
                None => Author::User,
                Some(id) if id == run.agent_id => Author::Own,
                Some(id) => participants
                    .iter()
                    .find(|participant| participant.id == id)
                    .map_or(Author::User, |participant| Author::Other {
                        name: participant.name.clone(),
                        role: participant.role.clone(),
                    }),
            },
            body: message.body,
        })
        .collect();
    Ok(Some(PromptInput {
        agent_name: agent.name,
        agent_role: agent.role,
        instructions: agent.instructions,
        room_name,
        participants,
        locale,
        history,
        request: request.body,
    }))
}

async fn read_key(pool: &sqlx::SqlitePool, run: &AgentRun) -> Option<String> {
    let workspace_id = rooms::workspace_id(pool, &run.room_id).await.ok()??;
    keychain::read(&format!("{workspace_id}/{}", run.provider))
        .ok()
        .flatten()
}

async fn complete(engine: &Engine, run: &AgentRun, usage: Usage) {
    let Some(text) = engine.registry.full_text(&run.id) else {
        return;
    };
    let body = text.trim();
    if body.is_empty() {
        return finish_failed(engine, &run.id, ErrorKind::InvalidResponse).await;
    }
    match commit_success(engine, run, body, usage).await {
        Ok(Some((message, done))) => {
            engine.registry.remove(&run.id);
            engine.emit(RunEvent::MessageCreated(message));
            engine.emit(RunEvent::Status(done));
        }
        _ => engine.registry.remove(&run.id),
    }
}

async fn commit_success(
    engine: &Engine,
    run: &AgentRun,
    body: &str,
    usage: Usage,
) -> Result<Option<(crate::domain::room::Message, AgentRun)>, sqlx::Error> {
    let mut transaction = engine.pool.begin_with("BEGIN IMMEDIATE").await?;
    let message = messages::insert_from_agent(
        &mut *transaction,
        &Uuid::now_v7().to_string(),
        &run.room_id,
        &run.agent_id,
        body,
    )
    .await?;
    let Some(done) = agent_runs::succeed(
        &mut *transaction,
        &run.id,
        &message.id,
        usage.input_tokens,
        usage.output_tokens,
    )
    .await?
    else {
        return Ok(None);
    };
    rooms::touch(&mut *transaction, &run.room_id, &message.created_at).await?;
    transaction.commit().await?;
    Ok(Some((message, done)))
}

async fn finish_failed(engine: &Engine, run_id: &str, kind: ErrorKind) {
    let failed = agent_runs::fail(&engine.pool, run_id, kind).await;
    engine.registry.remove(run_id);
    if let Ok(Some(failed)) = failed {
        engine.emit(RunEvent::Status(failed));
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::agent_run::{AgentRun, ErrorKind, Limits, RunStatus};
    use crate::repositories::agent_runs;
    use crate::services::agent_run_engine::testing::{
        Harness, chat, delta, fast_limits, harness, scripted_server,
    };
    use crate::services::agent_run_engine::{Engine, RunEvent};
    use crate::services::session_load::SessionError;
    use crate::services::{agent_run_cancel, message_send, room_delete, room_get, session_load};
    use crate::system::keychain;
    use std::sync::Arc;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, body_string_contains, header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const LOCAL: (&str, &str) = ("ollama", "qwen-test");

    async fn server_with(response: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .mount(&server)
            .await;
        server
    }

    async fn fails_once_then_answers(status: u16) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&server)
            .await;
        server
    }

    fn statuses(h: &Harness, run_id: &str) -> Vec<(RunStatus, i64)> {
        h.events()
            .into_iter()
            .filter_map(|event| match event {
                RunEvent::Status(run) if run.id == run_id => Some((run.status, run.attempts)),
                _ => None,
            })
            .collect()
    }

    fn task(body: &[u8]) -> String {
        let text = String::from_utf8_lossy(body);
        ["Task A", "Task B", "Task C"]
            .into_iter()
            .filter_map(|task| text.rfind(task).map(|position| (position, task)))
            .max()
            .map_or("?", |(_, task)| task)
            .to_string()
    }

    async fn agent_messages(h: &Harness) -> i64 {
        h.count("SELECT COUNT(*) FROM messages WHERE agent_id IS NOT NULL")
            .await
    }

    #[tokio::test]
    async fn a_local_agent_answers_in_the_room_with_its_tokens() {
        let server = server_with(chat(&["Hel", "lo"])).await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Say hello").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.status, RunStatus::Succeeded);
        assert_eq!((done.input_tokens, done.output_tokens), (Some(11), Some(2)));
        assert_eq!(done.attempts, 1);
        let detail = room_get::run(&h.engine.pool, &h.engine.registry, &h.room_id)
            .await
            .unwrap();
        let reply = detail.messages.last().unwrap();
        assert_eq!(reply.body, "Hello");
        assert_eq!(reply.agent_id.as_deref(), Some(h.agents[0].as_str()));
        assert_eq!(done.response_message_id.as_deref(), Some(reply.id.as_str()));
        assert_eq!(detail.room.updated_at, reply.created_at);
        assert_eq!(detail.runs, std::slice::from_ref(&done));
        assert!(detail.partials.is_empty());
        assert_eq!(
            statuses(&h, &sent.run.id),
            [
                (RunStatus::Queued, 0),
                (RunStatus::Running, 1),
                (RunStatus::Succeeded, 1)
            ]
        );
        let events = h.events();
        let reply_event = events
            .iter()
            .position(|event| matches!(event, RunEvent::MessageCreated(message) if message.id == reply.id))
            .unwrap();
        assert_eq!(events[reply_event + 1], RunEvent::Status(done));
    }

    #[tokio::test]
    async fn sends_the_frozen_context_with_labels_and_the_local_output_cap() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(serde_json::json!({
                "model": "qwen-test",
                "max_tokens": 2048,
                "stream": true,
            })))
            .respond_with(chat(&["ok"]))
            .expect(1)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL, LOCAL]).await;
        message_send::run(&h.engine.pool, &h.room_id, "Hi all")
            .await
            .unwrap();

        let sent = h.send(0, "Status?").await.unwrap();
        h.finished(&sent.run.id).await;

        let body: serde_json::Value = server.received_requests().await.unwrap()[0]
            .body_json()
            .unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert!(
            messages[0]["content"]
                .as_str()
                .unwrap()
                .contains("You are Agent0")
        );
        assert_eq!(messages[1]["content"], "Hi all\n\nStatus?");
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn a_reply_longer_than_4000_characters_is_saved_whole() {
        let long = "x".repeat(5000);
        let server = server_with(chat(&[&long])).await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Write a lot").await.unwrap();
        h.finished(&sent.run.id).await;

        let body: String =
            sqlx::query_scalar("SELECT body FROM messages WHERE agent_id IS NOT NULL")
                .fetch_one(&h.engine.pool)
                .await
                .unwrap();
        assert_eq!(body.len(), 5000);
    }

    #[tokio::test]
    async fn retries_with_exactly_the_same_context_even_if_the_room_and_agent_change() {
        let server = fails_once_then_answers(503).await;
        let mut limits = fast_limits();
        limits.backoff = [Duration::from_millis(400), Duration::from_millis(400)];
        let h = harness(&server.uri(), limits, &[LOCAL]).await;

        let sent = h.send(0, "Task A").await.unwrap();
        h.wait_until(&sent.run.id, |run| {
            run.status == RunStatus::Queued && run.attempts == 1
        })
        .await;
        message_send::run(&h.engine.pool, &h.room_id, "A late message")
            .await
            .unwrap();
        sqlx::query("UPDATE agents SET instructions = 'Changed', name = 'Renamed'")
            .execute(&h.engine.pool)
            .await
            .unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!((done.status, done.attempts), (RunStatus::Succeeded, 2));
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].body, requests[1].body);
        assert!(!String::from_utf8_lossy(&requests[1].body).contains("A late message"));
        assert_eq!(
            statuses(&h, &sent.run.id),
            [
                (RunStatus::Queued, 0),
                (RunStatus::Running, 1),
                (RunStatus::Queued, 1),
                (RunStatus::Running, 2),
                (RunStatus::Succeeded, 2)
            ]
        );
    }

    #[tokio::test]
    async fn a_run_waiting_for_a_retry_frees_the_local_slot_but_keeps_its_agent_busy() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("Task A"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&server)
            .await;
        let mut limits = fast_limits();
        limits.backoff = [Duration::from_millis(1500), Duration::from_millis(1500)];
        let h = harness(&server.uri(), limits, &[LOCAL, ("lmstudio", "other-model")]).await;

        let a = h.send(0, "Task A").await.unwrap();
        h.wait_until(&a.run.id, |run| {
            run.status == RunStatus::Queued && run.attempts == 1
        })
        .await;
        let b = h.send(1, "Task B").await.unwrap();
        let b_done = h.finished(&b.run.id).await;

        assert_eq!(b_done.status, RunStatus::Succeeded);
        assert_eq!(h.run(&a.run.id).await.status, RunStatus::Queued);
        assert!(matches!(
            h.send(0, "Again").await,
            Err(SessionError::AgentBusy)
        ));
        assert_eq!(h.finished(&a.run.id).await.status, RunStatus::Succeeded);
    }

    #[tokio::test]
    async fn serves_runs_in_arrival_order_and_sends_retries_to_the_back() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("Task A"))
            .respond_with(ResponseTemplate::new(503).set_delay(Duration::from_millis(300)))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]).set_delay(Duration::from_millis(50)))
            .mount(&server)
            .await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[LOCAL, LOCAL, ("lmstudio", "m")],
        )
        .await;

        let a = h.send(0, "Task A").await.unwrap();
        h.wait_until(&a.run.id, |run| run.status == RunStatus::Running)
            .await;
        let b = h.send(1, "Task B").await.unwrap();
        let c = h.send(2, "Task C").await.unwrap();
        for run in [&a.run, &b.run, &c.run] {
            assert_eq!(h.finished(&run.id).await.status, RunStatus::Succeeded);
        }

        let order: Vec<String> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| task(&request.body))
            .collect();
        assert_eq!(order, ["Task A", "Task B", "Task C", "Task A"]);
    }

    #[tokio::test]
    async fn four_cloud_runs_go_at_once_while_local_runs_share_one_slot() {
        let server = server_with(chat(&["ok"]).set_delay(Duration::from_millis(400))).await;
        let mut limits = fast_limits();
        limits.cloud.concurrency = 4;
        let h = harness(&server.uri(), limits, &[LOCAL, ("lmstudio", "m"), LOCAL]).await;

        let runs = [
            h.send(0, "one").await.unwrap(),
            h.send(1, "two").await.unwrap(),
            h.send(2, "three").await.unwrap(),
        ];
        tokio::time::sleep(Duration::from_millis(200)).await;
        let running: Vec<RunStatus> = {
            let mut statuses = Vec::new();
            for sent in &runs {
                statuses.push(h.run(&sent.run.id).await.status);
            }
            statuses
        };

        assert_eq!(
            running
                .iter()
                .filter(|status| **status == RunStatus::Running)
                .count(),
            1
        );
        for sent in &runs {
            assert_eq!(h.finished(&sent.run.id).await.status, RunStatus::Succeeded);
        }
        assert_eq!(h.engine.limits.cloud.concurrency, 4);
        assert_eq!(Limits::default().local.concurrency, 1);
        assert_eq!(Limits::default().cloud.concurrency, 4);
    }

    #[tokio::test]
    async fn permanent_errors_fail_at_once_without_a_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("secret provider detail"))
            .expect(1)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::ProviderAuthFailed));
        assert_eq!(done.attempts, 1);
        assert_eq!(agent_messages(&h).await, 0);
        assert!(!format!("{:?}", h.events()).contains("secret provider detail"));
    }

    #[tokio::test]
    async fn transient_errors_give_up_after_three_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .expect(3)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::ProviderUnavailable));
        assert_eq!(done.attempts, 3);
    }

    #[tokio::test]
    async fn honours_retry_after_and_fails_when_it_is_too_long() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0.2"))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::RateLimited));
        assert_eq!(done.attempts, 2);
    }

    #[tokio::test]
    async fn the_maximum_duration_counts_backoff_and_ends_in_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let mut limits = fast_limits();
        limits.local.max_duration = Duration::from_millis(500);
        limits.backoff = [Duration::from_secs(2), Duration::from_secs(2)];
        let h = harness(&server.uri(), limits, &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::Timeout));
        assert_eq!(done.attempts, 1);
    }

    #[tokio::test]
    async fn events_without_text_cannot_outlive_the_maximum_duration() {
        let ping = "data: {\"choices\":[]}\n\n".to_string();
        let script = (0..40)
            .map(|_| (Duration::from_millis(50), ping.clone()))
            .collect();
        let uri = scripted_server(script).await;
        let mut limits = fast_limits();
        limits.local.max_duration = Duration::from_millis(600);
        limits.local.idle_timeout = Duration::from_millis(300);
        let h = harness(&uri, limits, &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::Timeout));
        assert_eq!(done.attempts, 1);
    }

    #[tokio::test]
    async fn a_context_that_does_not_fit_fails_without_calling_the_provider() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .expect(0)
            .mount(&server)
            .await;
        let mut limits = fast_limits();
        limits.local.prompt_chars = 50;
        let h = harness(&server.uri(), limits, &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::ContextTooLarge));
        assert_eq!(done.attempts, 1);
    }

    #[tokio::test]
    async fn the_safety_limit_cuts_the_stream_and_saves_nothing() {
        let server = server_with(chat(&["0123456789", "0123456789"])).await;
        let mut limits = fast_limits();
        limits.max_response_bytes = 15;
        let h = harness(&server.uri(), limits, &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.error_kind, Some(ErrorKind::OutputTruncated));
        assert_eq!(done.attempts, 1);
        assert_eq!(agent_messages(&h).await, 0);
    }

    #[tokio::test]
    async fn an_empty_reply_is_an_invalid_response() {
        let server = server_with(chat(&["  \n "])).await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        let sent = h.send(0, "Hi").await.unwrap();

        assert_eq!(
            h.finished(&sent.run.id).await.error_kind,
            Some(ErrorKind::InvalidResponse)
        );
        assert_eq!(agent_messages(&h).await, 0);
    }

    #[tokio::test]
    async fn a_retired_model_is_unavailable_without_calling_the_provider() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .expect(0)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[("openai", "gpt-6.1-sol")]).await;
        sqlx::query("UPDATE agents SET model = 'gpt-5'")
            .execute(&h.engine.pool)
            .await
            .unwrap();

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        assert_eq!(done.model, "gpt-5");
        assert_eq!(done.error_kind, Some(ErrorKind::ModelUnavailable));
        let agent_model: String = sqlx::query_scalar("SELECT model FROM agents")
            .fetch_one(&h.engine.pool)
            .await
            .unwrap();
        assert_eq!(agent_model, "gpt-5");
    }

    #[tokio::test]
    async fn a_cloud_provider_without_a_key_is_not_configured_and_local_agents_still_work() {
        let server = server_with(chat(&["ok"])).await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("anthropic", "claude-haiku-5-5"), LOCAL],
        )
        .await;

        let cloud = h.send(0, "Hi").await.unwrap();
        let local = h.send(1, "Hi").await.unwrap();

        assert_eq!(
            h.finished(&cloud.run.id).await.error_kind,
            Some(ErrorKind::ProviderNotConfigured)
        );
        assert_eq!(h.finished(&local.run.id).await.status, RunStatus::Succeeded);
        assert_eq!(h.finished(&cloud.run.id).await.provider, "anthropic");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_cloud_run_reads_its_key_from_the_keychain_and_caps_the_output() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("authorization", "Bearer sk-groq-test"))
            .and(body_partial_json(serde_json::json!({
                "model": "openai/gpt-oss-120b",
                "max_completion_tokens": 16000,
            })))
            .respond_with(chat(&["ok"]))
            .expect(1)
            .mount(&server)
            .await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("groq", "openai/gpt-oss-120b")],
        )
        .await;
        let workspace_id = session_load::require(&h.engine.pool)
            .await
            .unwrap()
            .current_workspace_id;
        let secret = format!("{workspace_id}/groq");
        keychain::save(&secret, "sk-groq-test").unwrap();

        let sent = h.send(0, "Hi").await.unwrap();
        let done = h.finished(&sent.run.id).await;

        keychain::remove(&secret).unwrap();
        assert_eq!(done.status, RunStatus::Succeeded);
        assert!(!format!("{:?}", h.events()).contains("sk-groq-test"));
    }

    async fn streaming_harness() -> (Harness, AgentRun) {
        let uri = scripted_server(vec![(Duration::ZERO, delta("Hel"))]).await;
        let h = harness(&uri, fast_limits(), &[LOCAL, ("lmstudio", "m")]).await;
        let sent = h.send(0, "Say hello").await.unwrap();
        for _ in 0..500 {
            if h.events()
                .iter()
                .any(|event| matches!(event, RunEvent::Token(_)))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        (h, sent.run)
    }

    #[tokio::test]
    async fn streams_tokens_and_shows_the_snapshot_only_in_its_room() {
        let (h, run) = streaming_harness().await;

        let token = h
            .events()
            .into_iter()
            .find_map(|event| match event {
                RunEvent::Token(chunk) => Some(chunk),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            (token.attempt, token.seq, token.text.as_str()),
            (1, 1, "Hel")
        );
        assert_eq!(token.room_id, h.room_id);
        let detail = room_get::run(&h.engine.pool, &h.engine.registry, &h.room_id)
            .await
            .unwrap();
        assert_eq!(detail.partials.len(), 1);
        assert_eq!(detail.partials[0].text, "Hel");
        assert_eq!(detail.runs[0].status, RunStatus::Running);
        let other = crate::services::room_create::run(&h.engine.pool, "Other", vec![])
            .await
            .unwrap();
        let other_detail = room_get::run(&h.engine.pool, &h.engine.registry, &other.id)
            .await
            .unwrap();
        assert!(other_detail.partials.is_empty());
        let client_request_id =
            sqlx::query_scalar::<_, String>("SELECT client_request_id FROM agent_runs")
                .fetch_one(&h.engine.pool)
                .await
                .unwrap();
        let repeated = h
            .send_with(0, "Say hello", &client_request_id)
            .await
            .unwrap();
        assert_eq!(
            repeated.partial.map(|partial| partial.text).as_deref(),
            Some("Hel")
        );
        agent_run_cancel::run(&h.engine, &run.id).await.unwrap();
    }

    #[tokio::test]
    async fn cancelling_during_the_stream_stops_it_saves_nothing_and_frees_the_slot() {
        let (h, run) = streaming_harness().await;

        agent_run_cancel::run(&h.engine, &run.id).await.unwrap();
        agent_run_cancel::run(&h.engine, &run.id).await.unwrap();
        let done = h.finished(&run.id).await;
        let next = h.send(1, "Next").await.unwrap();

        assert_eq!(done.error_kind, Some(ErrorKind::Cancelled));
        assert_eq!(h.finished(&next.run.id).await.status, RunStatus::Succeeded);
        assert_eq!(agent_messages(&h).await, 1);
        assert!(h.engine.registry.partial(&run.id).is_none());
        let cancelled_events = h
            .events()
            .into_iter()
            .filter(|event| matches!(event, RunEvent::Status(status) if status.id == run.id && status.status == RunStatus::Failed))
            .count();
        assert_eq!(cancelled_events, 1);
        assert!(h.send(0, "Again").await.is_ok());
    }

    #[tokio::test]
    async fn cancelling_a_queued_run_never_calls_the_provider() {
        let (h, streaming) = streaming_harness().await;
        let queued = h.send(1, "Wait for it").await.unwrap();

        agent_run_cancel::run(&h.engine, &queued.run.id)
            .await
            .unwrap();
        agent_run_cancel::run(&h.engine, &streaming.id)
            .await
            .unwrap();

        let done = h.finished(&queued.run.id).await;
        assert_eq!(
            (done.error_kind, done.attempts),
            (Some(ErrorKind::Cancelled), 0)
        );
        assert!(done.started_at.is_none());
    }

    #[tokio::test]
    async fn cancelling_during_backoff_wakes_the_run_and_it_never_retries() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let mut limits = fast_limits();
        limits.backoff = [Duration::from_secs(5), Duration::from_secs(5)];
        let h = harness(&server.uri(), limits, &[LOCAL]).await;
        let sent = h.send(0, "Hi").await.unwrap();
        h.wait_until(&sent.run.id, |run| {
            run.status == RunStatus::Queued && run.attempts == 1
        })
        .await;

        agent_run_cancel::run(&h.engine, &sent.run.id)
            .await
            .unwrap();

        let done = h.finished(&sent.run.id).await;
        assert_eq!(
            (done.error_kind, done.attempts),
            (Some(ErrorKind::Cancelled), 1)
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(h.run(&sent.run.id).await.status, RunStatus::Failed);
    }

    #[tokio::test]
    async fn cancel_racing_with_success_never_leaves_a_message_of_a_cancelled_run() {
        let server = server_with(chat(&["ok"])).await;
        let h = harness(&server.uri(), fast_limits(), &[LOCAL]).await;

        for round in 0..15u64 {
            let sent = h.send(0, &format!("round {round}")).await.unwrap();
            tokio::time::sleep(Duration::from_millis(round % 5 * 3)).await;
            agent_run_cancel::run(&h.engine, &sent.run.id)
                .await
                .unwrap();
            let done = h.finished(&sent.run.id).await;
            let replies: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_runs JOIN messages ON messages.id = agent_runs.response_message_id WHERE agent_runs.id = ?")
                .bind(&sent.run.id)
                .fetch_one(&h.engine.pool)
                .await
                .unwrap();
            match done.status {
                RunStatus::Succeeded => assert_eq!(replies, 1),
                _ => assert_eq!(done.error_kind, Some(ErrorKind::Cancelled)),
            }
        }
        let succeeded = h
            .count("SELECT COUNT(*) FROM agent_runs WHERE status = 'succeeded'")
            .await;
        assert_eq!(agent_messages(&h).await, succeeded);
    }

    #[tokio::test]
    async fn deleting_the_room_cancels_its_runs_and_clears_their_buffers() {
        let (h, run) = streaming_harness().await;

        room_delete::run(&h.engine.pool, &h.engine.registry, &h.room_id)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(h.engine.registry.partial(&run.id).is_none());
        assert_eq!(h.count("SELECT COUNT(*) FROM agent_runs").await, 0);
        assert_eq!(h.count("SELECT COUNT(*) FROM messages").await, 0);
    }

    #[tokio::test]
    async fn cancelling_from_another_workspace_changes_nothing() {
        let (h, run) = streaming_harness().await;
        let first = session_load::require(&h.engine.pool)
            .await
            .unwrap()
            .current_workspace_id;
        crate::services::workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();

        agent_run_cancel::run(&h.engine, &run.id).await.unwrap();

        assert_eq!(h.run(&run.id).await.status, RunStatus::Running);
        assert!(h.engine.registry.partial(&run.id).is_some());
        crate::services::workspace_switch::run(&h.engine.pool, &first)
            .await
            .unwrap();
        agent_run_cancel::run(&h.engine, &run.id).await.unwrap();
        assert_eq!(
            h.finished(&run.id).await.error_kind,
            Some(ErrorKind::Cancelled)
        );
    }

    #[tokio::test]
    #[ignore = "needs MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL pointing at a running local provider"]
    async fn a_real_local_agent_answers_in_the_room() {
        let base_url = std::env::var("MEELTING_LOCAL_BASE_URL").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();
        let h = harness(&base_url, Limits::default(), &[("ollama", &model)]).await;

        let first = h
            .send(0, "Reply with one short sentence: what is 2 + 2?")
            .await
            .unwrap();
        let first = h.finished(&first.run.id).await;
        let second = h
            .send(0, "Now repeat your previous answer in Spanish.")
            .await
            .unwrap();
        let second = h.finished(&second.run.id).await;

        let tokens = h
            .events()
            .iter()
            .filter(|event| matches!(event, RunEvent::Token(_)))
            .count();
        let replies: Vec<String> = sqlx::query_scalar(
            "SELECT body FROM messages WHERE agent_id IS NOT NULL ORDER BY created_at",
        )
        .fetch_all(&h.engine.pool)
        .await
        .unwrap();
        println!(
            "first: {first:?}\nsecond: {second:?}\ntoken events: {tokens}\nreplies: {replies:?}"
        );
        assert_eq!(
            (first.status, second.status),
            (RunStatus::Succeeded, RunStatus::Succeeded)
        );
        assert_eq!(replies.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_event_is_emitted_after_its_commit() {
        let server = fails_once_then_answers(503).await;
        let (pool, _dir) = crate::repositories::database::open_for_test().await;
        let checked = pool.clone();
        let mismatches: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let recorded = mismatches.clone();
        let engine = Engine::new(
            pool.clone(),
            crate::services::agent_run_engine::testing::providers_at(&server.uri()),
            fast_limits(),
            Arc::new(move |event| {
                let pool = checked.clone();
                let problem = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        match event {
                            RunEvent::Status(run) => {
                                let stored: Option<(String, i64)> = sqlx::query_as(
                                    "SELECT status, attempts FROM agent_runs WHERE id = ?",
                                )
                                .bind(&run.id)
                                .fetch_optional(&pool)
                                .await
                                .unwrap();
                                let expected =
                                    (format!("{:?}", run.status).to_lowercase(), run.attempts);
                                (stored != Some(expected.clone()))
                                    .then(|| format!("{expected:?} vs {stored:?}"))
                            }
                            RunEvent::MessageCreated(message) => {
                                let exists: bool = sqlx::query_scalar(
                                    "SELECT EXISTS (SELECT 1 FROM messages WHERE id = ?)",
                                )
                                .bind(&message.id)
                                .fetch_one(&pool)
                                .await
                                .unwrap();
                                (!exists).then(|| format!("message {}", message.id))
                            }
                            RunEvent::Token(_) => None,
                        }
                    })
                });
                if let Some(problem) = problem {
                    recorded.lock().unwrap().push(problem);
                }
            }),
        );
        crate::services::first_launch::run(&pool, crate::domain::locale::Locale::En, "Acme")
            .await
            .unwrap();
        let team = crate::services::team_create::run(&pool, "General")
            .await
            .unwrap()[0]
            .id
            .clone();
        let agent = crate::services::agent_create::run(
            &pool,
            crate::domain::team::AgentDraft {
                team_id: team,
                template_key: None,
                name: "Ana".into(),
                role: "QA Engineer".into(),
                description: String::new(),
                instructions: String::new(),
                provider: "ollama".into(),
                model: "qwen-test".into(),
            },
        )
        .await
        .unwrap()[0]
            .agents[0]
            .id
            .clone();
        let room = crate::services::room_create::run(&pool, "Daily", vec![agent.clone()])
            .await
            .unwrap();

        let sent = crate::services::intervention_send::run(
            &engine,
            crate::services::intervention_send::Intervention {
                room_id: &room.id,
                body: "Hi",
                selection: &crate::domain::recipient::Selection::Agent(agent.clone()),
                mention_ids: &[],
                client_request_id: "0196f3a0-0000-7000-8000-000000000001",
            },
        )
        .await
        .unwrap();
        let crate::services::intervention_send::InterventionSent::Sent {
            run: Some(sent_run),
            ..
        } = sent
        else {
            panic!("expected a run");
        };
        for _ in 0..500 {
            let status: String = sqlx::query_scalar("SELECT status FROM agent_runs WHERE id = ?")
                .bind(&sent_run.id)
                .fetch_one(&pool)
                .await
                .unwrap();
            if status == "succeeded" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            mismatches.lock().unwrap().is_empty(),
            "{:?}",
            mismatches.lock().unwrap()
        );
        let _ = agent_runs::for_room(&pool, &room.id, 10).await.unwrap();
    }
}
