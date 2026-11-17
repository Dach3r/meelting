use crate::domain::session::Session;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, workspace_id: &str) -> Result<Session, SessionError> {
    let session = session_load::require(pool).await?;
    if !session
        .workspaces
        .iter()
        .any(|workspace| workspace.id == workspace_id)
    {
        return Err(SessionError::WorkspaceNotFound);
    }
    profile::set_current_workspace(pool, workspace_id).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, workspace_create};

    #[tokio::test]
    async fn switches_to_an_existing_workspace() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "First").await.unwrap();
        workspace_create::run(&pool, "Second").await.unwrap();

        let session = run(&pool, &first.current_workspace_id).await.unwrap();

        assert_eq!(session.current_workspace_id, first.current_workspace_id);
    }

    #[tokio::test]
    async fn rejects_unknown_workspace() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "First").await.unwrap();

        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::WorkspaceNotFound)
        ));
        assert_eq!(
            session_load::require(&pool)
                .await
                .unwrap()
                .current_workspace_id,
            first.current_workspace_id
        );
    }

    #[tokio::test]
    async fn requires_first_launch_to_be_completed() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(
            run(&pool, "any").await,
            Err(SessionError::NotSetUp)
        ));
    }
}
