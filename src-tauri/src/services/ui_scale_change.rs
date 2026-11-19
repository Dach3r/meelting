use crate::domain::session::Session;
use crate::domain::ui_scale::UiScale;
use crate::repositories::profile;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, ui_scale: UiScale) -> Result<Session, SessionError> {
    session_load::require(pool).await?;
    profile::set_ui_scale(pool, ui_scale).await?;
    session_load::require(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;

    #[tokio::test]
    async fn starts_at_one_hundred_and_saves_the_new_scale() {
        let (pool, _dir) = open_for_test().await;
        let first = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        assert_eq!(first.ui_scale, UiScale::default());

        let scale = UiScale::from_percent(125).unwrap();
        let session = run(&pool, scale).await.unwrap();

        assert_eq!(session.ui_scale, scale);
        assert_eq!(session_load::run(&pool).await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        let result = run(&pool, UiScale::default()).await;

        assert!(matches!(result, Err(SessionError::NotSetUp)));
    }
}
