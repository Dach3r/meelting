use crate::domain::session::Session;
use crate::domain::workspace::normalize_name;
use crate::repositories::{profile, workspaces};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(pool: &SqlitePool, workspace_name: &str) -> Result<Session, SessionError> {
    let name = normalize_name(workspace_name).ok_or(SessionError::InvalidName)?;
    session_load::require(pool).await?;
    let id = Uuid::now_v7().to_string();
    let mut transaction = pool.begin().await?;
    workspaces::insert(&mut *transaction, &id, &name).await?;
    profile::set_current_workspace(&mut *transaction, &id).await?;
    transaction.commit().await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn creates_workspace_and_makes_it_current() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();

        let session = run(&pool, "Second").await.unwrap();

        assert_eq!(session.workspaces.len(), 2);
        let current = session
            .workspaces
            .iter()
            .find(|workspace| workspace.id == session.current_workspace_id)
            .unwrap();
        assert_eq!(current.name, "Second");
    }

    #[tokio::test]
    async fn allows_a_repeated_name() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        let session = run(&pool, "Acme").await.unwrap();

        assert_eq!(session.workspaces.len(), 2);
    }

    #[tokio::test]
    async fn rejects_invalid_name() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        assert!(matches!(
            run(&pool, &"a".repeat(81)).await,
            Err(SessionError::InvalidName)
        ));
        assert_eq!(workspaces::list(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn requires_first_launch_to_be_completed() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(
            run(&pool, "Acme").await,
            Err(SessionError::NotSetUp)
        ));
        assert!(workspaces::list(&pool).await.unwrap().is_empty());
    }
}
