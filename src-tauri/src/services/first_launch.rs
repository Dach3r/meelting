use crate::domain::locale::Locale;
use crate::domain::session::Session;
use crate::domain::workspace::normalize_name;
use crate::repositories::{profile, workspaces};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(
    pool: &SqlitePool,
    locale: Locale,
    workspace_name: &str,
) -> Result<Session, SessionError> {
    let name = normalize_name(workspace_name).ok_or(SessionError::InvalidName)?;
    let id = Uuid::now_v7().to_string();
    let mut transaction = pool.begin().await?;
    workspaces::insert(&mut *transaction, &id, &name).await?;
    profile::create(&mut *transaction, locale, &id).await?;
    transaction.commit().await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn creates_profile_and_first_workspace() {
        let (pool, _dir) = open_for_test().await;

        let session = run(&pool, Locale::Es, "  Acme  ").await.unwrap();

        assert_eq!(session.locale, Locale::Es);
        assert_eq!(session.workspaces.len(), 1);
        assert_eq!(session.workspaces[0].name, "Acme");
        assert_eq!(session.current_workspace_id, session.workspaces[0].id);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn rejects_invalid_name_without_writing() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, Locale::En, "   ").await;

        assert!(matches!(result, Err(SessionError::InvalidName)));
        assert_eq!(session_load::run(&pool).await.unwrap(), None);
        assert!(workspaces::list(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn second_launch_does_not_leave_orphan_workspace() {
        let (pool, _dir) = open_for_test().await;
        run(&pool, Locale::En, "Acme").await.unwrap();

        assert!(run(&pool, Locale::En, "Other").await.is_err());
        assert_eq!(workspaces::list(&pool).await.unwrap().len(), 1);
    }
}
