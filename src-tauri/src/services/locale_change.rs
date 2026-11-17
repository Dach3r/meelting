use crate::domain::locale::Locale;
use crate::domain::session::Session;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, locale: Locale) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_locale(pool, locale).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn saves_the_new_language() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        let session = run(&pool, Locale::Es).await.unwrap();

        assert_eq!(session.locale, Locale::Es);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, Locale::Es).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
