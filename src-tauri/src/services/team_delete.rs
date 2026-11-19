use crate::domain::team::Team;
use crate::repositories::{canvas_positions, teams};
use crate::services::session_load::{self, SessionError};
use crate::services::teams_list;
use sqlx::SqlitePool;

const RESTRICT_VIOLATION: &str = "1811";

pub async fn run(pool: &SqlitePool, team_id: &str) -> Result<Vec<Team>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let deleted = teams::delete(&mut *transaction, &workspace_id, team_id)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(database)
                if database.is_foreign_key_violation()
                    || database.code().as_deref() == Some(RESTRICT_VIOLATION) =>
            {
                SessionError::TeamHasAgents
            }
            _ => SessionError::Database(error),
        })?;
    if !deleted {
        return Err(SessionError::TeamNotFound);
    }
    canvas_positions::delete(&mut *transaction, &workspace_id, team_id).await?;
    transaction.commit().await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{agent_create, first_launch, team_create, workspace_create};

    async fn setup(pool: &SqlitePool) -> Vec<Team> {
        first_launch::run(pool, Locale::En, "First").await.unwrap();
        team_create::run(pool, "General").await.unwrap();
        team_create::run(pool, "Design").await.unwrap()
    }

    async fn hire(pool: &SqlitePool, team_id: &str) {
        let draft = AgentDraft {
            team_id: team_id.to_string(),
            template_key: None,
            name: "Ana".into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        agent_create::run(pool, &crate::catalogs::llm::providers(), draft)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn deletes_an_empty_team_and_returns_the_updated_teams() {
        let (pool, _dir) = open_for_test().await;
        let teams = setup(&pool).await;

        let remaining = run(&pool, &teams[1].id).await.unwrap();

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, teams[0].id);
    }

    #[tokio::test]
    async fn removes_the_saved_position_of_the_team() {
        let (pool, _dir) = open_for_test().await;
        let teams = setup(&pool).await;
        sqlx::query("INSERT INTO canvas_positions SELECT workspace_id, id, 1.0, 2.0 FROM teams")
            .execute(&pool)
            .await
            .unwrap();

        run(&pool, &teams[1].id).await.unwrap();

        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_positions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 1);
    }

    #[tokio::test]
    async fn cannot_delete_a_team_with_agents() {
        let (pool, _dir) = open_for_test().await;
        let teams = setup(&pool).await;
        hire(&pool, &teams[0].id).await;
        let r = run(&pool, &teams[0].id).await;
        if let Err(SessionError::Database(e)) = &r {
            println!("RES {e:?}");
        }

        assert!(matches!(
            run(&pool, &teams[0].id).await,
            Err(SessionError::TeamHasAgents)
        ));
        assert_eq!(teams_list::run(&pool).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn rejects_unknown_teams_and_teams_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        let teams = setup(&pool).await;

        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::TeamNotFound)
        ));
        workspace_create::run(&pool, "Second").await.unwrap();
        assert!(matches!(
            run(&pool, &teams[0].id).await,
            Err(SessionError::TeamNotFound)
        ));
    }
}
