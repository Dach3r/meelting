use crate::catalogs::llm::Provider;
use crate::catalogs::{agent_roles, agent_templates};
use crate::domain::team::{AgentDraft, Team};
use crate::repositories::{agents, teams};
use crate::services::session_load::{self, SessionError};
use crate::services::{engine_check, teams_list};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(
    pool: &SqlitePool,
    providers: &[Provider],
    draft: AgentDraft,
) -> Result<Vec<Team>, SessionError> {
    let draft = draft.normalize().ok_or(SessionError::InvalidAgent)?;
    if !agent_roles::exists(&draft.role) {
        return Err(SessionError::InvalidAgent);
    }
    if !engine_check::run(providers, &draft.provider, &draft.model).await {
        return Err(SessionError::InvalidAgent);
    }
    if let Some(key) = &draft.template_key
        && !agent_templates::exists(key)
    {
        return Err(SessionError::InvalidAgent);
    }
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if !teams::exists(pool, &workspace_id, &draft.team_id).await? {
        return Err(SessionError::TeamNotFound);
    }
    agents::insert(pool, &Uuid::now_v7().to_string(), &workspace_id, &draft).await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::agent_run_engine::testing::providers_at;
    use crate::services::{first_launch, team_create, workspace_create, workspace_switch};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn draft(team_id: &str) -> AgentDraft {
        AgentDraft {
            team_id: team_id.into(),
            template_key: None,
            name: "Ana".into(),
            role: "Software Developer".into(),
            description: "Writes code".into(),
            instructions: "Be careful".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        }
    }

    #[tokio::test]
    async fn creates_an_agent_inside_the_team() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();

        let created = run(&pool, &llm::providers(), draft(&team_id))
            .await
            .unwrap();

        assert_eq!(created[0].agents.len(), 1);
        assert_eq!(created[0].agents[0].name, "Ana");
        assert_eq!(created[0].agents[0].role, "Software Developer");
    }

    #[tokio::test]
    async fn rejects_an_invalid_draft() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut invalid = draft(&team_id);
        invalid.model = " ".into();

        assert!(matches!(
            run(&pool, &llm::providers(), invalid).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn rejects_a_role_outside_the_catalog() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut invalid = draft(&team_id);
        invalid.role = "Wizard".into();

        assert!(matches!(
            run(&pool, &llm::providers(), invalid).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn rejects_a_model_outside_the_catalog() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut invalid = draft(&team_id);
        invalid.provider = "openai".into();
        invalid.model = "gpt-5".into();

        assert!(matches!(
            run(&pool, &llm::providers(), invalid).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn hires_only_local_models_installed_in_a_running_server() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "data": [{ "id": "qwen2.5:0.5b" }] })),
            )
            .mount(&server)
            .await;
        let local = |model: &str| AgentDraft {
            provider: "ollama".into(),
            model: model.into(),
            ..draft(&team_id)
        };

        assert!(matches!(
            run(
                &pool,
                &providers_at(&server.uri()),
                local("qwen2.5-coder:32b")
            )
            .await,
            Err(SessionError::InvalidAgent)
        ));
        assert!(matches!(
            run(
                &pool,
                &providers_at("http://127.0.0.1:1"),
                local("qwen2.5:0.5b")
            )
            .await,
            Err(SessionError::InvalidAgent)
        ));
        let hired = run(&pool, &providers_at(&server.uri()), local("qwen2.5:0.5b"))
            .await
            .unwrap();
        assert_eq!(hired[0].agents.len(), 1);
    }

    #[tokio::test]
    async fn hires_from_a_known_template_and_rejects_an_unknown_one() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut hired = draft(&team_id);
        hired.template_key = Some("qa_engineer".into());
        let mut unknown = draft(&team_id);
        unknown.template_key = Some("missing".into());

        assert!(run(&pool, &llm::providers(), hired).await.is_ok());
        assert!(matches!(
            run(&pool, &llm::providers(), unknown).await,
            Err(SessionError::InvalidAgent)
        ));
        let stored: Option<String> = sqlx::query_scalar("SELECT template_key FROM agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored.as_deref(), Some("qa_engineer"));
    }

    #[tokio::test]
    async fn rejects_unknown_teams_and_teams_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        workspace_create::run(&pool, "Second").await.unwrap();

        assert!(matches!(
            run(&pool, &llm::providers(), draft("missing")).await,
            Err(SessionError::TeamNotFound)
        ));
        assert!(matches!(
            run(&pool, &llm::providers(), draft(&team_id)).await,
            Err(SessionError::TeamNotFound)
        ));
        workspace_switch::run(&pool, &first.current_workspace_id)
            .await
            .unwrap();
        assert!(run(&pool, &llm::providers(), draft(&team_id)).await.is_ok());
    }
}
