use crate::domain::team::Position;
use crate::repositories::{agents, canvas_positions, teams};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, positions: Vec<Position>) -> Result<(), SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    for position in positions {
        let known = teams::exists(&mut *transaction, &workspace_id, &position.node_id).await?
            || agents::get(&mut *transaction, &workspace_id, &position.node_id)
                .await?
                .is_some();
        if known {
            canvas_positions::save(&mut *transaction, &workspace_id, &position).await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, teams_list, workspace_create};

    fn position(node_id: &str, x: f64, y: f64) -> Position {
        Position {
            node_id: node_id.into(),
            x,
            y,
        }
    }

    async fn setup(pool: &SqlitePool) -> String {
        let workspace_id = first_launch::run(pool, Locale::En, "First")
            .await
            .unwrap()
            .current_workspace_id;
        sqlx::query("INSERT INTO teams VALUES ('t1', ?1, 'General', 'x', 'x')")
            .bind(&workspace_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents VALUES ('a1', ?1, 't1', NULL, 'Ana', 'Developer', '', '', 'openai', 'gpt', 'x', 'x')")
            .bind(&workspace_id)
            .execute(pool)
            .await
            .unwrap();
        workspace_id
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(
            run(&pool, vec![]).await,
            Err(SessionError::NotSetUp)
        ));
    }

    #[tokio::test]
    async fn saves_team_and_agent_positions_and_ignores_unknown_nodes() {
        let (pool, _dir) = open_for_test().await;
        setup(&pool).await;

        run(
            &pool,
            vec![
                position("t1", 10.0, 20.0),
                position("a1", -4.5, 6.0),
                position("missing", 1.0, 1.0),
            ],
        )
        .await
        .unwrap();

        let teams = teams_list::run(&pool).await.unwrap();
        assert_eq!((teams[0].x, teams[0].y), (10.0, 20.0));
        assert_eq!((teams[0].agents[0].x, teams[0].agents[0].y), (-4.5, 6.0));
        let saved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_positions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(saved, 2);
    }

    #[tokio::test]
    async fn saves_nothing_when_one_position_fails() {
        let (pool, _dir) = open_for_test().await;
        setup(&pool).await;

        let result = run(
            &pool,
            vec![position("t1", 10.0, 20.0), position("a1", f64::NAN, 6.0)],
        )
        .await;

        assert!(matches!(result, Err(SessionError::Database(_))));
        let saved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_positions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(saved, 0);
    }

    #[tokio::test]
    async fn ignores_nodes_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        setup(&pool).await;
        workspace_create::run(&pool, "Second").await.unwrap();

        run(&pool, vec![position("t1", 10.0, 20.0)]).await.unwrap();

        let saved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_positions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(saved, 0);
    }
}
