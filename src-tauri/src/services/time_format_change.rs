use crate::domain::session::Session;
use crate::domain::time_format::TimeFormat;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, time_format: TimeFormat) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_time_format(pool, time_format).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn starts_automatic_and_saves_the_new_format() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        assert_eq!(first.time_format, TimeFormat::Auto);

        let session = run(&pool, TimeFormat::H24).await.unwrap();

        assert_eq!(session.time_format, TimeFormat::H24);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, TimeFormat::H12).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
