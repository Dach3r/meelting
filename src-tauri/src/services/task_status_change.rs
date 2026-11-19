use crate::domain::task::{Task, TaskStatus};
use crate::repositories::tasks;
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    task_id: &str,
    status: TaskStatus,
) -> Result<Task, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let current = tasks_list::one(pool, &workspace_id, task_id).await?;
    if !current.status.can_change_to(status)
        || !tasks::change_status(pool, &workspace_id, task_id, current.status, status).await?
    {
        return Err(SessionError::InvalidTaskTransition);
    }
    tasks_list::one(pool, &workspace_id, task_id).await
}
