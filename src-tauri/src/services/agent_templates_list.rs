use crate::catalogs::agent_templates::{self, AgentTemplate};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<AgentTemplate>, SessionError> {
    let locale = session_load::require(pool).await?.locale;
    Ok(agent_templates::all(locale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, locale_change};

    #[tokio::test]
    async fn requires_the_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert!(matches!(run(&pool).await, Err(SessionError::NotSetUp)));
    }

    #[tokio::test]
    async fn lists_the_templates_in_the_profile_language() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "First").await.unwrap();
        let english = run(&pool).await.unwrap();
        locale_change::run(&pool, Locale::Es).await.unwrap();
        let spanish = run(&pool).await.unwrap();

        assert_eq!(english.len(), 25);
        assert_eq!(english[0].key, spanish[0].key);
        assert_ne!(english[0].description, spanish[0].description);
    }
}
