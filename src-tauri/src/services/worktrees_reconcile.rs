use crate::domain::repository::RepositoryIdentity;
use crate::domain::worktree::{
    RegisteredWorktree, Worktree, WorktreeErrorCode, WorktreeStatus, parse_worktree_list,
};
use crate::repositories::{git_repositories, task_worktrees};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::system::git::{self, GitError};
use sqlx::SqlitePool;
use std::path::Path;

fn same_path(registered: &str, stored: &str) -> bool {
    let normalized = |path: &str| path.replace('\\', "/");
    normalized(registered) == normalized(stored)
        || matches!(
            (std::fs::canonicalize(registered), std::fs::canonicalize(stored)),
            (Ok(left), Ok(right)) if left == right
        )
}

pub async fn registration(
    repository_path: &Path,
    path: &str,
) -> Result<Option<RegisteredWorktree>, GitError> {
    let list = git::worktree_list(repository_path).await?;
    Ok(parse_worktree_list(&list)
        .into_iter()
        .find(|entry| same_path(&entry.path, path)))
}

pub async fn intact(
    repository: &RepositoryIdentity,
    worktree: &Worktree,
) -> Option<RegisteredWorktree> {
    let path = Path::new(&worktree.path);
    std::fs::symlink_metadata(path).ok()?;
    let entry = registration(Path::new(&repository.path), &worktree.path)
        .await
        .ok()??;
    let layout = git::layout(path).await.ok()?;
    let same_folder =
        std::fs::canonicalize(&layout.toplevel).ok() == std::fs::canonicalize(path).ok();
    (same_folder && layout.common_dir == repository.git_common_dir).then_some(entry)
}

fn next(
    worktree: &Worktree,
    entry: Option<&RegisteredWorktree>,
    gone: bool,
) -> Option<(WorktreeStatus, Option<WorktreeErrorCode>)> {
    use WorktreeStatus::*;
    let expected = entry.is_some_and(|entry| {
        entry.branch.as_deref() == Some(worktree.branch_name.as_str())
            && entry.head.as_deref() == Some(worktree.base_commit_sha.as_str())
    });
    match worktree.status {
        Preparing if expected => Some((Ready, None)),
        Preparing => Some((Failed, Some(WorktreeErrorCode::PreparationInterrupted))),
        Removing if gone => Some((Removed, None)),
        Removing if entry.is_some() => Some((Ready, None)),
        Removing => Some((Failed, Some(WorktreeErrorCode::RemovalInterrupted))),
        Ready if entry.is_none() => Some((Missing, None)),
        Missing if entry.is_some() => Some((Ready, None)),
        _ => None,
    }
}

pub async fn gone(repository: &RepositoryIdentity, worktree: &Worktree) -> bool {
    if std::fs::symlink_metadata(&worktree.path).is_ok() {
        return false;
    }
    if !Path::new(&repository.git_common_dir).exists() {
        return true;
    }
    matches!(
        registration(Path::new(&repository.path), &worktree.path).await,
        Ok(None)
    )
}

pub async fn run(pool: &SqlitePool, locks: &RepositoryLocks) -> Result<(), SessionError> {
    for (repository_id, worktree) in task_worktrees::transient(pool).await? {
        let Some(repository) = git_repositories::identity(pool, &repository_id).await? else {
            continue;
        };
        let _lock = locks.acquire(&repository.git_common_dir).await;
        let entry = intact(&repository, &worktree).await;
        let gone = gone(&repository, &worktree).await;
        if let Some((status, error_code)) = next(&worktree, entry.as_ref(), gone) {
            task_worktrees::transition(pool, &worktree.id, worktree.status, status, error_code)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worktree(status: WorktreeStatus) -> Worktree {
        Worktree {
            id: "k1".into(),
            path: "/w/x".into(),
            branch_name: "meelting/x-1".into(),
            base_ref: "main".into(),
            base_commit_sha: "abc".into(),
            status,
            error_code: None,
            copies: Vec::new(),
            created_at: "x".into(),
            updated_at: "x".into(),
        }
    }

    fn entry(branch: &str, head: &str) -> RegisteredWorktree {
        RegisteredWorktree {
            path: "/w/x".into(),
            head: Some(head.into()),
            branch: Some(branch.into()),
        }
    }

    #[test]
    fn decides_each_transient_state_without_deleting_anything() {
        use WorktreeStatus::*;
        let good = entry("meelting/x-1", "abc");
        let moved_on = entry("meelting/x-1", "def");

        assert_eq!(
            next(&worktree(Preparing), Some(&good), false),
            Some((Ready, None))
        );
        assert_eq!(
            next(&worktree(Preparing), Some(&moved_on), false),
            Some((Failed, Some(WorktreeErrorCode::PreparationInterrupted)))
        );
        assert_eq!(
            next(&worktree(Preparing), None, true),
            Some((Failed, Some(WorktreeErrorCode::PreparationInterrupted)))
        );
        assert_eq!(next(&worktree(Removing), None, true), Some((Removed, None)));
        assert_eq!(
            next(&worktree(Removing), Some(&good), false),
            Some((Ready, None))
        );
        assert_eq!(
            next(&worktree(Removing), None, false),
            Some((Failed, Some(WorktreeErrorCode::RemovalInterrupted)))
        );
        assert_eq!(next(&worktree(Ready), Some(&moved_on), false), None);
        assert_eq!(next(&worktree(Ready), None, true), Some((Missing, None)));
        assert_eq!(
            next(&worktree(Missing), Some(&good), false),
            Some((Ready, None))
        );
        assert_eq!(next(&worktree(Missing), None, true), None);
    }

    #[test]
    fn compares_paths_across_separators() {
        assert!(same_path("C:/w/x", r"C:\w\x"));
        assert!(!same_path("/w/x", "/w/y"));
    }
}
