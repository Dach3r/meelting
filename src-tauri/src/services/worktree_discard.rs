use crate::domain::task::Task;
use crate::domain::worktree::WorktreeStatus;
use crate::repositories::{git_repositories, task_worktrees};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use crate::services::worktrees_reconcile::gone;
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
) -> Result<Task, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let task = tasks_list::one(pool, &workspace_id, task_id).await?;
    let worktree = task
        .worktree
        .filter(|worktree| {
            matches!(
                worktree.status,
                WorktreeStatus::Failed | WorktreeStatus::Missing
            )
        })
        .ok_or(SessionError::EnvironmentNotFound)?;
    let repository_id = task
        .repository_id
        .ok_or(SessionError::EnvironmentNotFound)?;
    let repository = git_repositories::identity(pool, &repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    if !gone(&repository, &worktree).await {
        return Err(SessionError::EnvironmentStillExists);
    }
    if !task_worktrees::transition(
        pool,
        &worktree.id,
        worktree.status,
        WorktreeStatus::Removed,
        None,
    )
    .await?
    {
        return Err(SessionError::EnvironmentNotFound);
    }
    tasks_list::one(pool, &workspace_id, task_id).await
}
