use crate::domain::storage::StorageIssue;
use crate::domain::worktree::{WorktreesRoot, default_root};
use crate::repositories::profile;
use crate::services::session_load::SessionError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

pub async fn resolve(pool: &SqlitePool) -> Result<(PathBuf, bool), SessionError> {
    match profile::worktrees_root(pool).await? {
        Some(root) => Ok((PathBuf::from(root), true)),
        None => {
            let home = std::env::home_dir().ok_or(SessionError::StorageUnknown)?;
            Ok((default_root(&home), false))
        }
    }
}

pub async fn run(
    pool: &SqlitePool,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<WorktreesRoot, SessionError> {
    let (path, custom) = resolve(pool).await?;
    let issue = check_storage(&path).await.err();
    Ok(WorktreesRoot {
        path: path.to_string_lossy().into_owned(),
        custom,
        issue,
    })
}
