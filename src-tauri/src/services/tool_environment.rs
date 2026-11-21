use crate::catalogs::llm::{Location, Protocol};
use crate::domain::agent_run::AgentRun;
use crate::domain::prompt::ToolsNote;
use crate::domain::repository::RepositoryIdentity;
use crate::domain::tool_support::ToolSupport;
use crate::domain::tools::ToolContext;
use crate::domain::worktree::Worktree;
use crate::repositories::task_room_links;
use crate::services::agent_run_engine::Engine;
use crate::services::agent_run_execute::Target;
use crate::services::session_load::SessionError;
use crate::services::worktree_remove::ready_environment_in;
use crate::system::tool_support;
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct Access {
    pub note: ToolsNote,
    pub context: Option<ToolContext>,
}

impl Access {
    fn without(note: ToolsNote) -> Access {
        Access {
            note,
            context: None,
        }
    }
}

pub struct Gate {
    pub engine_enabled: bool,
    pub cloud_consent: bool,
}

pub fn tools_allowed(target: Target<'_>, gate: &Gate, local_support: ToolSupport) -> bool {
    gate.engine_enabled
        && target.provider.protocol == Protocol::ChatCompletions
        && match target.provider.location {
            Location::Cloud => target.tools && gate.cloud_consent,
            Location::Local => local_support == ToolSupport::Supported,
        }
}

pub async fn model_can_use_tools(engine: &Engine, target: Target<'_>, model: &str) -> bool {
    let gate = Gate {
        engine_enabled: engine.tools_enabled(),
        cloud_consent: engine.cloud_tools_consent(),
    };
    let local_support = if gate.engine_enabled && target.provider.location == Location::Local {
        tool_support::detect(target.provider, model).await
    } else {
        ToolSupport::Unknown
    };
    tools_allowed(target, &gate, local_support)
}

pub async fn revalidate(
    pool: &SqlitePool,
    run: &AgentRun,
    context: &ToolContext,
) -> Result<(Worktree, RepositoryIdentity), &'static str> {
    let unavailable = |_| "environment_unavailable";
    let candidates =
        task_room_links::candidate_tasks(pool, &context.workspace_id, &run.room_id, &run.agent_id)
            .await
            .map_err(unavailable)?;
    if candidates.as_slice() != [context.task_id.clone()] {
        return Err("environment_not_authorized");
    }
    let (_, worktree, repository) =
        ready_environment_in(pool, &context.workspace_id, &context.task_id)
            .await
            .map_err(|_| "environment_unavailable")?;
    if worktree.id != context.worktree_id || std::path::Path::new(&worktree.path) != context.root {
        return Err("environment_changed");
    }
    Ok((worktree, repository))
}

pub async fn resolve(
    pool: &SqlitePool,
    workspace_id: &str,
    room_id: &str,
    agent_id: &str,
    model_can_use_tools: impl AsyncFnOnce() -> bool,
    verify: impl AsyncFn(&RepositoryIdentity, &Worktree) -> Result<(), SessionError>,
) -> Result<Access, sqlx::Error> {
    let candidates =
        task_room_links::candidate_tasks(pool, workspace_id, room_id, agent_id).await?;
    let task_id = match candidates.as_slice() {
        [] => return Ok(Access::without(ToolsNote::Off)),
        [task_id] => task_id,
        _ => return Ok(Access::without(ToolsNote::Ambiguous)),
    };
    let Ok((_, worktree, repository)) = ready_environment_in(pool, workspace_id, task_id).await
    else {
        return Ok(Access::without(ToolsNote::EnvironmentNotReady));
    };
    if !model_can_use_tools().await {
        return Ok(Access::without(ToolsNote::Restricted));
    }
    if verify(&repository, &worktree).await.is_err() {
        return Ok(Access::without(ToolsNote::EnvironmentNotReady));
    }
    Ok(Access {
        note: ToolsNote::Enabled,
        context: Some(ToolContext {
            workspace_id: workspace_id.to_string(),
            task_id: task_id.clone(),
            worktree_id: worktree.id,
            root: PathBuf::from(worktree.path),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm;
    use crate::domain::agent_run::{AgentRun, RunStatus};
    use crate::domain::provider_key;
    use crate::domain::recipient::Selection;
    use crate::domain::task::{TaskDraft, TaskPriority};
    use crate::repositories::agent_runs;
    use crate::services::agent_run_engine::testing::{chat, fast_limits, hire, providers_at};
    use crate::services::agent_run_engine::{Engine, RunEvent};
    use crate::services::intervention_send::{self, Intervention, InterventionSent};
    use crate::services::worktree_prepare::tests::{Fixture, fixture, prepare};
    use crate::services::{room_create, session_load, task_save, team_create};
    use crate::system::keychain;
    use crate::system::llm_stream::testing::sse;
    use std::cell::Cell;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer};

    struct Office {
        fixture: Fixture,
        workspace_id: String,
        agent_id: String,
        room_id: String,
    }

    async fn office(provider: &str, model: &str) -> Office {
        let fixture = fixture().await;
        let pool = &fixture.pool;
        let workspace_id = session_load::require(pool)
            .await
            .unwrap()
            .current_workspace_id;
        let team_id = team_create::run(pool, "General").await.unwrap()[0]
            .id
            .clone();
        let agent_id = hire(pool, &team_id, "Ada", "Developer", provider, model).await;
        let room = room_create::run(pool, "Daily", vec![agent_id.clone()])
            .await
            .unwrap();
        Office {
            fixture,
            workspace_id,
            agent_id,
            room_id: room.id,
        }
    }

    impl Office {
        async fn task(&self, assigned: bool, linked: bool) -> String {
            let draft = TaskDraft {
                description: "Fix the login".into(),
                priority: TaskPriority::Medium,
                repository_id: Some(self.fixture.repository_id.clone()),
                base_ref: Some("main".into()),
                agent_ids: if assigned {
                    vec![self.agent_id.clone()]
                } else {
                    Vec::new()
                },
            };
            let task = task_save::create(&self.fixture.pool, draft, None)
                .await
                .unwrap();
            if linked {
                task_room_links::insert(
                    &self.fixture.pool,
                    &Uuid::now_v7().to_string(),
                    &task.id,
                    &self.workspace_id,
                    &self.room_id,
                    None,
                )
                .await
                .unwrap();
            }
            task.id
        }

        async fn prepared(&self, task_id: &str) -> String {
            let task =
                crate::services::tasks_list::one(&self.fixture.pool, &self.workspace_id, task_id)
                    .await
                    .unwrap();
            prepare(&self.fixture, &task).await.worktree.unwrap().path
        }

        async fn access(&self, allowed: bool, verified: bool) -> (Access, usize, usize) {
            let checks = Cell::new(0);
            let asked = Cell::new(0);
            let access = resolve(
                &self.fixture.pool,
                &self.workspace_id,
                &self.room_id,
                &self.agent_id,
                async || {
                    asked.set(asked.get() + 1);
                    allowed
                },
                async |_, _| {
                    checks.set(checks.get() + 1);
                    if verified {
                        Ok(())
                    } else {
                        Err(SessionError::EnvironmentIdentityMismatch)
                    }
                },
            )
            .await
            .unwrap();
            (access, checks.get(), asked.get())
        }
    }

    #[test]
    fn four_separate_conditions_must_all_hold_before_a_model_may_use_tools() {
        let providers = llm::providers();
        let target = |provider: &str, model: &str| {
            let provider = providers
                .iter()
                .find(|entry| entry.key == provider)
                .unwrap();
            crate::services::agent_run_execute::resolve(
                std::slice::from_ref(provider),
                &provider.key,
                model,
            )
            .unwrap()
        };
        let open = Gate {
            engine_enabled: true,
            cloud_consent: true,
        };
        let no_consent = Gate {
            engine_enabled: true,
            cloud_consent: false,
        };
        let closed = Gate {
            engine_enabled: false,
            cloud_consent: true,
        };
        let unknown = ToolSupport::Unknown;
        let deepseek = target("deepseek", "deepseek-flash");

        assert!(tools_allowed(deepseek, &open, unknown));
        assert!(!tools_allowed(deepseek, &no_consent, unknown));
        assert!(!tools_allowed(deepseek, &closed, unknown));
        assert!(!tools_allowed(
            target("groq", "openai/gpt-oss-120b"),
            &open,
            unknown
        ));
        assert!(!tools_allowed(
            target("anthropic", "claude-sonnet-5-5"),
            &open,
            ToolSupport::Supported
        ));
        assert!(!tools_allowed(
            target("openai", "gpt-6.1-sol"),
            &open,
            ToolSupport::Supported
        ));
        for (provider, supports, expected) in [
            ("ollama", ToolSupport::Supported, true),
            ("ollama", ToolSupport::Unsupported, false),
            ("ollama", ToolSupport::Unknown, false),
            ("lmstudio", ToolSupport::Supported, true),
            ("lmstudio", ToolSupport::Unknown, false),
        ] {
            assert_eq!(
                tools_allowed(target(provider, "m"), &open, supports),
                expected,
                "{provider} {supports:?}"
            );
            assert!(
                !tools_allowed(target(provider, "m"), &closed, supports),
                "a supported local model is still closed while the engine is"
            );
        }
        assert!(
            tools_allowed(target("ollama", "m"), &no_consent, ToolSupport::Supported),
            "cloud consent never applies to local models"
        );
    }

    #[tokio::test]
    async fn a_room_without_an_eligible_task_gets_no_note_and_no_tools() {
        let office = office("ollama", "qwen").await;
        office.task(true, false).await;
        office.task(false, true).await;

        let (access, checks, asked) = office.access(true, true).await;

        assert_eq!(access, Access::without(ToolsNote::Off));
        assert_eq!(checks, 0);
        assert_eq!(asked, 0);
    }

    #[tokio::test]
    async fn several_eligible_tasks_are_reported_as_ambiguous_and_none_is_chosen() {
        let office = office("ollama", "qwen").await;
        let first = office.task(true, true).await;
        office.task(true, true).await;
        office.prepared(&first).await;

        let (access, checks, asked) = office.access(true, true).await;

        assert_eq!(access, Access::without(ToolsNote::Ambiguous));
        assert_eq!(checks, 0);
        assert_eq!(asked, 0);
    }

    #[tokio::test]
    async fn one_task_without_a_ready_environment_is_reported_as_not_ready() {
        let office = office("ollama", "qwen").await;
        office.task(true, true).await;

        let (access, checks, asked) = office.access(true, true).await;

        assert_eq!(access, Access::without(ToolsNote::EnvironmentNotReady));
        assert_eq!(checks, 0);
        assert_eq!(asked, 0);
    }

    #[tokio::test]
    async fn a_model_that_may_not_use_tools_never_reaches_the_environment_check() {
        let office = office("ollama", "qwen").await;
        let task = office.task(true, true).await;
        office.prepared(&task).await;

        let (access, checks, asked) = office.access(false, true).await;

        assert_eq!(access, Access::without(ToolsNote::Restricted));
        assert_eq!(checks, 0);
        assert_eq!(asked, 1);
    }

    #[tokio::test]
    async fn the_single_ready_environment_is_verified_and_becomes_the_tool_root() {
        let office = office("ollama", "qwen").await;
        let task = office.task(true, true).await;
        let path = office.prepared(&task).await;

        let (access, checks, asked) = office.access(true, true).await;

        let context = access.context.unwrap();
        assert_eq!(access.note, ToolsNote::Enabled);
        assert_eq!(context.root, PathBuf::from(path));
        assert_eq!(context.task_id, task);
        assert_eq!(context.workspace_id, office.workspace_id);
        assert_eq!(checks, 1);
        assert_eq!(asked, 1);

        let (access, _, _) = office.access(true, false).await;
        assert_eq!(access, Access::without(ToolsNote::EnvironmentNotReady));
    }

    async fn ask(engine: &Arc<Engine>, office: &Office) -> AgentRun {
        let selection = Selection::Agent(office.agent_id.clone());
        match intervention_send::run(
            engine,
            Intervention {
                room_id: &office.room_id,
                body: "What is in src?",
                selection: &selection,
                mention_ids: &[],
                client_request_id: &Uuid::now_v7().to_string(),
            },
        )
        .await
        .unwrap()
        {
            InterventionSent::Sent { run: Some(run), .. } => *run,
            other => panic!("expected a run, got {other:?}"),
        }
    }

    async fn finished(engine: &Engine, room_id: &str, run_id: &str) -> AgentRun {
        for _ in 0..1000 {
            let runs = agent_runs::for_room(&engine.pool, room_id, 1000)
                .await
                .unwrap();
            let run = runs.into_iter().find(|run| run.id == run_id).unwrap();
            if matches!(run.status, RunStatus::Succeeded | RunStatus::Failed) {
                return run;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the run never finished");
    }

    fn engine(office: &Office, url: &str) -> Arc<Engine> {
        Engine::new(
            office.fixture.pool.clone(),
            providers_at(url),
            fast_limits(),
            office.fixture.locks.clone(),
            Arc::new(|_: RunEvent| {}),
        )
    }

    async fn bodies(server: &MockServer) -> Vec<String> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| String::from_utf8_lossy(&request.body).to_string())
            .collect()
    }

    #[tokio::test]
    async fn a_model_that_may_not_use_tools_keeps_talking_in_text_and_is_told_it_has_none() {
        let office = office("ollama", "qwen").await;
        let task = office.task(true, true).await;
        office.prepared(&task).await;
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["I cannot read files here."]))
            .mount(&server)
            .await;
        let engine = engine(&office, &server.uri());

        let run = ask(&engine, &office).await;

        assert_eq!(
            finished(&engine, &office.room_id, &run.id).await.status,
            RunStatus::Succeeded
        );
        let sent = bodies(&server).await;
        assert!(!sent[0].contains(r#""tools""#));
        assert!(sent[0].contains("cannot use tools"));
        let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM tool_calls")
            .fetch_one(&engine.pool)
            .await
            .unwrap();
        assert_eq!(calls, 0);
    }

    #[tokio::test]
    async fn an_ambiguous_room_tells_the_model_so_and_runs_as_plain_text() {
        let office = office("ollama", "qwen").await;
        office.task(true, true).await;
        office.task(true, true).await;
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["Which task?"]))
            .mount(&server)
            .await;
        let engine = engine(&office, &server.uri());

        let run = ask(&engine, &office).await;

        assert_eq!(
            finished(&engine, &office.room_id, &run.id).await.status,
            RunStatus::Succeeded
        );
        let sent = bodies(&server).await;
        assert!(sent[0].contains("several tasks"));
        assert!(!sent[0].contains(r#""tools""#));
    }

    #[tokio::test]
    async fn with_consent_a_declared_cloud_model_lists_the_worktree_and_answers_with_it() {
        let office = office("deepseek", "deepseek-flash").await;
        let task = office.task(true, true).await;
        let path = office.prepared(&task).await;
        let secret = provider_key::secret_name(&office.workspace_id, "deepseek");
        keychain::save(&secret, "sk-deepseek-test").unwrap();
        let server = MockServer::start().await;
        let call = [
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"list_dir","arguments":"{}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ];
        Mock::given(method("POST"))
            .respond_with(sse(&call))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["The repository has a README."]))
            .mount(&server)
            .await;
        let engine = engine(&office, &server.uri());
        engine.set_tools_enabled(true);
        engine.grant_cloud_tools_consent();

        let run = ask(&engine, &office).await;
        let done = finished(&engine, &office.room_id, &run.id).await;
        keychain::remove(&secret).unwrap();

        assert_eq!(done.status, RunStatus::Succeeded);
        let sent = bodies(&server).await;
        assert!(sent[0].contains(r#""tools""#) && sent[0].contains("untrusted data"));
        assert!(sent[1].contains("README.md"));
        let stored: Vec<(String, String, String)> =
            sqlx::query_as("SELECT tool, status, worktree_id FROM tool_calls")
                .fetch_all(&engine.pool)
                .await
                .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].1, "succeeded");
        let reply: String = sqlx::query_scalar(
            "SELECT body FROM messages WHERE agent_id IS NOT NULL ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_one(&engine.pool)
        .await
        .unwrap();
        assert_eq!(reply, "The repository has a README.");
        assert!(PathBuf::from(path).join("README.md").exists());
    }

    #[tokio::test]
    async fn with_consent_the_model_reads_the_git_state_of_the_worktree() {
        let office = office("deepseek", "deepseek-flash").await;
        let task = office.task(true, true).await;
        let path = office.prepared(&task).await;
        std::fs::write(
            PathBuf::from(&path).join("README.md"),
            "hello\nedited by hand\n",
        )
        .unwrap();
        std::fs::write(PathBuf::from(&path).join(".env"), "TOKEN=abc\n").unwrap();
        let secret = provider_key::secret_name(&office.workspace_id, "deepseek");
        keychain::save(&secret, "sk-deepseek-test").unwrap();
        let server = MockServer::start().await;
        let calls = [
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"git_status","arguments":"{}"}},{"index":1,"id":"call_2","function":{"name":"git_diff","arguments":"{\"path\":\"README.md\"}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ];
        Mock::given(method("POST"))
            .respond_with(sse(&calls))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["README was edited."]))
            .mount(&server)
            .await;
        let engine = engine(&office, &server.uri());
        engine.set_tools_enabled(true);
        engine.grant_cloud_tools_consent();

        let run = ask(&engine, &office).await;
        let done = finished(&engine, &office.room_id, &run.id).await;
        keychain::remove(&secret).unwrap();

        assert_eq!(done.status, RunStatus::Succeeded);
        let sent = bodies(&server).await;
        assert!(sent[1].contains(".M README.md") && sent[1].contains("+edited by hand"));
        assert!(sent[1].contains("[1 protected entries hidden]"));
        assert!(!sent[1].contains("TOKEN=abc"));
        let stored: Vec<(String, String)> =
            sqlx::query_as("SELECT tool, status FROM tool_calls ORDER BY position")
                .fetch_all(&engine.pool)
                .await
                .unwrap();
        assert_eq!(
            stored,
            vec![
                ("git_status".into(), "succeeded".into()),
                ("git_diff".into(), "succeeded".into())
            ]
        );
    }

    fn shown(capabilities: serde_json::Value) -> wiremock::ResponseTemplate {
        wiremock::ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({ "capabilities": capabilities }))
    }

    async fn local_server(
        show: Vec<wiremock::ResponseTemplate>,
        chat_turns: Vec<wiremock::ResponseTemplate>,
    ) -> MockServer {
        let server = MockServer::start().await;
        for response in show {
            Mock::given(method("POST"))
                .and(wiremock::matchers::path("/api/show"))
                .respond_with(response)
                .up_to_n_times(1)
                .mount(&server)
                .await;
        }
        for response in chat_turns {
            Mock::given(method("POST"))
                .and(wiremock::matchers::path("/chat/completions"))
                .respond_with(response)
                .up_to_n_times(1)
                .mount(&server)
                .await;
        }
        server
    }

    async fn paths(server: &MockServer) -> Vec<String> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| request.url.path().to_string())
            .collect()
    }

    fn list_dir_turn() -> wiremock::ResponseTemplate {
        sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"list_dir","arguments":"{}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ])
    }

    #[tokio::test]
    async fn a_local_model_detected_as_supporting_tools_uses_them_once_the_engine_is_open() {
        let office = office("ollama", "qwen3:4b").await;
        let task = office.task(true, true).await;
        office.prepared(&task).await;
        let server = local_server(
            vec![shown(serde_json::json!(["completion", "tools"]))],
            vec![list_dir_turn(), chat(&["It has a README."])],
        )
        .await;
        let engine = engine(&office, &server.uri());
        engine.set_tools_enabled(true);

        let run = ask(&engine, &office).await;

        assert_eq!(
            finished(&engine, &office.room_id, &run.id).await.status,
            RunStatus::Succeeded
        );
        let sent = bodies(&server).await;
        let chat_bodies: Vec<_> = sent
            .iter()
            .filter(|body| body.contains("\"messages\""))
            .collect();
        assert!(chat_bodies[0].contains(r#""tools""#));
        assert!(chat_bodies[1].contains("README.md"));
        assert_eq!(
            paths(&server)
                .await
                .iter()
                .filter(|path| *path == "/api/show")
                .count(),
            1
        );
        let calls: i64 =
            sqlx::query_scalar("SELECT count(*) FROM tool_calls WHERE status = 'succeeded'")
                .fetch_one(&engine.pool)
                .await
                .unwrap();
        assert_eq!(calls, 1);
    }

    #[tokio::test]
    async fn a_local_model_that_does_not_declare_tools_keeps_talking_in_text() {
        for capabilities in [serde_json::json!(["completion"]), serde_json::Value::Null] {
            let office = office("ollama", "qwen3:4b").await;
            let task = office.task(true, true).await;
            office.prepared(&task).await;
            let server = local_server(
                vec![shown(capabilities)],
                vec![chat(&["I cannot read files."])],
            )
            .await;
            let engine = engine(&office, &server.uri());
            engine.set_tools_enabled(true);

            let run = ask(&engine, &office).await;

            assert_eq!(
                finished(&engine, &office.room_id, &run.id).await.status,
                RunStatus::Succeeded
            );
            let sent = bodies(&server).await;
            let chat_body = sent
                .iter()
                .find(|body| body.contains("\"messages\""))
                .unwrap();
            assert!(!chat_body.contains(r#""tools""#));
            assert!(chat_body.contains("cannot use tools"));
        }
    }

    #[tokio::test]
    async fn a_closed_engine_never_even_asks_the_local_runtime() {
        let office = office("ollama", "qwen3:4b").await;
        let task = office.task(true, true).await;
        office.prepared(&task).await;
        let server = local_server(
            vec![shown(serde_json::json!(["completion", "tools"]))],
            vec![chat(&["text only"])],
        )
        .await;
        let engine = engine(&office, &server.uri());

        let run = ask(&engine, &office).await;

        assert_eq!(
            finished(&engine, &office.room_id, &run.id).await.status,
            RunStatus::Succeeded
        );
        assert!(!paths(&server).await.contains(&"/api/show".to_string()));
        let sent = bodies(&server).await;
        assert!(!sent[0].contains(r#""tools""#) && sent[0].contains("cannot use tools"));
    }

    #[tokio::test]
    async fn a_room_without_a_task_never_asks_the_local_runtime_either() {
        let office = office("ollama", "qwen3:4b").await;
        let server = local_server(vec![], vec![chat(&["hello"])]).await;
        let engine = engine(&office, &server.uri());
        engine.set_tools_enabled(true);

        let run = ask(&engine, &office).await;

        assert_eq!(
            finished(&engine, &office.room_id, &run.id).await.status,
            RunStatus::Succeeded
        );
        assert_eq!(paths(&server).await, vec!["/chat/completions".to_string()]);
    }

    #[tokio::test]
    async fn the_capability_is_read_again_for_every_run_so_a_changed_model_is_noticed() {
        let office = office("ollama", "qwen3:4b").await;
        let task = office.task(true, true).await;
        office.prepared(&task).await;
        let server = local_server(
            vec![
                shown(serde_json::json!(["completion", "tools"])),
                shown(serde_json::json!(["completion"])),
            ],
            vec![chat(&["first"]), chat(&["second"])],
        )
        .await;
        let engine = engine(&office, &server.uri());
        engine.set_tools_enabled(true);

        let first = ask(&engine, &office).await;
        finished(&engine, &office.room_id, &first.id).await;
        let second = ask(&engine, &office).await;
        finished(&engine, &office.room_id, &second.id).await;

        let sent = bodies(&server).await;
        let chats: Vec<_> = sent
            .iter()
            .filter(|body| body.contains("\"messages\""))
            .collect();
        assert!(chats[0].contains(r#""tools""#));
        assert!(!chats[1].contains(r#""tools""#) && chats[1].contains("cannot use tools"));
    }

    struct RealRun {
        engine: Arc<Engine>,
        run: AgentRun,
        calls: Vec<(String, String, Option<String>)>,
        reply: Option<String>,
    }

    async fn real_local_run(office: &Office, body: &str) -> RealRun {
        let key = std::env::var("MEELTING_LOCAL_PROVIDER").unwrap();
        let base_url = std::env::var("MEELTING_LOCAL_BASE_URL").unwrap();
        let providers = llm::providers()
            .into_iter()
            .map(|mut provider| {
                if provider.key == key {
                    provider.base_url = base_url.clone();
                }
                provider
            })
            .collect();
        let engine = Engine::new(
            office.fixture.pool.clone(),
            providers,
            crate::domain::agent_run::Limits::default(),
            office.fixture.locks.clone(),
            Arc::new(|_: RunEvent| {}),
        );
        engine.set_tools_enabled(true);
        let selection = Selection::Agent(office.agent_id.clone());
        let InterventionSent::Sent { run: Some(run), .. } = intervention_send::run(
            &engine,
            Intervention {
                room_id: &office.room_id,
                body,
                selection: &selection,
                mention_ids: &[],
                client_request_id: &Uuid::now_v7().to_string(),
            },
        )
        .await
        .unwrap() else {
            panic!("expected a run");
        };

        let mut done = None;
        for _ in 0..3000 {
            let runs = agent_runs::for_room(&engine.pool, &office.room_id, 10)
                .await
                .unwrap();
            let current = runs.into_iter().find(|entry| entry.id == run.id).unwrap();
            if matches!(current.status, RunStatus::Succeeded | RunStatus::Failed) {
                done = Some(current);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let run = done.expect("the run never finished in 5 minutes");
        let calls = sqlx::query_as(
            "SELECT ifnull(tool, '-'), status, error_code FROM tool_calls ORDER BY position",
        )
        .fetch_all(&engine.pool)
        .await
        .unwrap();
        let reply = sqlx::query_scalar(
            "SELECT body FROM messages WHERE agent_id IS NOT NULL ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&engine.pool)
        .await
        .unwrap();
        println!("run {:?} error {:?}", run.status, run.error_kind);
        println!("tool calls: {calls:?}");
        println!("reply: {reply:?}");
        RealRun {
            engine,
            run,
            calls,
            reply,
        }
    }

    #[tokio::test]
    #[ignore = "needs a running local runtime: MEELTING_LOCAL_PROVIDER (ollama or lmstudio), MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL naming an installed model that declares native tool support; set MEELTING_LOCAL_EXPECT_TOOLS to require a successful tool call"]
    async fn a_real_local_model_runs_the_whole_tool_cycle_in_a_real_environment() {
        let key = std::env::var("MEELTING_LOCAL_PROVIDER").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();
        let office = office(&key, &model).await;
        let task = office.task(true, true).await;
        let path = PathBuf::from(office.prepared(&task).await);
        std::fs::create_dir_all(path.join("src")).unwrap();
        std::fs::write(
            path.join("src/main.rs"),
            "fn main() { println!(\"hello\"); }\n",
        )
        .unwrap();
        std::fs::write(path.join("README.md"), "Test project\nedited line\n").unwrap();

        let real = real_local_run(
            &office,
            "Use your tools. List the files in the repository root, read README.md, and show the git status. Then tell me the first line of README.md.",
        )
        .await;

        assert_eq!(
            real.run.status,
            RunStatus::Succeeded,
            "{:?}",
            real.run.error_kind
        );
        assert!(
            real.calls.iter().all(|call| call.1 != "running"),
            "{:?}",
            real.calls
        );
        if std::env::var("MEELTING_LOCAL_EXPECT_TOOLS").is_ok() {
            assert!(
                real.calls.iter().any(|call| call.1 == "succeeded"),
                "the model called no tool successfully: {:?}",
                real.calls
            );
        }
    }

    #[tokio::test]
    #[ignore = "needs a running local runtime: MEELTING_LOCAL_PROVIDER (ollama or lmstudio), MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL naming an installed model that declares native tool support"]
    async fn a_real_local_model_changes_a_file_that_changes_shows_and_the_repository_keeps() {
        let key = std::env::var("MEELTING_LOCAL_PROVIDER").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();
        let office = office(&key, &model).await;
        let task = office.task(true, true).await;
        let path = PathBuf::from(office.prepared(&task).await);

        let real = real_local_run(
            &office,
            "Use your tools to change README.md in the task environment: replace its first line with exactly `Hello from Ada`. Then confirm what you changed.",
        )
        .await;

        assert_eq!(
            real.run.status,
            RunStatus::Succeeded,
            "{:?}",
            real.run.error_kind
        );
        assert!(real.reply.is_some());
        assert!(
            real.calls
                .iter()
                .any(|call| matches!(call.0.as_str(), "write_file" | "edit_file")
                    && call.1 == "succeeded"),
            "{:?}",
            real.calls
        );
        let readme = std::fs::read_to_string(path.join("README.md")).unwrap();
        assert_eq!(readme.lines().next(), Some("Hello from Ada"), "{readme:?}");
        assert_eq!(
            std::fs::read_to_string(office.fixture.repo.join("README.md")).unwrap(),
            "hello\n"
        );
        let original = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&office.fixture.repo)
            .output()
            .unwrap();
        assert!(
            original.stdout.is_empty(),
            "{:?}",
            String::from_utf8_lossy(&original.stdout)
        );
        let review = crate::services::environment_review::run(
            &real.engine.pool,
            &office.fixture.locks,
            &crate::services::review_registry::ReviewRegistry::default(),
            &task,
            async |_: &std::path::Path| Ok(()),
        )
        .await
        .unwrap();
        assert!(
            review
                .entries
                .iter()
                .any(|entry| entry.path == "README.md" && entry.unstaged.is_some()),
            "{:?}",
            review.entries
        );
    }

    #[tokio::test]
    async fn the_five_tools_and_a_rejected_call_work_through_the_real_engine_and_audit() {
        let office = office("ollama", "qwen3:4b").await;
        let task = office.task(true, true).await;
        let path = PathBuf::from(office.prepared(&task).await);
        std::fs::write(path.join("README.md"), "hello\nneedle-line\n").unwrap();
        let calls = serde_json::json!([
            {"index": 0, "id": "c1", "function": {"name": "list_dir", "arguments": "{}"}},
            {"index": 1, "id": "c2", "function": {"name": "read_file", "arguments": "{\"path\":\"README.md\"}"}},
            {"index": 2, "id": "c3", "function": {"name": "search", "arguments": "{\"query\":\"needle\"}"}},
            {"index": 3, "id": "c4", "function": {"name": "git_status", "arguments": "{}"}},
            {"index": 4, "id": "c5", "function": {"name": "git_diff", "arguments": "{\"path\":\"README.md\"}"}},
            {"index": 5, "id": "c6", "function": {"name": "read_file", "arguments": "{SECRET-PAYLOAD"}}
        ]);
        let first = sse(&[
            &serde_json::json!({"choices": [{"delta": {"tool_calls": calls}}]}).to_string(),
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ]);
        let server = local_server(
            vec![shown(serde_json::json!(["completion", "tools"]))],
            vec![first, chat(&["All five worked."])],
        )
        .await;
        let recorded = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = recorded.clone();
        let engine = Engine::new(
            office.fixture.pool.clone(),
            providers_at(&server.uri()),
            fast_limits(),
            office.fixture.locks.clone(),
            Arc::new(move |event: RunEvent| {
                if let RunEvent::ToolCall(activity) = event {
                    sink.lock().unwrap().push(activity);
                }
            }),
        );
        engine.set_tools_enabled(true);

        let run = ask(&engine, &office).await;
        let done = finished(&engine, &office.room_id, &run.id).await;

        assert_eq!(done.status, RunStatus::Succeeded);
        type Row = (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let stored: Vec<Row> =
            sqlx::query_as(
                "SELECT ifnull(tool, '-'), status, error_code, arguments, CAST(result_bytes AS TEXT) FROM tool_calls ORDER BY position",
            )
            .fetch_all(&engine.pool)
            .await
            .unwrap();
        let tools: Vec<(&str, &str, Option<&str>)> = stored
            .iter()
            .map(|row| (row.0.as_str(), row.1.as_str(), row.2.as_deref()))
            .collect();
        assert_eq!(
            tools,
            vec![
                ("list_dir", "succeeded", None),
                ("read_file", "succeeded", None),
                ("search", "succeeded", None),
                ("git_status", "succeeded", None),
                ("git_diff", "succeeded", None),
                ("read_file", "failed", Some("invalid_arguments")),
            ]
        );
        assert_eq!(stored[5].3, None);
        let table: String = sqlx::query_scalar(
            "SELECT group_concat(ifnull(arguments, '-') || ifnull(error_code, '-'), '|') FROM tool_calls",
        )
        .fetch_one(&engine.pool)
        .await
        .unwrap();
        assert!(
            !table.contains("SECRET") && !table.contains("needle-line") && !table.contains("hello")
        );
        let sent = bodies(&server).await;
        let second = sent
            .iter()
            .rfind(|body| body.contains("tool_call_id"))
            .unwrap();
        assert!(second.contains("README.md"));
        assert!(second.contains("needle-line"));
        assert!(second.contains(".M README.md"));
        assert!(second.contains("+needle-line"));
        assert!(second.contains("error: invalid_arguments"));
        assert!(!second.contains("SECRET"));
        assert!(!engine.cloud_tools_consent());
        let announced = recorded.lock().unwrap().clone();
        let running = announced.iter().filter(|a| a.status == "running").count();
        assert_eq!(running, 5, "{announced:?}");
        let finals: Vec<&str> = announced
            .iter()
            .filter(|a| a.status != "running")
            .map(|a| a.status.as_str())
            .collect();
        assert_eq!(
            finals,
            [
                "succeeded",
                "succeeded",
                "succeeded",
                "succeeded",
                "succeeded",
                "failed"
            ]
        );
        let detail =
            crate::services::room_get::run(&engine.pool, &engine.registry, &office.room_id)
                .await
                .unwrap();
        assert_eq!(detail.tool_calls.len(), 6);
        assert_eq!(detail.tool_calls[1].tool.as_deref(), Some("read_file"));
        assert_eq!(detail.tool_calls[1].path.as_deref(), Some("README.md"));
        assert_eq!(detail.tool_calls[5].path, None);
        assert_eq!(
            detail.tool_calls[5].error_code.as_deref(),
            Some("invalid_arguments")
        );
    }
}
