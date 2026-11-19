use crate::domain::session::Session;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, loading_enabled: bool) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_loading_enabled(pool, loading_enabled).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn starts_enabled_and_saves_the_new_value() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        assert!(first.loading_enabled);

        let session = run(&pool, false).await.unwrap();

        assert!(!session.loading_enabled);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, false).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
