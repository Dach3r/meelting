use crate::catalogs::agent_roles;
use crate::catalogs::llm::Provider;
use crate::domain::team::{AgentEdit, Team};
use crate::repositories::{agents, canvas_positions, teams};
use crate::services::session_load::{self, SessionError};
use crate::services::{engine_check, teams_list};
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    providers: &[Provider],
    agent_id: &str,
    edit: AgentEdit,
) -> Result<Vec<Team>, SessionError> {
    let edit = edit.normalize().ok_or(SessionError::InvalidAgent)?;
    if !agent_roles::exists(&edit.role) {
        return Err(SessionError::InvalidAgent);
    }
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin().await?;
    if !teams::exists(&mut *transaction, &workspace_id, &edit.team_id).await? {
        return Err(SessionError::TeamNotFound);
    }
    let current = agents::get(&mut *transaction, &workspace_id, agent_id)
        .await?
        .ok_or(SessionError::AgentNotFound)?;
    let engine_changed = current.provider != edit.provider || current.model != edit.model;
    if engine_changed && !engine_check::run(providers, &edit.provider, &edit.model).await {
        return Err(SessionError::InvalidAgent);
    }
    agents::update(&mut *transaction, &workspace_id, agent_id, &edit)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(database) if database.is_foreign_key_violation() => SessionError::TeamNotFound,
            _ => SessionError::Database(error),
        })?;
    if current.team_id != edit.team_id {
        canvas_positions::delete(&mut *transaction, &workspace_id, agent_id).await?;
    }
    transaction.commit().await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm;
    use crate::domain::locale::Locale;
    use crate::domain::team::{AgentDraft, Position};
    use crate::repositories::database::open_for_test;
    use crate::services::agent_run_engine::testing::providers_at;
    use crate::services::{agent_create, first_launch, team_create, workspace_create};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup(pool: &SqlitePool) -> (String, String, String) {
        first_launch::run(pool, Locale::En, "First").await.unwrap();
        team_create::run(pool, "General").await.unwrap();
        let teams = team_create::run(pool, "Platform").await.unwrap();
        let draft = AgentDraft {
            team_id: teams[0].id.clone(),
            template_key: Some("qa_engineer".into()),
            name: "Ana".into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        let agent_id = agent_create::run(pool, &llm::providers(), draft)
            .await
            .unwrap()[0]
            .agents[0]
            .id
            .clone();
        (agent_id, teams[0].id.clone(), teams[1].id.clone())
    }

    async fn ollama(models: &[&str]) -> MockServer {
        let server = MockServer::start().await;
        let data: Vec<_> = models.iter().map(|id| json!({ "id": id })).collect();
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
            .mount(&server)
            .await;
        server
    }

    fn edit(team_id: &str) -> AgentEdit {
        AgentEdit {
            team_id: team_id.into(),
            name: "Bea".into(),
            role: "Software Developer".into(),
            description: "Writes code".into(),
            instructions: "Be careful".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        }
    }

    #[tokio::test]
    async fn edits_the_agent_and_moves_it_to_another_team() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, _, other_team) = setup(&pool).await;

        let teams = run(&pool, &llm::providers(), &agent_id, edit(&other_team))
            .await
            .unwrap();

        assert!(teams[0].agents.is_empty());
        assert_eq!(teams[1].agents[0].name, "Bea");
        assert_eq!(teams[1].agents[0].role, "Software Developer");
    }

    #[tokio::test]
    async fn resets_the_canvas_position_only_when_the_team_changes() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, other_team) = setup(&pool).await;
        let workspace_id = session_load::require(&pool)
            .await
            .unwrap()
            .current_workspace_id;
        let position = |x: f64| Position {
            node_id: agent_id.clone(),
            x,
            y: 5.0,
        };
        canvas_positions::save(&pool, &workspace_id, &position(10.0))
            .await
            .unwrap();

        run(&pool, &llm::providers(), &agent_id, edit(&team_id))
            .await
            .unwrap();
        let kept = canvas_positions::list(&pool, &workspace_id).await.unwrap();
        assert_eq!(kept.len(), 1);

        run(&pool, &llm::providers(), &agent_id, edit(&other_team))
            .await
            .unwrap();
        let reset = canvas_positions::list(&pool, &workspace_id).await.unwrap();
        assert!(reset.is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_edits() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let mut blank = edit(&team_id);
        blank.name = " ".into();
        let mut wizard = edit(&team_id);
        wizard.role = "Wizard".into();

        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, blank).await,
            Err(SessionError::InvalidAgent)
        ));
        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, wizard).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn rejects_unknown_agents_and_teams_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;

        assert!(matches!(
            run(&pool, &llm::providers(), "missing", edit(&team_id)).await,
            Err(SessionError::AgentNotFound)
        ));
        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, edit("missing")).await,
            Err(SessionError::TeamNotFound)
        ));
        workspace_create::run(&pool, "Second").await.unwrap();
        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, edit(&team_id)).await,
            Err(SessionError::TeamNotFound)
        ));
    }

    #[tokio::test]
    async fn changes_the_provider_and_model_to_an_installed_local_model() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let server = ollama(&["qwen2.5:0.5b"]).await;
        let mut local = edit(&team_id);
        local.provider = "ollama".into();
        local.model = "qwen2.5:0.5b".into();

        run(&pool, &providers_at(&server.uri()), &agent_id, local)
            .await
            .unwrap();

        let workspace_id = session_load::require(&pool)
            .await
            .unwrap()
            .current_workspace_id;
        let agent = agents::get(&pool, &workspace_id, &agent_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (agent.provider.as_str(), agent.model.as_str()),
            ("ollama", "qwen2.5:0.5b")
        );
    }

    #[tokio::test]
    async fn rejects_a_local_model_that_is_not_installed_or_whose_server_is_stopped() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let server = ollama(&["qwen2.5:0.5b"]).await;
        let mut missing = edit(&team_id);
        missing.provider = "ollama".into();
        missing.model = "qwen2.5-coder:32b".into();
        let mut installed = edit(&team_id);
        installed.provider = "ollama".into();
        installed.model = "qwen2.5:0.5b".into();

        assert!(matches!(
            run(&pool, &providers_at(&server.uri()), &agent_id, missing).await,
            Err(SessionError::InvalidAgent)
        ));
        assert!(matches!(
            run(
                &pool,
                &providers_at("http://127.0.0.1:1"),
                &agent_id,
                installed
            )
            .await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn an_agent_on_an_uninstalled_local_model_stays_editable() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        sqlx::query("UPDATE agents SET provider = 'ollama', model = 'qwen2.5-coder:32b'")
            .execute(&pool)
            .await
            .unwrap();
        let mut rename = edit(&team_id);
        rename.provider = "ollama".into();
        rename.model = "qwen2.5-coder:32b".into();

        run(
            &pool,
            &providers_at("http://127.0.0.1:1"),
            &agent_id,
            rename,
        )
        .await
        .unwrap();

        let name: String = sqlx::query_scalar("SELECT name FROM agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Bea");
    }

    #[tokio::test]
    async fn rejects_a_new_pair_outside_the_catalog() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let mut retired = edit(&team_id);
        retired.provider = "openai".into();
        retired.model = "gpt-5".into();
        let mut mismatched = edit(&team_id);
        mismatched.model = "gpt-6.1-sol".into();

        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, retired).await,
            Err(SessionError::InvalidAgent)
        ));
        assert!(matches!(
            run(&pool, &llm::providers(), &agent_id, mismatched).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn an_agent_on_a_retired_model_stays_editable_and_can_move_to_a_current_one() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        sqlx::query("UPDATE agents SET provider = 'openai', model = 'gpt-5'")
            .execute(&pool)
            .await
            .unwrap();
        let mut rename = edit(&team_id);
        rename.provider = "openai".into();
        rename.model = "gpt-5".into();

        run(&pool, &llm::providers(), &agent_id, rename)
            .await
            .unwrap();
        let mut upgrade = edit(&team_id);
        upgrade.provider = "openai".into();
        upgrade.model = "gpt-6.1-sol".into();
        run(&pool, &llm::providers(), &agent_id, upgrade)
            .await
            .unwrap();

        let model: String = sqlx::query_scalar("SELECT model FROM agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(model, "gpt-6.1-sol");
    }

    #[tokio::test]
    async fn existing_runs_keep_the_provider_and_model_they_copied() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let room = crate::services::room_create::run(&pool, "Daily", vec![agent_id.clone()])
            .await
            .unwrap();
        let request = crate::services::message_send::run(&pool, &room.id, "hola")
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO agent_runs (id, room_id, agent_id, request_message_id, client_request_id, status, attempts, provider, model, created_at) \
             VALUES ('run1', ?, ?, ?, '0196f3a0-0000-7000-8000-000000000001', 'queued', 0, 'anthropic', 'claude-sonnet-5-5', ?)",
        )
        .bind(&room.id)
        .bind(&agent_id)
        .bind(&request.id)
        .bind(&request.created_at)
        .execute(&pool)
        .await
        .unwrap();
        let mut upgrade = edit(&team_id);
        upgrade.provider = "openai".into();
        upgrade.model = "gpt-6.1-sol".into();

        run(&pool, &llm::providers(), &agent_id, upgrade)
            .await
            .unwrap();

        let copied: (String, String) =
            sqlx::query_as("SELECT provider, model FROM agent_runs WHERE id = 'run1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(copied, ("anthropic".into(), "claude-sonnet-5-5".into()));
    }
}
