use crate::domain::team::Team;
use crate::repositories::{agents, canvas_positions};
use crate::services::session_load::{self, SessionError};
use crate::services::teams_list;
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, agent_id: &str) -> Result<Vec<Team>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if !agents::delete(pool, &workspace_id, agent_id).await? {
        return Err(SessionError::AgentNotFound);
    }
    canvas_positions::delete(pool, &workspace_id, agent_id).await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{agent_create, first_launch, team_create, workspace_create};

    async fn setup(pool: &SqlitePool) -> String {
        first_launch::run(pool, Locale::En, "First").await.unwrap();
        let teams = team_create::run(pool, "General").await.unwrap();
        let draft = AgentDraft {
            team_id: teams[0].id.clone(),
            template_key: None,
            name: "Ana".into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        agent_create::run(pool, draft).await.unwrap()[0].agents[0]
            .id
            .clone()
    }

    #[tokio::test]
    async fn deletes_the_agent_and_returns_the_updated_teams() {
        let (pool, _dir) = open_for_test().await;
        let agent_id = setup(&pool).await;

        let teams = run(&pool, &agent_id).await.unwrap();

        assert_eq!(teams.len(), 1);
        assert!(teams[0].agents.is_empty());
    }

    #[tokio::test]
    async fn removes_the_saved_position_of_the_agent() {
        let (pool, _dir) = open_for_test().await;
        let agent_id = setup(&pool).await;
        sqlx::query("INSERT INTO canvas_positions SELECT workspace_id, id, 1.0, 2.0 FROM agents")
            .execute(&pool)
            .await
            .unwrap();

        run(&pool, &agent_id).await.unwrap();

        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_positions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }

    #[tokio::test]
    async fn rejects_unknown_agents_and_agents_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        let agent_id = setup(&pool).await;

        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::AgentNotFound)
        ));
        workspace_create::run(&pool, "Second").await.unwrap();
        assert!(matches!(
            run(&pool, &agent_id).await,
            Err(SessionError::AgentNotFound)
        ));
    }
}
