use crate::domain::team::{Agent, Team};
use crate::repositories::{agents, canvas_positions, teams};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Team>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let positions: HashMap<String, (f64, f64)> = canvas_positions::list(pool, &workspace_id)
        .await?
        .into_iter()
        .map(|position| (position.node_id, (position.x, position.y)))
        .collect();
    let all_agents: Vec<Agent> = agents::list(pool, &workspace_id)
        .await?
        .into_iter()
        .map(|agent| {
            let (x, y) = positions.get(&agent.id).copied().unwrap_or_default();
            Agent { x, y, ..agent }
        })
        .collect();
    Ok(teams::list(pool, &workspace_id)
        .await?
        .into_iter()
        .map(|(id, name)| {
            let (x, y) = positions.get(&id).copied().unwrap_or_default();
            Team {
                agents: all_agents
                    .iter()
                    .filter(|agent| agent.team_id == id)
                    .cloned()
                    .collect(),
                id,
                name,
                x,
                y,
            }
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

    #[tokio::test]
    async fn applies_the_saved_canvas_positions() {
        let (pool, _dir) = crate::repositories::database::open_for_test().await;
        let w1 = first_launch::run(&pool, Locale::En, "First")
            .await
            .unwrap()
            .current_workspace_id;
        sqlx::query("INSERT INTO teams VALUES ('t1', ?1, 'General', 'x', 'x'), ('t2', ?1, 'Empty', 'y', 'y')")
            .bind(&w1)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents VALUES ('a1', ?1, 't1', NULL, 'Ana', 'Developer', '', '', 'openai', 'gpt', 'x', 'x')")
            .bind(&w1)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO canvas_positions VALUES (?1, 't1', 12.5, -3.0), (?1, 'a1', 4.0, 8.0)",
        )
        .bind(&w1)
        .execute(&pool)
        .await
        .unwrap();

        let teams = run(&pool).await.unwrap();

        assert_eq!((teams[0].x, teams[0].y), (12.5, -3.0));
        assert_eq!((teams[0].agents[0].x, teams[0].agents[0].y), (4.0, 8.0));
        assert_eq!((teams[1].x, teams[1].y), (0.0, 0.0));
    }
}
