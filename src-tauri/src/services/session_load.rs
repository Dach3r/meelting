use crate::domain::session::Session;
use crate::repositories::{profile, workspaces};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("database")]
    Database(#[from] sqlx::Error),
    #[error("invalid_name")]
    InvalidName,
    #[error("workspace_not_found")]
    WorkspaceNotFound,
    #[error("team_not_found")]
    TeamNotFound,
    #[error("invalid_agent")]
    InvalidAgent,
    #[error("not_set_up")]
    NotSetUp,
}

impl Serialize for SessionError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub async fn run(pool: &SqlitePool) -> Result<Option<Session>, SessionError> {
    let Some(profile) = profile::get(pool).await? else {
        return Ok(None);
    };
    let Some(current_workspace_id) = profile.current_workspace_id else {
        return Ok(None);
    };
    Ok(Some(Session {
        locale: profile.locale,
        theme: profile.theme,
        current_workspace_id,
        workspaces: workspaces::list(pool).await?,
    }))
}

pub async fn require(pool: &SqlitePool) -> Result<Session, SessionError> {
    run(pool).await?.ok_or(SessionError::NotSetUp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn is_none_before_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert_eq!(run(&pool).await.unwrap(), None);
        assert!(matches!(require(&pool).await, Err(SessionError::NotSetUp)));
    }

    #[tokio::test]
    async fn loads_profile_and_workspaces() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();
        profile::create(&pool, Locale::Es, "01").await.unwrap();

        let session = run(&pool).await.unwrap().unwrap();

        assert_eq!(session.locale, Locale::Es);
        assert_eq!(session.current_workspace_id, "01");
        assert_eq!(session.workspaces.len(), 1);
    }

    #[tokio::test]
    async fn error_serializes_as_its_kind() {
        assert_eq!(
            serde_json::to_string(&SessionError::InvalidName).unwrap(),
            "\"invalid_name\""
        );
    }
}
