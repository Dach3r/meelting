use crate::domain::review::{DiffRequest, DiffSource, FileDiff, parse_patch, protected_path};
use crate::domain::storage::StorageIssue;
use crate::services::environment_review::{guarded, kept_review, review_error, unchanged_since};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::review_registry::{ReviewRegistry, file_token};
use crate::services::session_load::SessionError;
use crate::services::worktree_remove::{ready_environment, verify_identity};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    registry: &ReviewRegistry,
    task_id: &str,
    request: &DiffRequest,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<FileDiff, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, &check_storage).await?;
    let review = kept_review(registry, task_id, &request.review_id, &worktree)?;
    let entry = review
        .entries
        .get(request.entry_id as usize)
        .filter(|entry| entry.sources.contains(&request.source))
        .ok_or(SessionError::InvalidReviewEntry)?;
    if protected_path(&entry.path) || entry.original_path.as_deref().is_some_and(protected_path) {
        return Err(SessionError::ProtectedFile);
    }
    let guard = guarded(&repository, &worktree).await?;
    unchanged_since(&guard, &review.state).await?;
    if request.source == DiffSource::Unstaged
        && file_token(Path::new(&worktree.path), &entry.path) != entry.file
    {
        return Err(SessionError::ReviewStale);
    }
    let output = guard
        .diff_file(request.source, &entry.path, entry.original_path.as_deref())
        .await
        .map_err(review_error)?;
    if output.truncated {
        return Err(SessionError::DiffTooLarge);
    }
    unchanged_since(&guard, &review.state).await?;
    guard.finish().await.map_err(review_error)?;
    Ok(parse_patch(&output.stdout, false))
}
