use crate::domain::task::Task;
use crate::repositories::tasks;
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, task_id: &str) -> Result<Vec<Task>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if !tasks::delete(pool, &workspace_id, task_id).await? {
        return Err(SessionError::TaskNotFound);
    }
    tasks_list::run(pool).await
}
