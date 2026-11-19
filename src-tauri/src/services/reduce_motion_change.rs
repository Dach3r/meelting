use crate::domain::reduce_motion::ReduceMotion;
use crate::domain::session::Session;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, reduce_motion: ReduceMotion) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_reduce_motion(pool, reduce_motion).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn follows_the_system_by_default_and_saves_the_new_value() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        assert_eq!(first.reduce_motion, ReduceMotion::System);

        let session = run(&pool, ReduceMotion::On).await.unwrap();

        assert_eq!(session.reduce_motion, ReduceMotion::On);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, ReduceMotion::Off).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
