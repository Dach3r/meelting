use crate::domain::team::AgentDetail;
use crate::repositories::agents;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, agent_id: &str) -> Result<AgentDetail, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    agents::get(pool, &workspace_id, agent_id)
        .await?
        .ok_or(SessionError::AgentNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{agent_create, first_launch, team_create};

    #[tokio::test]
    async fn returns_the_full_agent_or_not_found() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let team_id = team_create::run(&pool, "General").await.unwrap()[0]
            .id
            .clone();
        let draft = AgentDraft {
            team_id,
            template_key: None,
            name: "Ana".into(),
            role: "Software Developer".into(),
            description: "Writes code".into(),
            instructions: "Be careful".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        let agent_id = agent_create::run(&pool, draft).await.unwrap()[0].agents[0]
            .id
            .clone();

        let agent = run(&pool, &agent_id).await.unwrap();

        assert_eq!(agent.instructions, "Be careful");
        assert_eq!(agent.model, "claude-sonnet-5-5");
        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::AgentNotFound)
        ));
    }
}
