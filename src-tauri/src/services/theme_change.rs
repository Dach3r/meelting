use crate::domain::session::Session;
use crate::domain::theme::Theme;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, theme: Theme) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_theme(pool, theme).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn starts_following_the_system_and_saves_the_new_theme() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        assert_eq!(first.theme, Theme::System);

        let session = run(&pool, Theme::Dark).await.unwrap();

        assert_eq!(session.theme, Theme::Dark);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, Theme::Light).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
