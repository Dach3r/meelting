use crate::domain::repository::Repository;
use crate::repositories::git_repositories;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Repository>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    Ok(git_repositories::list(pool, &workspace_id).await?)
}
