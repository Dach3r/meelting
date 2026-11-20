use crate::catalogs::llm::{Location, Provider};
use crate::domain::agent_run::{AgentRun, Limits, TokenChunk};
use crate::domain::room::Message;
use crate::services::agent_run_execute;
use crate::services::agent_run_registry::Registry;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvent {
    Status(AgentRun),
    Token(TokenChunk),
    MessageCreated(Message),
}

pub type Emit = Arc<dyn Fn(RunEvent) + Send + Sync>;

pub struct Engine {
    pub pool: SqlitePool,
    pub registry: Registry,
    pub providers: Vec<Provider>,
    pub limits: Limits,
    local_slots: Semaphore,
    cloud_slots: Semaphore,
    emit: Emit,
}

impl Engine {
    pub fn new(
        pool: SqlitePool,
        providers: Vec<Provider>,
        limits: Limits,
        emit: Emit,
    ) -> Arc<Self> {
        Arc::new(Engine {
            pool,
            registry: Registry::default(),
            providers,
            local_slots: Semaphore::new(limits.local.concurrency),
            cloud_slots: Semaphore::new(limits.cloud.concurrency),
            limits,
            emit,
        })
    }

    pub fn emit(&self, event: RunEvent) {
        (self.emit)(event);
    }

    pub fn slots(&self, location: Location) -> &Semaphore {
        match location {
            Location::Local => &self.local_slots,
            Location::Cloud => &self.cloud_slots,
        }
    }

    pub fn submit(self: &Arc<Self>, run: AgentRun, round: bool) {
        let cancel = self.registry.insert(&run.id, &run.room_id, round);
        tokio::spawn(agent_run_execute::run(self.clone(), run, cancel));
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use crate::catalogs::llm;
    use crate::domain::agent_run::{Partial, RunStatus};
    use crate::domain::locale::Locale;
    use crate::domain::recipient::Selection;
    use crate::domain::room::Message;
    use crate::domain::team::AgentDraft;
    use crate::repositories::agents;
    use crate::repositories::database::open_for_test;
    use crate::services::intervention_send::{self, Intervention, InterventionSent};
    use crate::services::session_load::{self, SessionError};
    use crate::services::{first_launch, room_create, team_create};
    use std::sync::Mutex;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use uuid::Uuid;
    use wiremock::ResponseTemplate;

    #[derive(Debug)]
    pub struct Sent {
        pub message: Message,
        pub run: AgentRun,
        pub partial: Option<Partial>,
    }

    pub struct Harness {
        pub engine: Arc<Engine>,
        pub events: Arc<Mutex<Vec<RunEvent>>>,
        pub room_id: String,
        pub agents: Vec<String>,
        pub _dir: tempfile::TempDir,
    }

    pub fn fast_limits() -> Limits {
        let mut limits = Limits::default();
        for location in [&mut limits.cloud, &mut limits.local] {
            location.idle_timeout = Duration::from_secs(3);
            location.max_duration = Duration::from_secs(20);
        }
        limits.backoff = [Duration::from_millis(30), Duration::from_millis(30)];
        limits
    }

    pub fn providers_at(base_url: &str) -> Vec<Provider> {
        llm::providers()
            .into_iter()
            .map(|mut provider| {
                provider.base_url = base_url.to_string();
                provider
            })
            .collect()
    }

    pub async fn hire(
        pool: &SqlitePool,
        team_id: &str,
        name: &str,
        role: &str,
        provider: &str,
        model: &str,
    ) -> String {
        let workspace_id = session_load::require(pool)
            .await
            .unwrap()
            .current_workspace_id;
        let id = Uuid::now_v7().to_string();
        let draft = AgentDraft {
            team_id: team_id.into(),
            template_key: None,
            name: name.into(),
            role: role.into(),
            description: String::new(),
            instructions: "Be brief.".into(),
            provider: provider.into(),
            model: model.into(),
        };
        agents::insert(pool, &id, &workspace_id, &draft)
            .await
            .unwrap();
        id
    }

    pub async fn harness(base_url: &str, limits: Limits, agents: &[(&str, &str)]) -> Harness {
        let (pool, dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut ids = Vec::new();
        for (index, (provider, model)) in agents.iter().enumerate() {
            ids.push(
                hire(
                    &pool,
                    &team_id,
                    &format!("Agent{index}"),
                    "QA Engineer",
                    provider,
                    model,
                )
                .await,
            );
        }
        let room = room_create::run(&pool, "Daily", ids.clone()).await.unwrap();
        let events: Arc<Mutex<Vec<RunEvent>>> = Arc::default();
        let recorded = events.clone();
        let engine = Engine::new(
            pool,
            providers_at(base_url),
            limits,
            Arc::new(move |event| recorded.lock().unwrap().push(event)),
        );
        Harness {
            engine,
            events,
            room_id: room.id,
            agents: ids,
            _dir: dir,
        }
    }

    impl Harness {
        pub async fn send(&self, agent: usize, body: &str) -> Result<Sent, SessionError> {
            self.send_with(agent, body, &Uuid::now_v7().to_string())
                .await
        }

        pub async fn send_with(
            &self,
            agent: usize,
            body: &str,
            client_request_id: &str,
        ) -> Result<Sent, SessionError> {
            let selection = Selection::Agent(self.agents[agent].clone());
            match intervention_send::run(
                &self.engine,
                Intervention {
                    room_id: &self.room_id,
                    body,
                    selection: &selection,
                    mention_ids: &[],
                    client_request_id,
                },
            )
            .await?
            {
                InterventionSent::Sent {
                    message,
                    run: Some(run),
                    partial,
                } => Ok(Sent {
                    message,
                    run: *run,
                    partial,
                }),
                other => panic!("expected a run, got {other:?}"),
            }
        }

        pub async fn send_team(&self, body: &str) -> Sent {
            match intervention_send::run(
                &self.engine,
                Intervention {
                    room_id: &self.room_id,
                    body,
                    selection: &Selection::Team,
                    mention_ids: &[],
                    client_request_id: &Uuid::now_v7().to_string(),
                },
            )
            .await
            .unwrap()
            {
                InterventionSent::Sent {
                    message,
                    run: Some(run),
                    partial,
                } => Sent {
                    message,
                    run: *run,
                    partial,
                },
                other => panic!("expected a run, got {other:?}"),
            }
        }

        pub async fn settled_runs(&self, expected: usize) -> Vec<AgentRun> {
            for _ in 0..1000 {
                let runs = crate::repositories::agent_runs::for_room(
                    &self.engine.pool,
                    &self.room_id,
                    1000,
                )
                .await
                .unwrap();
                let active = runs
                    .iter()
                    .any(|run| matches!(run.status, RunStatus::Queued | RunStatus::Running));
                if runs.len() == expected && !active {
                    return runs;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("the room never settled with {expected} runs");
        }

        pub async fn run(&self, run_id: &str) -> AgentRun {
            crate::repositories::agent_runs::for_room(&self.engine.pool, &self.room_id, 1000)
                .await
                .unwrap()
                .into_iter()
                .find(|run| run.id == run_id)
                .expect("run exists")
        }

        pub async fn wait_until(&self, run_id: &str, done: impl Fn(&AgentRun) -> bool) -> AgentRun {
            for _ in 0..1000 {
                let run = self.run(run_id).await;
                if done(&run) {
                    return run;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("run {run_id} never reached the expected state");
        }

        pub async fn finished(&self, run_id: &str) -> AgentRun {
            let run = self
                .wait_until(run_id, |run| {
                    matches!(
                        run.status,
                        RunStatus::Succeeded | RunStatus::Passed | RunStatus::Failed
                    )
                })
                .await;
            for _ in 0..200 {
                if self.engine.registry.partial(run_id).is_none() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            run
        }

        pub async fn count(&self, sql: &'static str) -> i64 {
            sqlx::query_scalar(sql)
                .fetch_one(&self.engine.pool)
                .await
                .unwrap()
        }

        pub fn events(&self) -> Vec<RunEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    pub fn chat_body(parts: &[&str]) -> String {
        let mut body: String = parts
            .iter()
            .map(|part| {
                format!(
                    "data: {}\n\n",
                    serde_json::json!({ "choices": [{ "delta": { "content": part } }] })
                )
            })
            .collect();
        body.push_str("data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n");
        body.push_str(
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":2}}\n\n",
        );
        body.push_str("data: [DONE]\n\n");
        body
    }

    pub fn chat(parts: &[&str]) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_raw(chat_body(parts), "text/event-stream")
    }

    pub fn delta(text: &str) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({ "choices": [{ "delta": { "content": text } }] })
        )
    }

    pub async fn scripted_server(script: Vec<(Duration, String)>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut first = true;
            while let Ok((mut socket, _)) = listener.accept().await {
                let script = if std::mem::take(&mut first) {
                    script.clone()
                } else {
                    vec![(Duration::ZERO, chat_body(&["ok"]))]
                };
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 65536];
                    let _ = socket.read(&mut buffer).await;
                    let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
                    if socket.write_all(head.as_bytes()).await.is_err() {
                        return;
                    }
                    let finishes = script.iter().any(|(_, chunk)| chunk.contains("[DONE]"));
                    for (pause, chunk) in script {
                        tokio::time::sleep(pause).await;
                        if socket.write_all(chunk.as_bytes()).await.is_err() {
                            return;
                        }
                        let _ = socket.flush().await;
                    }
                    if !finishes {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    }
                });
            }
        });
        format!("http://{address}")
    }
}
