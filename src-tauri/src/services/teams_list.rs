use crate::domain::team::Team;
use crate::repositories::{agents, teams};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Team>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let all_agents = agents::list(pool, &workspace_id).await?;
    Ok(teams::list(pool, &workspace_id)
        .await?
        .into_iter()
        .map(|(id, name)| Team {
            agents: all_agents
                .iter()
                .filter(|agent| agent.team_id == id)
                .cloned()
                .collect(),
            id,
            name,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::services::{first_launch, workspace_create};

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = crate::repositories::database::open_for_test().await;

        assert!(matches!(run(&pool).await, Err(SessionError::NotSetUp)));
    }

    #[tokio::test]
    async fn groups_agents_under_their_team_in_the_current_workspace() {
        let (pool, _dir) = crate::repositories::database::open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let w1 = first.current_workspace_id;
        let second = workspace_create::run(&pool, "Second").await.unwrap();
        let w2 = second.current_workspace_id;
        sqlx::query("INSERT INTO teams VALUES ('t1', ?1, 'General', 'x', 'x'), ('t2', ?1, 'Empty', 'y', 'y'), ('t3', ?2, 'Other', 'x', 'x')")
            .bind(&w1)
            .bind(&w2)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents VALUES ('a1', ?1, 't1', NULL, 'Ana', 'Developer', '', '', 'openai', 'gpt', 'x', 'x'), ('a2', ?2, 't3', NULL, 'Bob', 'QA', '', '', 'openai', 'gpt', 'x', 'x')")
            .bind(&w1)
            .bind(&w2)
            .execute(&pool)
            .await
            .unwrap();
        crate::services::workspace_switch::run(&pool, &w1)
            .await
            .unwrap();

        let teams = run(&pool).await.unwrap();

        assert_eq!(teams.len(), 2);
        assert_eq!(teams[0].name, "General");
        assert_eq!(teams[0].agents.len(), 1);
        assert_eq!(teams[0].agents[0].name, "Ana");
        assert!(teams[1].agents.is_empty());
    }
}
