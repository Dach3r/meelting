use crate::domain::repository::RepositoryIdentity;
use crate::domain::storage::StorageIssue;
use crate::domain::worktree::{Worktree, WorktreeStatus, trash_folder};
use crate::repositories::{git_repositories, task_worktrees};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use crate::services::worktree_remove::verify_identity;
use crate::system::git;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct EnvironmentFailure {
    pub task_id: String,
    pub error: SessionError,
}

pub fn record(
    failures: &mut Vec<EnvironmentFailure>,
    task_id: &str,
    result: Result<(), SessionError>,
) {
    if let Err(error) = result {
        failures.push(EnvironmentFailure {
            task_id: task_id.to_string(),
            error,
        });
    }
}

pub async fn environment(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<(Worktree, RepositoryIdentity), SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let task = tasks_list::one(pool, &workspace_id, task_id).await?;
    let worktree = task.worktree.ok_or(SessionError::EnvironmentNotFound)?;
    let repository_id = task
        .repository_id
        .ok_or(SessionError::EnvironmentNotFound)?;
    let repository = git_repositories::identity(pool, &repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    Ok((worktree, repository))
}

fn trash_destination(worktree: &Worktree) -> Result<PathBuf, SessionError> {
    let destination =
        trash_folder(Path::new(&worktree.path)).ok_or(SessionError::WorktreePathConflict)?;
    let folder = destination
        .parent()
        .ok_or(SessionError::WorktreePathConflict)?;
    match std::fs::symlink_metadata(folder) {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(folder).map_err(|_| SessionError::WorktreePathConflict)?;
        }
        _ => return Err(SessionError::WorktreePathConflict),
    }
    if std::fs::symlink_metadata(&destination).is_ok() {
        return Err(SessionError::WorktreePathConflict);
    }
    Ok(destination)
}

async fn trash_one(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    let (worktree, repository) = environment(pool, task_id).await?;
    if worktree.status != WorktreeStatus::Ready {
        return Err(SessionError::EnvironmentNotFound);
    }
    if worktree.trashed_at.is_some() {
        return Err(SessionError::EnvironmentInTrash);
    }
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, check_storage).await?;
    let destination = trash_destination(&worktree)?;
    git::move_worktree(
        Path::new(&repository.path),
        Path::new(&worktree.path),
        &destination,
    )
    .await
    .map_err(|_| SessionError::EnvironmentMoveRefused)?;
    task_worktrees::set_trash(pool, &worktree.id, Some(&destination.to_string_lossy())).await?;
    Ok(())
}

async fn restore_one(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    let (worktree, repository) = environment(pool, task_id).await?;
    if worktree.trashed_at.is_none() {
        return Err(SessionError::EnvironmentNotInTrash);
    }
    if worktree.status != WorktreeStatus::Ready {
        return Err(SessionError::EnvironmentNotFound);
    }
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, check_storage).await?;
    if std::fs::symlink_metadata(&worktree.path).is_ok() {
        return Err(SessionError::WorktreePathConflict);
    }
    git::move_worktree(
        Path::new(&repository.path),
        Path::new(worktree.location()),
        Path::new(&worktree.path),
    )
    .await
    .map_err(|_| SessionError::EnvironmentMoveRefused)?;
    task_worktrees::set_trash(pool, &worktree.id, None).await?;
    Ok(())
}

pub async fn trash(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_ids: &[String],
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Vec<EnvironmentFailure> {
    let mut failures = Vec::new();
    for task_id in task_ids {
        let result = trash_one(pool, locks, task_id, &check_storage).await;
        record(&mut failures, task_id, result);
    }
    failures
}

pub async fn restore(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_ids: &[String],
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Vec<EnvironmentFailure> {
    let mut failures = Vec::new();
    for task_id in task_ids {
        let result = restore_one(pool, locks, task_id, &check_storage).await;
        record(&mut failures, task_id, result);
    }
    failures
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::services::repository_register::tests::local;
    use crate::services::worktree_prepare::tests::{Fixture, fixture, prepare, task};
    use crate::services::{worktree_remove, worktrees_reconcile};

    pub async fn trashed(fixture: &Fixture, title: &str) -> (String, Worktree) {
        let task = task(fixture, title, "main").await;
        let worktree = prepare(fixture, &task).await.worktree.unwrap();
        let ids = [task.id.clone()];
        assert!(
            trash(&fixture.pool, &fixture.locks, &ids, local)
                .await
                .is_empty()
        );
        (task.id, worktree)
    }

    pub async fn current(fixture: &Fixture, task_id: &str) -> Worktree {
        environment(&fixture.pool, task_id).await.unwrap().0
    }

    #[tokio::test]
    async fn moves_an_environment_with_its_local_files_into_the_trash_and_back() {
        let fixture = fixture().await;
        let task = task(&fixture, "Dirty", "main").await;
        let original = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
        std::fs::write(original.join(".env"), "SECRET=1").unwrap();
        std::fs::write(original.join("README.md"), "changed").unwrap();
        let ids = [task.id.clone()];

        assert!(
            trash(&fixture.pool, &fixture.locks, &ids, local)
                .await
                .is_empty()
        );
        let in_trash = current(&fixture, &task.id).await;
        let location = PathBuf::from(in_trash.location());
        assert!(in_trash.trashed_at.is_some());
        assert!(!original.exists());
        assert_eq!(
            location,
            original
                .parent()
                .unwrap()
                .join(".trash")
                .join(original.file_name().unwrap())
        );
        assert_eq!(
            std::fs::read_to_string(location.join(".env")).unwrap(),
            "SECRET=1"
        );

        assert!(
            restore(&fixture.pool, &fixture.locks, &ids, local)
                .await
                .is_empty()
        );
        let restored = current(&fixture, &task.id).await;
        assert_eq!(restored.trashed_at, None);
        assert_eq!(restored.location(), original.to_string_lossy());
        assert_eq!(
            std::fs::read_to_string(original.join("README.md")).unwrap(),
            "changed"
        );
        assert!(!location.exists());
    }

    #[tokio::test]
    async fn a_trashed_environment_cannot_be_used_removed_or_trashed_again() {
        let fixture = fixture().await;
        let (task_id, _) = trashed(&fixture, "Trashed").await;
        let ids = [task_id.clone()];

        let failures = trash(&fixture.pool, &fixture.locks, &ids, local).await;
        assert!(matches!(
            failures[..],
            [EnvironmentFailure {
                error: SessionError::EnvironmentInTrash,
                ..
            }]
        ));
        assert!(matches!(
            worktree_remove::check(&fixture.pool, &fixture.locks, &task_id, local).await,
            Err(SessionError::EnvironmentInTrash)
        ));
        assert!(matches!(
            worktree_remove::ready_environment(&fixture.pool, &task_id).await,
            Err(SessionError::EnvironmentInTrash)
        ));
    }

    #[tokio::test]
    async fn reports_each_failure_and_keeps_going() {
        let fixture = fixture().await;
        let task = task(&fixture, "Good", "main").await;
        prepare(&fixture, &task).await;
        let ids = ["missing".to_string(), task.id.clone()];

        let failures = trash(&fixture.pool, &fixture.locks, &ids, local).await;

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].task_id, "missing");
        assert!(current(&fixture, &task.id).await.trashed_at.is_some());
        let restored = restore(&fixture.pool, &fixture.locks, &ids[1..], local).await;
        assert!(restored.is_empty());
        let again = restore(&fixture.pool, &fixture.locks, &ids[1..], local).await;
        assert!(matches!(
            again[..],
            [EnvironmentFailure {
                error: SessionError::EnvironmentNotInTrash,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn recovery_follows_a_move_that_was_not_recorded() {
        let fixture = fixture().await;
        let task = task(&fixture, "Crash", "main").await;
        let worktree = prepare(&fixture, &task).await.worktree.unwrap();
        let destination = trash_destination(&worktree).unwrap();
        git::move_worktree(&fixture.repo, Path::new(&worktree.path), &destination)
            .await
            .unwrap();

        worktrees_reconcile::run(&fixture.pool, &fixture.locks)
            .await
            .unwrap();

        let recovered = current(&fixture, &task.id).await;
        assert_eq!(recovered.status, WorktreeStatus::Ready);
        assert_eq!(Path::new(recovered.location()), destination);

        git::move_worktree(&fixture.repo, &destination, Path::new(&worktree.path))
            .await
            .unwrap();
        worktrees_reconcile::run(&fixture.pool, &fixture.locks)
            .await
            .unwrap();

        let back = current(&fixture, &task.id).await;
        assert_eq!(
            (back.status, back.trashed_at),
            (WorktreeStatus::Ready, None)
        );
    }
}
