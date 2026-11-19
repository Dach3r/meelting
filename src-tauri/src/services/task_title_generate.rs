use crate::catalogs::llm::Location;
use crate::domain::locale::Locale;
use crate::domain::prompt;
use crate::domain::provider_key;
use crate::domain::task::{Task, clean_generated_title, provisional_title};
use crate::repositories::{agents, profile, tasks};
use crate::services::agent_run_engine::Engine;
use crate::services::agent_run_execute::{Target, resolve};
use crate::services::session_load;
use crate::services::tasks_list;
use crate::system::llm_stream::{Failure, StreamRequest};
use crate::system::{keychain, llm_call};
use std::sync::Arc;

pub fn spawn(engine: &Arc<Engine>, task: Task, on_updated: impl FnOnce(Task) + Send + 'static) {
    if task.title != provisional_title(&task.description) || task.agent_ids.is_empty() {
        return;
    }
    let engine = engine.clone();
    tokio::spawn(async move {
        if let Some(updated) = run(&engine, &task).await {
            on_updated(updated);
        }
    });
}

async fn run(engine: &Engine, task: &Task) -> Option<Task> {
    let workspace_id = session_load::require(&engine.pool)
        .await
        .ok()?
        .current_workspace_id;
    let agent = agents::get(&engine.pool, &workspace_id, task.agent_ids.first()?)
        .await
        .ok()??;
    let Target {
        provider,
        model_max,
    } = resolve(&engine.providers, &agent.provider, &agent.model)?;
    let api_key = match provider.location {
        Location::Local => None,
        Location::Cloud => Some(
            keychain::read(&provider_key::secret_name(&workspace_id, &agent.provider)).ok()??,
        ),
    };
    let locale = profile::get(&engine.pool)
        .await
        .ok()?
        .map_or(Locale::En, |profile| profile.locale);
    let prompt = prompt::task_title(locale, &task.description);
    let limits = engine.limits.at(provider.location);
    let request = StreamRequest {
        base_url: &provider.base_url,
        api_key: api_key.as_deref(),
        model: &agent.model,
        max_output_tokens: engine.limits.output_tokens(provider.location, model_max),
        prompt: &prompt,
        connect_timeout: limits.connect_timeout,
        idle_timeout: limits.idle_timeout,
    };
    let _permit = engine.slots(provider.location).acquire().await.ok()?;
    let mut text = String::new();
    let mut on_text = |chunk: &str| -> Result<(), Failure> {
        text.push_str(chunk);
        Ok(())
    };
    tokio::time::timeout(
        limits.max_duration,
        llm_call::stream(provider, &request, &mut on_text),
    )
    .await
    .ok()?
    .ok()?;
    let title = clean_generated_title(&text)?;
    if !tasks::set_generated_title(
        &engine.pool,
        &workspace_id,
        &task.id,
        &task.description,
        &title,
    )
    .await
    .ok()?
    {
        return None;
    }
    tasks_list::one(&engine.pool, &workspace_id, &task.id)
        .await
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{TaskDraft, TaskPriority};
    use crate::services::agent_run_engine::testing::{Harness, chat, fast_limits, harness};
    use crate::services::task_save;
    use std::sync::Mutex;
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer};

    async fn created(harness: &Harness, description: &str, agents: Vec<String>) -> Task {
        task_save::create(
            &harness.engine.pool,
            TaskDraft {
                description: description.into(),
                priority: TaskPriority::Medium,
                repository_id: None,
                base_ref: None,
                agent_ids: agents,
            },
            None,
        )
        .await
        .unwrap()
    }

    async fn updated_within(engine: &Arc<Engine>, task: Task) -> Option<Task> {
        let slot: Arc<Mutex<Option<Task>>> = Arc::default();
        let sink = slot.clone();
        spawn(engine, task, move |updated| {
            *sink.lock().unwrap() = Some(updated)
        });
        for _ in 0..30 {
            if let Some(updated) = slot.lock().unwrap().take() {
                return Some(updated);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    }

    #[tokio::test]
    async fn the_first_assigned_agent_writes_the_title() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("Fix the login timeout"))
            .respond_with(chat(&["\"Login ", "timeout fix.\""]))
            .expect(1)
            .mount(&server)
            .await;
        let harness = harness(&server.uri(), fast_limits(), &[("ollama", "qwen")]).await;
        let task = created(
            &harness,
            "Fix the login timeout\nIt expires too early",
            harness.agents.clone(),
        )
        .await;

        let updated = updated_within(&harness.engine, task).await.unwrap();

        assert_eq!(updated.title, "Login timeout fix");
        assert_eq!(
            updated.description,
            "Fix the login timeout\nIt expires too early"
        );
    }

    #[tokio::test]
    async fn keeps_the_provisional_title_when_the_agent_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let harness = harness(&server.uri(), fast_limits(), &[("ollama", "qwen")]).await;
        let task = created(&harness, "Fix login", harness.agents.clone()).await;

        assert!(
            updated_within(&harness.engine, task.clone())
                .await
                .is_none()
        );

        let stored = tasks_list::run(&harness.engine.pool).await.unwrap();
        assert_eq!(stored[0].title, "Fix login");
    }

    #[tokio::test]
    async fn does_nothing_without_agents_or_when_the_title_is_already_generated() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["Never used"]))
            .expect(0)
            .mount(&server)
            .await;
        let harness = harness(&server.uri(), fast_limits(), &[("ollama", "qwen")]).await;
        let without_agents = created(&harness, "Fix login", Vec::new()).await;
        let mut generated = created(&harness, "Fix signup", harness.agents.clone()).await;
        generated.title = "Signup fix".into();

        assert!(
            updated_within(&harness.engine, without_agents)
                .await
                .is_none()
        );
        assert!(updated_within(&harness.engine, generated).await.is_none());
    }

    #[tokio::test]
    async fn ignores_a_title_for_a_description_that_changed_meanwhile() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(chat(&["Old title"]).set_delay(Duration::from_millis(300)))
            .mount(&server)
            .await;
        let harness = harness(&server.uri(), fast_limits(), &[("ollama", "qwen")]).await;
        let task = created(&harness, "Old text", harness.agents.clone()).await;
        let pool = harness.engine.pool.clone();
        let id = task.id.clone();
        let slot: Arc<Mutex<Option<Task>>> = Arc::default();
        let sink = slot.clone();
        spawn(&harness.engine, task, move |updated| {
            *sink.lock().unwrap() = Some(updated)
        });

        task_save::update(
            &pool,
            &id,
            TaskDraft {
                description: "New text".into(),
                priority: TaskPriority::Medium,
                repository_id: None,
                base_ref: None,
                agent_ids: Vec::new(),
            },
        )
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(900)).await;

        assert!(slot.lock().unwrap().is_none());
        assert_eq!(tasks_list::run(&pool).await.unwrap()[0].title, "New text");
    }
}
