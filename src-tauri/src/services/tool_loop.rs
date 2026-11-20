use crate::catalogs::llm::Provider;
use crate::domain::agent_run::{AgentRun, ErrorKind};
use crate::domain::tools::{self, Budget, Prepared, Request, ToolContext};
use crate::repositories::tool_calls::{self, Closing, NewCall, NewRejected};
use crate::services::agent_run_engine::Engine;
use crate::services::{tool_environment, tool_files, tool_git};
use crate::system::chat_completions::{RawCall, assistant_message, tool_message};
use crate::system::llm_call;
use crate::system::llm_stream::{Failure, StreamRequest, Usage};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_PROVIDER_CALL_ID_BYTES: usize = 128;

pub(crate) enum Outcome {
    Final { text: String, usage: Usage },
    Failed { failure: Failure, after_tools: bool },
    DeadlineExceeded,
    Cancelled,
    Aborted,
}

pub(crate) struct Session<'a> {
    pub engine: &'a Engine,
    pub run: &'a AgentRun,
    pub provider: &'a Provider,
    pub request: &'a StreamRequest<'a>,
    pub context: &'a ToolContext,
    pub deadline: Instant,
    pub cancel: &'a CancellationToken,
}

#[derive(Default)]
struct State {
    exchange: Vec<Value>,
    budget: Budget,
    usage: Usage,
    position: i64,
    seen: HashSet<String>,
    executed: bool,
}

impl State {
    fn failed(&self, kind: ErrorKind) -> Outcome {
        self.attribute(Outcome::Failed {
            failure: kind.into(),
            after_tools: false,
        })
    }

    fn attribute(&self, outcome: Outcome) -> Outcome {
        match outcome {
            Outcome::Failed { failure, .. } => Outcome::Failed {
                failure,
                after_tools: self.executed,
            },
            other => other,
        }
    }
}

struct Call {
    id: String,
    name: String,
    prepared: Prepared,
}

pub(crate) async fn run(session: &Session<'_>) -> Outcome {
    match drive(session).await {
        Ok((text, usage)) => Outcome::Final { text, usage },
        Err(outcome) => outcome,
    }
}

async fn drive(session: &Session<'_>) -> Result<(String, Usage), Outcome> {
    let definitions = tools::definitions();
    let mut state = State::default();
    loop {
        state
            .budget
            .start_iteration()
            .map_err(|kind| state.failed(kind))?;
        let (usage, text, raw_calls) = model_turn(session, &definitions, &state.exchange)
            .await
            .map_err(|outcome| state.attribute(outcome))?;
        state.usage = Usage {
            input_tokens: sum(state.usage.input_tokens, usage.input_tokens),
            output_tokens: sum(state.usage.output_tokens, usage.output_tokens),
        };
        if raw_calls.is_empty() {
            return Ok((text, state.usage));
        }
        let calls = validated(&mut state, raw_calls)?;
        state
            .budget
            .admit_calls(calls.len())
            .map_err(|kind| state.failed(kind))?;
        state.exchange.push(assistant_of(&text, &calls));
        for call in &calls {
            let result = execute(session, &mut state, call).await?;
            state
                .budget
                .add_output(result.len())
                .map_err(|kind| state.failed(kind))?;
            state.exchange.push(tool_message(&call.id, &result));
        }
    }
}

fn sum(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (None, None) => None,
        (left, right) => Some(left.unwrap_or(0).saturating_add(right.unwrap_or(0))),
    }
}

async fn model_turn(
    session: &Session<'_>,
    definitions: &[tools::Definition],
    exchange: &[Value],
) -> Result<(Usage, String, Vec<RawCall>), Outcome> {
    let mut text = String::new();
    let max_bytes = session.engine.limits.max_response_bytes;
    let result = {
        let mut on_text = |chunk: &str| {
            if text.len() + chunk.len() > max_bytes {
                Err(Failure::from(ErrorKind::OutputTruncated))
            } else {
                text.push_str(chunk);
                Ok(())
            }
        };
        let call = llm_call::stream_turn(
            session.provider,
            session.request,
            definitions,
            exchange,
            &mut on_text,
        );
        tokio::select! {
            biased;
            _ = session.cancel.cancelled() => return Err(Outcome::Cancelled),
            _ = tokio::time::sleep_until(session.deadline) => return Err(Outcome::DeadlineExceeded),
            result = call => result,
        }
    };
    match result {
        Ok((usage, calls)) => Ok((usage, text, calls)),
        Err(failure) => Err(Outcome::Failed {
            failure,
            after_tools: false,
        }),
    }
}

fn validated(state: &mut State, raw_calls: Vec<RawCall>) -> Result<Vec<Call>, Outcome> {
    let invalid = || state.failed(ErrorKind::InvalidResponse);
    let mut calls = Vec::with_capacity(raw_calls.len());
    let mut batch = HashSet::new();
    for raw in raw_calls {
        let (Some(id), Some(name)) = (raw.id, raw.name) else {
            return Err(invalid());
        };
        if id.len() > MAX_PROVIDER_CALL_ID_BYTES
            || !id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
            })
            || state.seen.contains(&id)
            || !batch.insert(id.clone())
        {
            return Err(invalid());
        }
        let prepared = tools::prepare(&name, &raw.arguments, raw.oversized);
        calls.push(Call { id, name, prepared });
    }
    state.seen.extend(batch);
    Ok(calls)
}

fn assistant_of(text: &str, calls: &[Call]) -> Value {
    let calls: Vec<(&str, &str, &str)> = calls
        .iter()
        .map(|call| {
            let arguments = match &call.prepared {
                Prepared::Audited { arguments, .. } => arguments.as_str(),
                Prepared::Rejected { .. } => "{}",
            };
            (call.id.as_str(), call.name.as_str(), arguments)
        })
        .collect();
    assistant_message(text, &calls)
}

fn error_text(code: &str) -> String {
    format!("error: {code}\n")
}

async fn execute(session: &Session<'_>, state: &mut State, call: &Call) -> Result<String, Outcome> {
    state.position += 1;
    let row_id = Uuid::now_v7().to_string();
    let pool = &session.engine.pool;
    let (arguments, request) = match &call.prepared {
        Prepared::Rejected { named } => {
            tool_calls::insert_rejected(
                pool,
                &NewRejected {
                    id: &row_id,
                    run_id: &session.run.id,
                    room_id: &session.run.room_id,
                    workspace_id: &session.context.workspace_id,
                    task_id: &session.context.task_id,
                    worktree_id: &session.context.worktree_id,
                    iteration: i64::from(state.budget.iteration()),
                    position: state.position,
                    provider_call_id: &call.id,
                    tool: named.then_some(call.name.as_str()),
                },
            )
            .await
            .map_err(|_| Outcome::Aborted)?;
            state.executed = true;
            return Ok(error_text("invalid_arguments"));
        }
        Prepared::Audited { arguments, request } => (arguments, request),
    };
    tool_calls::insert_running(
        pool,
        &NewCall {
            id: &row_id,
            run_id: &session.run.id,
            room_id: &session.run.room_id,
            workspace_id: &session.context.workspace_id,
            task_id: &session.context.task_id,
            worktree_id: &session.context.worktree_id,
            iteration: i64::from(state.budget.iteration()),
            position: state.position,
            provider_call_id: &call.id,
            tool: &call.name,
            arguments,
        },
    )
    .await
    .map_err(|_| Outcome::Aborted)?;
    state.executed = true;

    let request = match request {
        Ok(request) => request.clone(),
        Err(code) => {
            close(pool, &row_id, &Closing::Failed { code }).await?;
            return Ok(error_text(code));
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    let work = async {
        let (worktree, repository) =
            tool_environment::revalidate(pool, session.run, session.context).await?;
        if request.is_git() {
            tool_git::run(&session.engine.locks, &worktree, &repository, &request).await
        } else {
            perform(&session.context.root, request, stop.clone()).await
        }
    };
    let interrupted = tokio::select! {
        biased;
        _ = session.cancel.cancelled() => Some(Outcome::Cancelled),
        _ = tokio::time::sleep_until(session.deadline) => Some(Outcome::DeadlineExceeded),
        result = work => {
            return match result {
                Ok((text, truncated)) => {
                    let closing = Closing::Succeeded {
                        result_bytes: i64::try_from(text.len()).unwrap_or(i64::MAX),
                        truncated,
                    };
                    close(pool, &row_id, &closing).await?;
                    Ok(text)
                }
                Err(code) => {
                    close(pool, &row_id, &Closing::Failed { code }).await?;
                    Ok(error_text(code))
                }
            };
        }
    };
    stop.store(true, Ordering::Relaxed);
    let _ = tool_calls::close(pool, &row_id, &Closing::Cancelled).await;
    Err(interrupted.unwrap_or(Outcome::Cancelled))
}

async fn close(
    pool: &sqlx::SqlitePool,
    row_id: &str,
    closing: &Closing<'_>,
) -> Result<(), Outcome> {
    tool_calls::close(pool, row_id, closing)
        .await
        .map(|_| ())
        .map_err(|_| Outcome::Aborted)
}

async fn perform(root: &Path, request: Request, stop: Arc<AtomicBool>) -> tool_files::ToolResult {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || tool_files::run(&root, request, &stop))
        .await
        .unwrap_or(Err("internal_error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm::{Location, Protocol};
    use crate::services::agent_run_engine::testing::{Harness, chat, fast_limits, harness};
    use crate::services::session_load;
    use crate::system::llm_stream::testing::{prompt, request, sse};
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    struct Rig {
        harness: Harness,
        run: AgentRun,
        root: tempfile::TempDir,
        context: ToolContext,
        _background: MockServer,
    }

    async fn rig() -> Rig {
        let background = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&background)
            .await;
        let harness = harness(&background.uri(), fast_limits(), &[("ollama", "qwen")]).await;
        let run = harness.send(0, "hi").await.unwrap().run;
        let workspace_id = session_load::require(&harness.engine.pool)
            .await
            .unwrap()
            .current_workspace_id;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().to_string_lossy().replace('\'', "''");
        let agent_id = harness.agents[0].clone();
        let room_id = harness.room_id.clone();
        for sql in [
            format!(
                "INSERT INTO repositories VALUES ('p1', '{workspace_id}', 'app', '/app', '/app/.git', 'x', 'x')"
            ),
            format!(
                "INSERT INTO tasks (id, workspace_id, title, description, status, priority, repository_id, created_at, updated_at) VALUES ('t1', '{workspace_id}', 'Fix', '', 'pending', 'low', 'p1', 'x', 'x')"
            ),
            format!(
                "INSERT INTO task_worktrees (id, workspace_id, task_id, repository_id, path, branch_name, base_ref, base_commit_sha, status, created_at, updated_at) VALUES ('k1', '{workspace_id}', 't1', 'p1', '{path}', 'meelting/fix-k1', 'main', '{SHA}', 'ready', 'x', 'x')"
            ),
            format!("INSERT INTO task_agents VALUES ('t1', '{agent_id}', '{workspace_id}', 'x')"),
            format!(
                "INSERT INTO task_room_links (id, task_id, room_id, message_id, workspace_id, created_at) VALUES ('l1', 't1', '{room_id}', NULL, '{workspace_id}', 'x')"
            ),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(&harness.engine.pool)
                .await
                .unwrap();
        }
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/a.txt"), "a").unwrap();
        std::fs::write(root.path().join(".env"), "SECRET=1").unwrap();
        let context = ToolContext {
            workspace_id,
            task_id: "t1".into(),
            worktree_id: "k1".into(),
            root: root.path().to_path_buf(),
        };
        Rig {
            harness,
            run,
            root,
            context,
            _background: background,
        }
    }

    fn provider(base_url: &str) -> Provider {
        Provider {
            key: "test".into(),
            name: "Test".into(),
            location: Location::Local,
            protocol: Protocol::ChatCompletions,
            base_url: base_url.into(),
            output_tokens_param: None,
            models: vec![],
        }
    }

    fn calls_turn(calls: &[(&str, &str, &str)]) -> ResponseTemplate {
        let fragments: Vec<String> = calls
            .iter()
            .enumerate()
            .map(|(index, (id, name, arguments))| {
                serde_json::json!({
                    "index": index, "id": id, "type": "function",
                    "function": { "name": name, "arguments": arguments }
                })
                .to_string()
            })
            .collect();
        let events = [
            format!(
                r#"{{"choices":[{{"delta":{{"tool_calls":[{}]}}}}]}}"#,
                fragments.join(",")
            ),
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.to_string(),
            r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":3}}"#.to_string(),
            "[DONE]".to_string(),
        ];
        let events: Vec<&str> = events.iter().map(String::as_str).collect();
        sse(&events)
    }

    async fn model(responses: Vec<ResponseTemplate>) -> MockServer {
        let server = MockServer::start().await;
        for response in responses {
            Mock::given(method("POST"))
                .respond_with(response)
                .up_to_n_times(1)
                .mount(&server)
                .await;
        }
        server
    }

    async fn drive_run(rig: &Rig, server: &MockServer, cancel: &CancellationToken) -> Outcome {
        let prompt = prompt();
        let uri = server.uri();
        let request = request(&uri, &prompt);
        let provider = provider(&uri);
        run(&Session {
            engine: &rig.harness.engine,
            run: &rig.run,
            provider: &provider,
            request: &request,
            context: &rig.context,
            deadline: Instant::now() + Duration::from_secs(20),
            cancel,
        })
        .await
    }

    async fn rows(rig: &Rig) -> Vec<(i64, String, String, Option<String>, Option<i64>, i64)> {
        sqlx::query_as(
            "SELECT position, tool, status, error_code, result_bytes, truncated FROM tool_calls ORDER BY position",
        )
        .fetch_all(&rig.harness.engine.pool)
        .await
        .unwrap()
    }

    async fn rows_count(rig: &Rig) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM tool_calls")
            .fetch_one(&rig.harness.engine.pool)
            .await
            .unwrap()
    }

    fn failure(outcome: Outcome) -> (ErrorKind, bool) {
        match outcome {
            Outcome::Failed {
                failure,
                after_tools,
            } => (failure.kind, after_tools),
            _ => panic!("expected a failure"),
        }
    }

    #[tokio::test]
    async fn lists_a_directory_and_answers_with_what_it_saw() {
        let rig = rig().await;
        let server = model(vec![
            calls_turn(&[("call_1", "list_dir", r#"{"path":"src"}"#)]),
            chat(&["Found a.txt"]),
        ])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        let Outcome::Final { text, usage } = outcome else {
            panic!("expected an answer");
        };
        assert_eq!(text, "Found a.txt");
        assert_eq!(usage.input_tokens, Some(21));
        assert_eq!(usage.output_tokens, Some(5));
        let stored = rows(&rig).await;
        assert_eq!(stored.len(), 1);
        assert_eq!(&stored[0].1, "list_dir");
        assert_eq!(&stored[0].2, "succeeded");
        assert_eq!(stored[0].4, Some("a.txt\n".len() as i64));
        let requests = server.received_requests().await.unwrap();
        let second = String::from_utf8_lossy(&requests[1].body).to_string();
        assert!(second.contains(r#""tool_call_id":"call_1""#));
        assert!(second.contains("a.txt"));
        let arguments: String = sqlx::query_scalar("SELECT arguments FROM tool_calls")
            .fetch_one(&rig.harness.engine.pool)
            .await
            .unwrap();
        assert_eq!(arguments, r#"{"path":"src"}"#);
    }

    #[tokio::test]
    async fn a_text_answer_executes_nothing() {
        let rig = rig().await;
        let server = model(vec![chat(&["Just text"])]).await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert!(matches!(outcome, Outcome::Final { .. }));
        assert!(rows(&rig).await.is_empty());
    }

    #[tokio::test]
    async fn rejected_calls_are_audited_with_their_reason_and_answered_as_errors() {
        let rig = rig().await;
        let server = model(vec![
            calls_turn(&[
                ("c1", "run_command", "{}"),
                ("c2", "list_dir", r#"{"path":"../outside"}"#),
                ("c3", "list_dir", r#"{"path":".env"}"#),
                ("c4", "list_dir", r#"{"path":"missing"}"#),
                ("c5", "list_dir", r#"{"path":"src/a.txt"}"#),
            ]),
            chat(&["done"]),
        ])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert!(matches!(outcome, Outcome::Final { .. }));
        let codes: Vec<_> = rows(&rig)
            .await
            .into_iter()
            .map(|row| (row.2, row.3))
            .collect();
        let failed = |code: &str| ("failed".to_string(), Some(code.to_string()));
        assert_eq!(
            codes,
            vec![
                failed("unknown_tool"),
                failed("path_outside"),
                failed("path_protected"),
                failed("not_found"),
                failed("not_a_directory"),
            ]
        );
        let requests = server.received_requests().await.unwrap();
        let second = String::from_utf8_lossy(&requests[1].body).to_string();
        for code in [
            "unknown_tool",
            "path_outside",
            "path_protected",
            "not_found",
            "not_a_directory",
        ] {
            assert!(second.contains(&format!("error: {code}")), "{code}");
        }
    }

    #[tokio::test]
    async fn rejected_calls_are_audited_without_their_arguments_and_answered_as_errors() {
        let rig = rig().await;
        let server = model(vec![
            calls_turn(&[
                ("c1", "list_dir", "{not json SECRET-ONE"),
                ("c2", "read_file", r#"["SECRET-TWO"]"#),
                ("c3", "Bad Name", r#"{"path":"SECRET-THREE"}"#),
                (
                    "c4",
                    "search",
                    &format!(r#"{{"query":"SECRET-FOUR{}"}}"#, "x".repeat(5000)),
                ),
            ]),
            chat(&["done"]),
        ])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert!(matches!(outcome, Outcome::Final { .. }));
        type Row = (
            i64,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            Option<i64>,
        );
        let stored: Vec<Row> =
            sqlx::query_as(
                "SELECT position, tool, arguments, status, error_code, result_bytes FROM tool_calls ORDER BY position",
            )
            .fetch_all(&rig.harness.engine.pool)
            .await
            .unwrap();
        let failed = |position: i64, tool: Option<&str>| {
            (
                position,
                tool.map(str::to_string),
                None,
                "failed".to_string(),
                Some("invalid_arguments".to_string()),
                None,
            )
        };
        assert_eq!(
            stored,
            vec![
                failed(1, Some("list_dir")),
                failed(2, Some("read_file")),
                failed(3, None),
                failed(4, Some("search")),
            ]
        );
        let everything: String = sqlx::query_scalar(
            "SELECT group_concat(ifnull(tool, '-') || ifnull(arguments, '-') || provider_call_id, '|') FROM tool_calls",
        )
        .fetch_one(&rig.harness.engine.pool)
        .await
        .unwrap();
        assert!(!everything.contains("SECRET"));
        let requests = server.received_requests().await.unwrap();
        let second = String::from_utf8_lossy(&requests[1].body).to_string();
        assert_eq!(second.matches("error: invalid_arguments").count(), 4);
        assert!(!second.contains("SECRET"));
    }

    #[tokio::test]
    async fn a_rejected_call_counts_as_audited_so_the_run_is_never_restarted_over_it() {
        let rig = rig().await;
        let server = model(vec![
            calls_turn(&[("c1", "list_dir", "{oops")]),
            ResponseTemplate::new(503),
        ])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(failure(outcome), (ErrorKind::ProviderUnavailable, true));
        assert_eq!(rows_count(&rig).await, 1);
    }

    #[tokio::test]
    async fn invalid_calls_still_spend_the_call_budget() {
        let rig = rig().await;
        let batch: Vec<(String, &str, &str)> = (0..41)
            .map(|index| (format!("c{index}"), "list_dir", "{oops"))
            .collect();
        let batch: Vec<(&str, &str, &str)> = batch
            .iter()
            .map(|(id, name, arguments)| (id.as_str(), *name, *arguments))
            .collect();
        let server = model(vec![calls_turn(&batch)]).await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(
            failure(outcome),
            (ErrorKind::ToolCallBudgetExhausted, false)
        );
        assert!(rows(&rig).await.is_empty());
    }

    #[tokio::test]
    async fn the_call_budget_is_checked_for_the_whole_batch_before_running_any_of_it() {
        let rig = rig().await;
        let first: Vec<String> = (0..21).map(|index| format!("a{index}")).collect();
        let second: Vec<String> = (0..21).map(|index| format!("b{index}")).collect();
        let batch = |ids: &[String]| {
            calls_turn(
                &ids.iter()
                    .map(|id| (id.as_str(), "list_dir", "{}"))
                    .collect::<Vec<_>>(),
            )
        };
        let server = model(vec![batch(&first), batch(&second)]).await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(failure(outcome), (ErrorKind::ToolCallBudgetExhausted, true));
        assert_eq!(rows(&rig).await.len(), 21);
    }

    #[tokio::test]
    async fn the_twenty_first_model_request_is_never_sent() {
        let rig = rig().await;
        let ids: Vec<String> = (0..20).map(|index| format!("call_{index}")).collect();
        let responses = ids
            .iter()
            .map(|id| calls_turn(&[(id.as_str(), "list_dir", "{}")]))
            .collect();
        let server = model(responses).await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(
            failure(outcome),
            (ErrorKind::IterationBudgetExhausted, true)
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 20);
        assert_eq!(rows(&rig).await.len(), 20);
    }

    #[tokio::test]
    async fn accumulated_output_beyond_the_budget_ends_the_run_after_recording_the_calls() {
        let rig = rig().await;
        for index in 0..300 {
            std::fs::write(
                rig.root
                    .path()
                    .join(format!("{index:03}-{}", "n".repeat(40))),
                "x",
            )
            .unwrap();
        }
        let ids: Vec<String> = (0..20).map(|index| format!("c{index}")).collect();
        let batch: Vec<(&str, &str, &str)> = ids
            .iter()
            .map(|id| (id.as_str(), "list_dir", "{}"))
            .collect();
        let server = model(vec![calls_turn(&batch)]).await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(
            failure(outcome),
            (ErrorKind::ToolOutputBudgetExhausted, true)
        );
        let stored = rows(&rig).await;
        assert!(stored.len() < 20);
        assert!(stored.iter().all(|row| row.2 == "succeeded" && row.5 == 1));
        assert!(stored.iter().all(|row| row.4.unwrap() <= 8 * 1024));
    }

    #[tokio::test]
    async fn duplicate_missing_or_reused_call_ids_are_an_invalid_response() {
        let rig = rig().await;
        let server = model(vec![calls_turn(&[
            ("same", "list_dir", "{}"),
            ("same", "list_dir", "{}"),
        ])])
        .await;
        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;
        assert_eq!(failure(outcome), (ErrorKind::InvalidResponse, false));

        let server = model(vec![calls_turn(&[("", "list_dir", "{}")])]).await;
        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;
        assert_eq!(failure(outcome), (ErrorKind::InvalidResponse, false));

        let server = model(vec![
            calls_turn(&[("again", "list_dir", "{}")]),
            calls_turn(&[("again", "list_dir", "{}")]),
        ])
        .await;
        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;
        assert_eq!(failure(outcome), (ErrorKind::InvalidResponse, true));
        assert_eq!(rows(&rig).await.len(), 1);
    }

    #[tokio::test]
    async fn a_failure_before_any_tool_ran_may_be_retried_but_not_after() {
        let rig = rig().await;
        let server = model(vec![ResponseTemplate::new(503)]).await;
        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;
        assert_eq!(failure(outcome), (ErrorKind::ProviderUnavailable, false));

        let server = model(vec![
            calls_turn(&[("call_1", "list_dir", "{}")]),
            ResponseTemplate::new(503),
        ])
        .await;
        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;
        assert_eq!(failure(outcome), (ErrorKind::ProviderUnavailable, true));
    }

    #[tokio::test]
    async fn a_call_cut_off_mid_stream_executes_nothing() {
        let rig = rig().await;
        let server = model(vec![sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"list_dir","arguments":"{\"pa"}}]}}]}"#,
        ])])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(failure(outcome), (ErrorKind::ProviderUnavailable, false));
        assert!(rows(&rig).await.is_empty());
    }

    #[tokio::test]
    async fn cancelling_while_the_model_answers_records_nothing() {
        let rig = rig().await;
        let server = model(vec![
            calls_turn(&[("call_1", "list_dir", "{}")]).set_delay(Duration::from_secs(5)),
        ])
        .await;
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            trigger.cancel();
        });

        let outcome = drive_run(&rig, &server, &cancel).await;

        assert!(matches!(outcome, Outcome::Cancelled));
        assert!(rows(&rig).await.is_empty());
    }

    #[tokio::test]
    async fn cancelling_a_running_call_closes_its_row_as_cancelled() {
        let rig = rig().await;
        let server = MockServer::start().await;
        let prompt = prompt();
        let uri = server.uri();
        let request = request(&uri, &prompt);
        let provider = provider(&uri);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let session = Session {
            engine: &rig.harness.engine,
            run: &rig.run,
            provider: &provider,
            request: &request,
            context: &rig.context,
            deadline: Instant::now() + Duration::from_secs(20),
            cancel: &cancel,
        };
        let mut state = State::default();
        state.budget.start_iteration().unwrap();
        let call = Call {
            id: "call_1".into(),
            name: "list_dir".into(),
            prepared: tools::prepare("list_dir", "{}", false),
        };

        let outcome = execute(&session, &mut state, &call).await;

        assert!(matches!(outcome, Err(Outcome::Cancelled)));
        let stored = rows(&rig).await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].2, "cancelled");
        assert_eq!(stored[0].4, None);
        assert!(state.executed);
    }

    #[tokio::test]
    async fn the_text_of_a_tool_turn_is_part_of_the_history_sent_back() {
        let rig = rig().await;
        let with_text = sse(&[
            r#"{"choices":[{"delta":{"content":"Let me look. "}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"list_dir","arguments":"{}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ]);
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("tool_call_id"))
            .respond_with(chat(&["final"]))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(with_text)
            .mount(&server)
            .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        let Outcome::Final { text, .. } = outcome else {
            panic!("expected an answer");
        };
        assert_eq!(text, "final");
        let requests = server.received_requests().await.unwrap();
        let second = String::from_utf8_lossy(&requests[1].body).to_string();
        assert!(second.contains("Let me look. "));
    }

    #[tokio::test]
    async fn startup_recovery_closes_the_calls_left_running_exactly_once() {
        let rig = rig().await;
        let pool = &rig.harness.engine.pool;
        tool_calls::insert_running(
            pool,
            &NewCall {
                id: "left-behind",
                run_id: &rig.run.id,
                room_id: &rig.run.room_id,
                workspace_id: &rig.context.workspace_id,
                task_id: "t1",
                worktree_id: "k1",
                iteration: 1,
                position: 1,
                provider_call_id: "call_1",
                tool: "list_dir",
                arguments: "{}",
            },
        )
        .await
        .unwrap();

        crate::services::agent_runs_expire::run(pool).await.unwrap();
        crate::services::agent_runs_expire::run(pool).await.unwrap();

        let stored = rows(&rig).await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].2, "cancelled");
    }

    #[tokio::test]
    async fn reads_and_searches_files_and_audits_both_without_storing_content() {
        let rig = rig().await;
        std::fs::write(rig.root.path().join("src/a.txt"), "alpha\nneedle line\n").unwrap();
        std::fs::write(rig.root.path().join("bin.dat"), [0u8, 1, 2]).unwrap();
        let server = model(vec![
            calls_turn(&[
                ("c1", "read_file", r#"{"path":"src/a.txt"}"#),
                ("c2", "search", r#"{"query":"needle"}"#),
                ("c3", "read_file", r#"{"path":".env"}"#),
                ("c4", "read_file", r#"{"path":"bin.dat"}"#),
                ("c5", "read_file", r#"{"path":"src/a.txt","offset":9999}"#),
            ]),
            chat(&["done"]),
        ])
        .await;

        let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

        assert!(matches!(outcome, Outcome::Final { .. }));
        let stored = rows(&rig).await;
        let summary: Vec<_> = stored
            .iter()
            .map(|row| (row.1.as_str(), row.2.as_str(), row.3.as_deref()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("read_file", "succeeded", None),
                ("search", "succeeded", None),
                ("read_file", "failed", Some("path_protected")),
                ("read_file", "failed", Some("binary_file")),
                ("read_file", "failed", Some("offset_out_of_range")),
            ]
        );
        let requests = server.received_requests().await.unwrap();
        let second = String::from_utf8_lossy(&requests[1].body).to_string();
        assert!(second.contains("alpha"));
        assert!(second.contains("src/a.txt:2: needle line"));
        assert!(!second.contains("SECRET=1"));
        let persisted: String = sqlx::query_scalar(
            "SELECT group_concat(arguments || ifnull(error_code, ''), '|') FROM tool_calls",
        )
        .fetch_one(&rig.harness.engine.pool)
        .await
        .unwrap();
        assert!(!persisted.contains("alpha") && !persisted.contains("needle line"));
    }

    #[tokio::test]
    async fn a_search_still_running_is_stopped_when_the_run_is_cancelled() {
        let rig = rig().await;
        let server = MockServer::start().await;
        let prompt = prompt();
        let uri = server.uri();
        let request = request(&uri, &prompt);
        let provider = provider(&uri);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let session = Session {
            engine: &rig.harness.engine,
            run: &rig.run,
            provider: &provider,
            request: &request,
            context: &rig.context,
            deadline: Instant::now() + Duration::from_secs(20),
            cancel: &cancel,
        };
        let mut state = State::default();
        state.budget.start_iteration().unwrap();
        let call = Call {
            id: "call_1".into(),
            name: "search".into(),
            prepared: tools::prepare("search", r#"{"query":"a"}"#, false),
        };

        let outcome = execute(&session, &mut state, &call).await;

        assert!(matches!(outcome, Err(Outcome::Cancelled)));
        assert_eq!(rows(&rig).await[0].2, "cancelled");
    }

    #[tokio::test]
    async fn a_task_that_is_no_longer_linked_or_moved_stops_every_tool_before_it_runs() {
        let rig = rig().await;
        let pool = &rig.harness.engine.pool;
        let server = model(vec![
            calls_turn(&[("c1", "list_dir", "{}")]),
            calls_turn(&[("c2", "git_status", "{}")]),
            calls_turn(&[("c3", "read_file", r#"{"path":"src/a.txt"}"#)]),
            chat(&["done"]),
        ])
        .await;
        let session_cancel = CancellationToken::new();
        sqlx::query("DELETE FROM task_room_links")
            .execute(pool)
            .await
            .unwrap();

        let outcome = drive_run(&rig, &server, &session_cancel).await;

        assert!(matches!(outcome, Outcome::Final { .. }));
        let codes: Vec<_> = rows(&rig).await.into_iter().map(|row| row.3).collect();
        assert_eq!(
            codes,
            vec![Some("environment_not_authorized".to_string()); 3]
        );
    }

    #[tokio::test]
    async fn an_environment_whose_path_changed_is_refused() {
        let rig = rig().await;
        sqlx::query("UPDATE task_worktrees SET path = '/somewhere/else'")
            .execute(&rig.harness.engine.pool)
            .await
            .unwrap();
        let server = model(vec![
            calls_turn(&[("c1", "list_dir", "{}")]),
            chat(&["done"]),
        ])
        .await;

        drive_run(&rig, &server, &CancellationToken::new()).await;

        assert_eq!(
            rows(&rig).await[0].3.as_deref(),
            Some("environment_changed")
        );
    }

    #[tokio::test]
    async fn a_git_call_is_cancellable_like_any_other() {
        let rig = rig().await;
        let server = MockServer::start().await;
        let prompt = prompt();
        let uri = server.uri();
        let request = request(&uri, &prompt);
        let provider = provider(&uri);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let session = Session {
            engine: &rig.harness.engine,
            run: &rig.run,
            provider: &provider,
            request: &request,
            context: &rig.context,
            deadline: Instant::now() + Duration::from_secs(20),
            cancel: &cancel,
        };
        let mut state = State::default();
        state.budget.start_iteration().unwrap();
        let call = Call {
            id: "call_1".into(),
            name: "git_diff".into(),
            prepared: tools::prepare("git_diff", r#"{"path":"a.txt"}"#, false),
        };

        let outcome = execute(&session, &mut state, &call).await;

        assert!(matches!(outcome, Err(Outcome::Cancelled)));
        assert_eq!(rows(&rig).await[0].2, "cancelled");
    }

    #[tokio::test]
    async fn a_call_id_with_unexpected_characters_is_an_invalid_response() {
        let rig = rig().await;
        for id in ["call 1", "id\\u0000x", "a/b", "ñ"] {
            let server = model(vec![calls_turn(&[(id, "list_dir", "{}")])]).await;

            let outcome = drive_run(&rig, &server, &CancellationToken::new()).await;

            assert_eq!(
                failure(outcome),
                (ErrorKind::InvalidResponse, false),
                "{id}"
            );
        }
        let server = model(vec![
            calls_turn(&[("functions.list_dir:0", "list_dir", "{}")]),
            chat(&["ok"]),
        ])
        .await;
        assert!(matches!(
            drive_run(&rig, &server, &CancellationToken::new()).await,
            Outcome::Final { .. }
        ));
    }
}
