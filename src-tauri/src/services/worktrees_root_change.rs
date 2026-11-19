use crate::domain::storage::{StorageIssue, is_inside};
use crate::domain::worktree::{WorktreesRoot, simplified};
use crate::repositories::{git_repositories, profile};
use crate::services::session_load::{self, SessionError};
use crate::services::worktrees_root_get;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

pub async fn run(
    pool: &SqlitePool,
    folder: &Path,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<WorktreesRoot, SessionError> {
    session_load::require(pool).await?;
    let root = std::fs::canonicalize(folder)
        .map(simplified)
        .map_err(|_| SessionError::InvalidWorktreesRoot)?;
    if !root.is_dir() {
        return Err(SessionError::InvalidWorktreesRoot);
    }
    let case_sensitive = cfg!(target_os = "linux");
    for (path, common_dir) in git_repositories::all_locations(pool).await? {
        for location in [path, common_dir] {
            let location = std::fs::canonicalize(&location)
                .map(simplified)
                .unwrap_or_else(|_| PathBuf::from(location));
            if is_inside(&root, std::slice::from_ref(&location), case_sensitive)
                || is_inside(&location, std::slice::from_ref(&root), case_sensitive)
            {
                return Err(SessionError::InvalidWorktreesRoot);
            }
        }
    }
    check_storage(&root).await?;
    profile::set_worktrees_root(pool, &root.to_string_lossy()).await?;
    worktrees_root_get::run(pool, check_storage).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;
    use crate::services::repository_register::{self, tests::repo_with_commit};

    async fn local(_: &Path) -> Result<(), StorageIssue> {
        Ok(())
    }

    async fn network(_: &Path) -> Result<(), StorageIssue> {
        Err(StorageIssue::Network)
    }

    #[tokio::test]
    async fn uses_the_home_folder_until_a_folder_is_chosen() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let chosen = tempfile::tempdir().unwrap();

        let before = worktrees_root_get::run(&pool, local).await.unwrap();
        let after = run(&pool, chosen.path(), local).await.unwrap();

        assert!(!before.custom);
        assert!(
            before
                .path
                .ends_with(&format!("Meelting{}worktrees", std::path::MAIN_SEPARATOR))
        );
        assert!(after.custom);
        assert_eq!(
            PathBuf::from(&after.path),
            simplified(std::fs::canonicalize(chosen.path()).unwrap())
        );
        assert_eq!(after.issue, None);
    }

    #[tokio::test]
    async fn rejects_unsupported_storage_and_missing_folders_without_saving() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let chosen = tempfile::tempdir().unwrap();

        assert!(matches!(
            run(&pool, chosen.path(), network).await,
            Err(SessionError::StorageNetwork)
        ));
        assert!(matches!(
            run(&pool, &chosen.path().join("missing"), local).await,
            Err(SessionError::InvalidWorktreesRoot)
        ));
        assert_eq!(profile::worktrees_root(&pool).await.unwrap(), None);
    }

    #[tokio::test]
    async fn rejects_a_folder_inside_or_around_a_registered_repository() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        repository_register::run(&pool, &repo, local).await.unwrap();
        std::fs::create_dir(repo.join("worktrees")).unwrap();

        for folder in [
            repo.join("worktrees"),
            repo.join(".git"),
            parent.path().to_path_buf(),
        ] {
            assert!(
                matches!(
                    run(&pool, &folder, local).await,
                    Err(SessionError::InvalidWorktreesRoot)
                ),
                "{folder:?} was accepted"
            );
        }
    }

    #[tokio::test]
    async fn reports_the_storage_issue_of_the_current_folder() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        let root = worktrees_root_get::run(&pool, network).await.unwrap();

        assert_eq!(root.issue, Some(StorageIssue::Network));
    }
}
