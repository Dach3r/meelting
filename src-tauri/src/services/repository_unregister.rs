use crate::domain::repository::Repository;
use crate::repositories::git_repositories;
use crate::services::repositories_list;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, repository_id: &str) -> Result<Vec<Repository>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let deleted = git_repositories::delete(pool, &workspace_id, repository_id)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(database) if database.is_foreign_key_violation() => SessionError::RepositoryInUse,
            _ => SessionError::Database(error),
        })?;
    if !deleted {
        return Err(SessionError::RepositoryNotFound);
    }
    repositories_list::run(pool).await
}
