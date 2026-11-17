use crate::domain::team::Team;
use crate::domain::workspace::normalize_name;
use crate::repositories::teams;
use crate::services::session_load::{self, SessionError};
use crate::services::teams_list;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(pool: &SqlitePool, team_name: &str) -> Result<Vec<Team>, SessionError> {
    let name = normalize_name(team_name).ok_or(SessionError::InvalidName)?;
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    teams::insert(pool, &Uuid::now_v7().to_string(), &workspace_id, &name).await?;
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn creates_a_team_in_the_current_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();

        let created = run(&pool, "  Design ").await.unwrap();

        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "Design");
        assert!(created[0].agents.is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_names() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();

        assert!(matches!(
            run(&pool, "  ").await,
            Err(SessionError::InvalidName)
        ));
        assert!(matches!(
            run(&pool, &"a".repeat(81)).await,
            Err(SessionError::InvalidName)
        ));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(
            run(&pool, "Design").await,
            Err(SessionError::NotSetUp)
        ));
    }
}
