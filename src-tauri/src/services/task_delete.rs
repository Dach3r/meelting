use crate::domain::task::Task;
use crate::repositories::{task_worktrees, tasks};
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, task_id: &str) -> Result<Vec<Task>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if task_worktrees::active_for_task(&mut *transaction, &workspace_id, task_id)
        .await?
        .is_some()
    {
        return Err(SessionError::TaskHasEnvironment);
    }
    task_worktrees::delete_removed(&mut *transaction, task_id).await?;
    if !tasks::delete(&mut *transaction, &workspace_id, task_id).await? {
        return Err(SessionError::TaskNotFound);
    }
    transaction.commit().await?;
    tasks_list::run(pool).await
}
