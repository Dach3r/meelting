use crate::domain::repository::RepositoryIdentity;
use crate::domain::storage::StorageIssue;
use crate::domain::worktree::{TRASH_DAYS, Worktree, WorktreeErrorCode, WorktreeStatus};
use crate::repositories::{git_repositories, task_worktrees};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::services::worktree_remove::verify_identity;
use crate::services::worktree_trash::{EnvironmentFailure, environment, record};
use crate::services::worktrees_reconcile::{gone, intact};
use crate::system::git;
use sqlx::SqlitePool;
use std::path::Path;

async fn purge(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    repository: &RepositoryIdentity,
    worktree: &Worktree,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    use WorktreeStatus::*;
    if worktree.trashed_at.is_none() {
        return Err(SessionError::EnvironmentNotInTrash);
    }
    let _lock = locks.acquire(&repository.git_common_dir).await;
    let repository_path = Path::new(&repository.path);
    match worktree.status {
        Ready => {
            verify_identity(repository, worktree, check_storage).await?;
            if !task_worktrees::transition(pool, &worktree.id, Ready, Removing, None).await? {
                return Err(SessionError::EnvironmentNotFound);
            }
            let _ =
                git::force_remove_worktree(repository_path, Path::new(worktree.location())).await;
            let (status, error_code) = if gone(repository, worktree).await {
                (Removed, None)
            } else if intact(repository, worktree).await.is_some() {
                (Ready, None)
            } else {
                (Failed, Some(WorktreeErrorCode::RemovalInterrupted))
            };
            task_worktrees::transition(pool, &worktree.id, Removing, status, error_code).await?;
            if status != Removed {
                return Err(SessionError::EnvironmentRemoveRefused);
            }
        }
        Failed | Missing => {
            if !gone(repository, worktree).await {
                return Err(SessionError::EnvironmentStillExists);
            }
            if !task_worktrees::transition(pool, &worktree.id, worktree.status, Removed, None)
                .await?
            {
                return Err(SessionError::EnvironmentNotFound);
            }
        }
        _ => return Err(SessionError::EnvironmentNotFound),
    }
    let still_there = git::branch_exists(repository_path, &worktree.branch_name)
        .await
        .unwrap_or(true);
    if still_there
        && git::delete_branch(repository_path, &worktree.branch_name)
            .await
            .is_err()
    {
        return Err(SessionError::EnvironmentBranchNotDeleted);
    }
    Ok(())
}

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_ids: &[String],
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Vec<EnvironmentFailure> {
    let mut failures = Vec::new();
    for task_id in task_ids {
        let result = match environment(pool, task_id).await {
            Ok((worktree, repository)) => {
                purge(pool, locks, &repository, &worktree, &check_storage).await
            }
            Err(error) => Err(error),
        };
        record(&mut failures, task_id, result);
    }
    failures
}

pub async fn expired(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    for (repository_id, worktree) in task_worktrees::expired_trash(pool, TRASH_DAYS).await? {
        if let Some(repository) = git_repositories::identity(pool, &repository_id).await? {
            let _ = purge(pool, locks, &repository, &worktree, &check_storage).await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::repository_register::tests::local;
    use crate::services::worktree_prepare::tests::{fixture, prepare, task};
    use crate::services::worktree_trash::tests::trashed;
    use crate::services::{task_delete, tasks_list};

    #[tokio::test]
    async fn purges_the_folder_its_local_files_and_its_branch_with_commits() {
        let fixture = fixture().await;
        let (task_id, worktree) = trashed(&fixture, "Gone").await;
        let location = tasks_list::run(&fixture.pool).await.unwrap()[0]
            .worktree
            .clone()
            .unwrap()
            .trash_path
            .unwrap();
        crate::services::repository_register::tests::git(
            Path::new(&location),
            &["commit", "--allow-empty", "-m", "work"],
        );
        std::fs::write(Path::new(&location).join(".env"), "SECRET=1").unwrap();

        let failures = run(
            &fixture.pool,
            &fixture.locks,
            std::slice::from_ref(&task_id),
            local,
        )
        .await;

        assert!(failures.is_empty());
        assert!(!Path::new(&location).exists());
        assert!(
            !git::branch_exists(&fixture.repo, &worktree.branch_name)
                .await
                .unwrap()
        );
        let task = tasks_list::run(&fixture.pool).await.unwrap();
        assert!(task[0].worktree.is_none());
        task_delete::run(&fixture.pool, &task_id).await.unwrap();
    }

    #[tokio::test]
    async fn only_purges_environments_in_the_trash() {
        let fixture = fixture().await;
        let task = task(&fixture, "Kept", "main").await;
        let path = prepare(&fixture, &task).await.worktree.unwrap().path;

        let failures = run(
            &fixture.pool,
            &fixture.locks,
            std::slice::from_ref(&task.id),
            local,
        )
        .await;

        assert!(matches!(
            failures[..],
            [EnvironmentFailure {
                error: SessionError::EnvironmentNotInTrash,
                ..
            }]
        ));
        assert!(Path::new(&path).exists());
    }

    #[tokio::test]
    async fn purges_automatically_only_after_thirty_days() {
        let fixture = fixture().await;
        let (recent, _) = trashed(&fixture, "Recent").await;
        let (old, _) = trashed(&fixture, "Old").await;
        sqlx::query(
            "UPDATE task_worktrees SET trashed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-31 days') \
             WHERE task_id = ?",
        )
        .bind(&old)
        .execute(&fixture.pool)
        .await
        .unwrap();

        expired(&fixture.pool, &fixture.locks, local).await.unwrap();

        let tasks = tasks_list::run(&fixture.pool).await.unwrap();
        let worktree_of = |id: &str| {
            tasks
                .iter()
                .find(|task| task.id == id)
                .unwrap()
                .worktree
                .clone()
        };
        assert!(worktree_of(&old).is_none());
        assert!(worktree_of(&recent).unwrap().trashed_at.is_some());
    }
}
