use crate::domain::team::Team;
use crate::repositories::canvas_positions;
use crate::services::session_load::{self, SessionError};
use crate::services::teams_list;
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Team>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    canvas_positions::clear(pool, &workspace_id).await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(run(&pool).await, Err(SessionError::NotSetUp)));
    }

    #[tokio::test]
    async fn returns_every_node_to_its_default_position() {
        let (pool, _dir) = open_for_test().await;
        let workspace_id = first_launch::run(&pool, Locale::En, "First")
            .await
            .unwrap()
            .current_workspace_id;
        sqlx::query("INSERT INTO teams VALUES ('t1', ?1, 'General', 'x', 'x')")
            .bind(&workspace_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO canvas_positions VALUES (?1, 't1', 12.5, -3.0)")
            .bind(&workspace_id)
            .execute(&pool)
            .await
            .unwrap();

        let teams = run(&pool).await.unwrap();

        assert_eq!((teams[0].x, teams[0].y), (0.0, 0.0));
    }
}
